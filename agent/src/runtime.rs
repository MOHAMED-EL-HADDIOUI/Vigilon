use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;
use vigilon_core::blocklist::BlocklistManager;
use vigilon_core::config::AgentConfig;
use vigilon_core::events::{BusEvent, RiskLevel, TimelineEntry};
use vigilon_core::metrics::{
    BandwidthQuotaInfo, Connection, DnsQuery, ListeningPort, ProcessBandwidth, ProcessInfo,
    ServiceInfo, Snapshot,
};
use vigilon_core::{EventBus, Storage};
use vigilon_platform::{GeoIpResolver, Platform};

use crate::engine::DetectionEngine;
use crate::notifier::Notifier;

#[derive(Clone, Default)]
pub struct LiveState {
    pub snapshot: Option<Snapshot>,
    pub processes: Vec<ProcessInfo>,
    pub ports: Vec<ListeningPort>,
    pub connections: Vec<Connection>,
    pub interfaces: Vec<vigilon_core::metrics::NetworkInterface>,
    pub services: Vec<ServiceInfo>,
    pub dns: Vec<vigilon_core::metrics::DnsCacheEntry>,
    pub process_bandwidth: Vec<ProcessBandwidth>,
    pub dns_queries: Vec<DnsQuery>,
    pub bandwidth_quota: Option<BandwidthQuotaInfo>,
}

#[derive(Clone)]
pub struct AgentRuntime {
    pub bus: EventBus,
    pub storage: Storage,
    pub live: Arc<RwLock<LiveState>>,
    pub config: AgentConfig,
    pub blocklist: BlocklistManager,
    pub geoip: GeoIpResolver,
    pub notifier: Arc<Notifier>,
}

impl AgentRuntime {
    pub fn new(bus: EventBus, storage: Storage) -> Self {
        Self::with_config(bus, storage, AgentConfig::from_env())
    }

    pub fn with_config(bus: EventBus, storage: Storage, config: AgentConfig) -> Self {
        let dummy_path = std::path::Path::new(".");
        Self::new_full(
            bus,
            storage,
            config.clone(),
            BlocklistManager::new(dummy_path),
            GeoIpResolver::new(dummy_path),
            Arc::new(Notifier::new(
                config.notifications_enabled,
                config.notification_min_risk.clone(),
            )),
        )
    }

    pub fn new_full(
        bus: EventBus,
        storage: Storage,
        config: AgentConfig,
        blocklist: BlocklistManager,
        geoip: GeoIpResolver,
        notifier: Arc<Notifier>,
    ) -> Self {
        Self {
            bus,
            storage,
            live: Arc::new(RwLock::new(LiveState::default())),
            config,
            blocklist,
            geoip,
            notifier,
        }
    }

    pub fn spawn(self) {
        let mut sub = self.bus.subscribe();
        let notifier = self.notifier.clone();
        tokio::spawn(async move {
            let mut lagged_total: u64 = 0;
            loop {
                match sub.recv().await {
                    Ok(ev) => {
                        if let BusEvent::Security(sec) = ev {
                            notifier.notify(&sec);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        lagged_total += n;
                        tracing::warn!(
                            skipped = n,
                            total = lagged_total,
                            "notifier lagged behind event bus; skipping missed events"
                        );
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                tracing::error!("agent runtime stopped: {e:#}");
            }
        });
    }

    async fn run(self) -> anyhow::Result<()> {
        let mut platform = Platform::new();
        let mut engine = DetectionEngine::new();
        engine.load_baselines(&self.storage).await;

        let boot = TimelineEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            kind: "SYSTEM".into(),
            risk: RiskLevel::Info,
            title: "System monitor started".into(),
            summary: "Vigilon collectors are running locally. No data leaves this machine.".into(),
            event_id: None,
        };
        self.storage.insert_timeline(&boot).await?;
        self.bus.publish(BusEvent::Timeline(boot));

        let mut last_metrics = Instant::now()
            .checked_sub(Duration::from_secs(60))
            .unwrap_or_else(Instant::now);
        let mut last_sockets = last_metrics;
        let mut last_procs = last_metrics;
        let mut last_services = last_metrics;
        let mut last_iface_store = last_metrics;
        let mut last_persist = Instant::now();
        let mut last_prune = Instant::now();
        let mut last_device = Instant::now()
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or_else(Instant::now);
        let mut last_total_rx: u64 = 0;
        let mut last_total_tx: u64 = 0;
        let mut current_rx_bps = 0.0;
        let mut current_tx_bps = 0.0;
        // DNS cache snapshots re-list the same names every tick: only
        // persist and evaluate names not seen in the previous batch.
        let mut last_dns_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        loop {
            let now = Instant::now();
            // Collectors are synchronous OS calls (sysinfo, COM, netlink
            // shims). `block_in_place` keeps them off the async worker's
            // critical path on the multi-thread runtime the daemon uses.
            // NOTE: this panics on a current-thread runtime; the binary
            // always runs multi-threaded (see `main.rs`).
            let device = tokio::task::block_in_place(|| platform.device());
            // Device identity changes rarely: persist at most every 10 min
            // instead of on every 250ms loop pass.
            if now.duration_since(last_device) >= Duration::from_secs(600) {
                last_device = now;
                if let Err(e) = self.storage.upsert_device(&device).await {
                    tracing::warn!(error = %e, "upsert_device failed");
                }
            }

            if now.duration_since(last_metrics) >= self.config.metrics_interval() {
                last_metrics = now;
                let (cpu, memory) = tokio::task::block_in_place(|| platform.cpu_memory());
                let mut gpus = tokio::task::block_in_place(|| platform.gpus());
                let disks = tokio::task::block_in_place(|| platform.disks());
                let battery = tokio::task::block_in_place(|| platform.battery());
                let ifaces = tokio::task::block_in_place(|| platform.interfaces());
                let fw = tokio::task::block_in_place(|| platform.firewall());
                let rx: f64 = ifaces.iter().map(|i| i.rx_bps).sum();
                let tx: f64 = ifaces.iter().map(|i| i.tx_bps).sum();
                current_rx_bps = rx;
                current_tx_bps = tx;

                let total_rx_bytes: u64 = ifaces.iter().map(|i| i.rx_total_bytes).sum();
                let total_tx_bytes: u64 = ifaces.iter().map(|i| i.tx_total_bytes).sum();
                if last_total_rx > 0 && total_rx_bytes >= last_total_rx {
                    let delta_rx = total_rx_bytes - last_total_rx;
                    let delta_tx = total_tx_bytes.saturating_sub(last_total_tx);
                    let month = Utc::now().format("%Y-%m").to_string();
                    if let Err(e) = self
                        .storage
                        .update_bandwidth(&month, delta_rx, delta_tx)
                        .await
                    {
                        tracing::warn!(error = %e, "update_bandwidth failed");
                    }
                    if let Ok((u_rx, u_tx)) = self.storage.get_monthly_bandwidth(&month).await {
                        engine
                            .observe_bandwidth_quota(
                                &self.bus,
                                &self.storage,
                                self.config.bandwidth_quota_gb,
                                self.config.bandwidth_warning_pct,
                                u_rx + u_tx,
                            )
                            .await;
                        let mut live = self.live.write().await;
                        live.bandwidth_quota = Some(BandwidthQuotaInfo {
                            quota_gb: self.config.bandwidth_quota_gb,
                            used_rx_bytes: u_rx,
                            used_tx_bytes: u_tx,
                            month,
                            warning_pct: self.config.bandwidth_warning_pct,
                        });
                    }
                }
                last_total_rx = total_rx_bytes;
                last_total_tx = total_tx_bytes;

                {
                    let live = self.live.read().await;
                    for g in gpus.iter_mut() {
                        for gp in g.processes.iter_mut() {
                            if gp.name.is_none() {
                                gp.name = live
                                    .processes
                                    .iter()
                                    .find(|p| p.pid == gp.pid)
                                    .map(|p| p.name.clone());
                            }
                        }
                    }
                }
                let snapshot = Snapshot {
                    timestamp: Utc::now(),
                    device,
                    cpu,
                    memory,
                    gpus,
                    disks,
                    battery,
                    network_rx_bps: rx,
                    network_tx_bps: tx,
                    firewall: fw.clone(),
                };
                if let Err(e) = self.storage.insert_snapshot(&snapshot).await {
                    tracing::warn!(error = %e, "insert_snapshot failed");
                }
                if now.duration_since(last_iface_store) >= Duration::from_secs(30) {
                    last_iface_store = now;
                    if let Err(e) = self
                        .storage
                        .insert_interfaces(snapshot.timestamp, &ifaces)
                        .await
                    {
                        tracing::warn!(error = %e, "insert_interfaces failed");
                    }
                }
                engine
                    .observe_net_config(&self.bus, &self.storage, &ifaces)
                    .await;
                if let Some(fw) = &fw {
                    engine
                        .observe_firewall(&self.bus, &self.storage, &fw.details, fw.enabled)
                        .await;
                }
                {
                    let mut live = self.live.write().await;
                    live.snapshot = Some(snapshot.clone());
                    live.interfaces = ifaces.clone();
                }
                self.bus.publish(BusEvent::Metrics(snapshot));
                self.bus.publish(BusEvent::Interfaces {
                    timestamp: Utc::now(),
                    items: ifaces,
                });
            }

            if now.duration_since(last_sockets) >= self.config.sockets_interval() {
                last_sockets = now;
                let (ports, mut conns) = tokio::task::block_in_place(|| platform.sockets());
                let mut processes = tokio::task::block_in_place(|| platform.process_list());
                platform.attach_connection_counts(&mut processes, &conns);
                engine.stamp_remote_seen(&mut conns);

                for c in &mut conns {
                    c.geo = self.geoip.resolve(&c.remote_addr);
                }

                let proc_bw = platform.process_bandwidth(&conns, current_rx_bps, current_tx_bps);

                engine.observe_ports(&self.bus, &self.storage, &ports).await;
                engine
                    .observe_connections(
                        &self.bus,
                        &self.storage,
                        &conns,
                        &processes,
                        &self.blocklist,
                    )
                    .await;
                if let Err(e) = self.storage.insert_connections(Utc::now(), &conns).await {
                    tracing::warn!(error = %e, "insert_connections failed");
                }
                {
                    let mut live = self.live.write().await;
                    live.ports = ports.clone();
                    live.connections = conns.clone();
                    live.processes = processes.clone();
                    live.process_bandwidth = proc_bw.clone();
                }
                self.bus.publish(BusEvent::Ports {
                    timestamp: Utc::now(),
                    items: ports,
                });
                self.bus.publish(BusEvent::Connections {
                    timestamp: Utc::now(),
                    items: conns,
                });
                self.bus.publish(BusEvent::Processes {
                    timestamp: Utc::now(),
                    items: processes,
                });
                self.bus.publish(BusEvent::ProcessBandwidth {
                    timestamp: Utc::now(),
                    items: proc_bw,
                });
            }

            if now.duration_since(last_procs) >= self.config.processes_interval() {
                last_procs = now;
                let processes = {
                    let live = self.live.read().await;
                    live.processes.clone()
                };
                engine
                    .observe_processes(&self.bus, &self.storage, &processes)
                    .await;
                if let Err(e) = self.storage.insert_processes(Utc::now(), &processes).await {
                    tracing::warn!(error = %e, "insert_processes failed");
                }
            }

            if now.duration_since(last_services) >= self.config.services_interval() {
                last_services = now;
                let services = tokio::task::block_in_place(|| platform.services());
                engine
                    .observe_services(&self.bus, &self.storage, &services)
                    .await;
                {
                    let mut live = self.live.write().await;
                    live.services = services.clone();
                }
                self.bus.publish(BusEvent::Services {
                    timestamp: Utc::now(),
                    items: services,
                });
                let auth_failed = tokio::task::block_in_place(|| platform.auth_failed_recent());
                engine
                    .observe_auth_failures(&self.bus, &self.storage, auth_failed)
                    .await;
                let dns = tokio::task::block_in_place(|| platform.dns_cache());
                let queries = tokio::task::block_in_place(|| platform.dns_queries());
                let fresh: Vec<DnsQuery> = vigilon_platform::dns_logger::cap_batch(queries, 500)
                    .into_iter()
                    .filter(|q| !last_dns_names.contains(&q.query_name))
                    .collect();
                last_dns_names = fresh.iter().map(|q| q.query_name.clone()).collect();
                if last_dns_names.len() > 2000 {
                    // Bound the set; worst case we re-evaluate once.
                    last_dns_names.clear();
                }
                let queries = fresh;
                if !queries.is_empty() {
                    if let Err(e) = self.storage.insert_dns_queries(&queries).await {
                        tracing::warn!(error = %e, "insert_dns_queries failed");
                    }
                    engine
                        .observe_dns_queries(&self.bus, &self.storage, &queries, &self.blocklist)
                        .await;
                }
                {
                    let mut live = self.live.write().await;
                    live.dns = dns.clone();
                    live.dns_queries = queries.clone();
                }
                self.bus.publish(BusEvent::DnsCache {
                    timestamp: Utc::now(),
                    items: dns,
                });
                if !queries.is_empty() {
                    self.bus.publish(BusEvent::DnsQueries {
                        timestamp: Utc::now(),
                        items: queries,
                    });
                }
            }

            if now.duration_since(last_persist) >= Duration::from_secs(60) {
                last_persist = now;
                engine.persist_baselines(&self.storage).await;
            }

            if now.duration_since(last_prune) >= Duration::from_secs(600) {
                last_prune = now;
                if let Err(e) = self.storage.prune(self.config.retention_days).await {
                    tracing::warn!(error = %e, "prune failed");
                }
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}
