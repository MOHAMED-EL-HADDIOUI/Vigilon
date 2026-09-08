use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use chrono::Utc;
use uuid::Uuid;
use vigilon_core::detection::score;
use vigilon_core::events::{BusEvent, ProcessRef, RuleId, SecurityEvent, TimelineEntry};
use vigilon_core::metrics::{
    Connection, ListeningPort, NetworkInterface, ProcessInfo, ServiceInfo,
};
use vigilon_core::{EventBus, Storage};

/// Baselines decay: entries unseen for this long are forgotten, so a
/// process seen once months ago stops suppressing "new" findings.
const BASELINE_TTL_DAYS: i64 = 30;
/// Hard caps keep the baseline tables bounded.
const MAX_DESTS: usize = 5000;
const MAX_PROCS: usize = 2000;
const MAX_PORTS: usize = 2000;
const MAX_SERVICES: usize = 1000;

#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct PortKey {
    proto: String,
    port: u16,
    addr: String,
}

impl PortKey {
    fn encode(&self) -> String {
        format!("{}|{}|{}", self.proto, self.port, self.addr)
    }

    fn decode(s: &str) -> Option<Self> {
        let mut parts = s.split('|');
        Some(Self {
            proto: parts.next()?.to_string(),
            port: parts.next()?.parse().ok()?,
            addr: parts.next()?.to_string(),
        })
    }
}

pub struct DetectionEngine {
    known_ports: HashSet<PortKey>,
    primed_ports: bool,
    seen_process_dest: HashMap<String, chrono::DateTime<Utc>>,
    known_processes: HashMap<String, chrono::DateTime<Utc>>,
    known_services: HashSet<String>,
    primed_services: bool,
    last_fw: Option<String>,
    last_dns: Option<Vec<String>>,
    last_gw: Option<Option<String>>,
    conn_window: VecDeque<(Instant, String)>,
    inbound_hits: HashMap<String, VecDeque<Instant>>,
    /// SYN-probing tracker: remote -> (seen at, local port hit). Catches
    /// scanners regardless of direction classification — a WAN host SYN-ing
    /// many local ports is a scan even though its address is "external".
    scan_targets: HashMap<String, VecDeque<(Instant, u16)>>,
    remote_first_seen: HashMap<String, chrono::DateTime<Utc>>,
    /// Per-key emission cooldowns: (rule|key) -> last emitted at.
    /// Stops re-alert storms (blocklist every 4s tick, quota every 2s...).
    cooldowns: HashMap<String, Instant>,
    /// Suppression keys loaded from the feedback table (`RULE|key`).
    suppressed: HashSet<String>,
    last_feedback_sync: Option<Instant>,
}

impl Default for DetectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DetectionEngine {
    pub fn new() -> Self {
        Self {
            known_ports: HashSet::new(),
            primed_ports: false,
            seen_process_dest: HashMap::new(),
            known_processes: HashMap::new(),
            known_services: HashSet::new(),
            primed_services: false,
            last_fw: None,
            last_dns: None,
            last_gw: None,
            conn_window: VecDeque::new(),
            inbound_hits: HashMap::new(),
            scan_targets: HashMap::new(),
            remote_first_seen: HashMap::new(),
            cooldowns: HashMap::new(),
            suppressed: HashSet::new(),
            last_feedback_sync: None,
        }
    }

    /// Decode a baseline value that may be the legacy `Vec<String>` shape
    /// or the current `{key: rfc3339}` map shape with TTL decay applied.
    fn decode_baseline(raw: &str, cap: usize) -> HashMap<String, chrono::DateTime<Utc>> {
        let now = Utc::now();
        let ttl = chrono::Duration::days(BASELINE_TTL_DAYS);
        if let Ok(map) = serde_json::from_str::<HashMap<String, chrono::DateTime<Utc>>>(raw) {
            return map
                .into_iter()
                .filter(|(_, ts)| now.signed_duration_since(*ts) < ttl)
                .take(cap)
                .collect();
        }
        // Legacy shape: plain list, stamped now so it decays naturally.
        if let Ok(list) = serde_json::from_str::<Vec<String>>(raw) {
            return list.into_iter().take(cap).map(|k| (k, now)).collect();
        }
        HashMap::new()
    }

    pub async fn load_baselines(&mut self, storage: &Storage) {
        if let Ok(Some(v)) = storage.baseline_get("seen_process_dest").await {
            self.seen_process_dest = Self::decode_baseline(&v, MAX_DESTS);
        }
        if let Ok(Some(v)) = storage.baseline_get("known_processes").await {
            self.known_processes = Self::decode_baseline(&v, MAX_PROCS);
        }
        // Ports/services now persist too: restarts no longer re-blind or
        // re-storm. Absent rows mean first run — prime silently as before.
        if let Ok(Some(v)) = storage.baseline_get("known_ports").await {
            let map = Self::decode_baseline(&v, MAX_PORTS);
            let ports: HashSet<PortKey> = map.keys().filter_map(|k| PortKey::decode(k)).collect();
            if !ports.is_empty() {
                self.known_ports = ports;
                self.primed_ports = true;
            }
        }
        if let Ok(Some(v)) = storage.baseline_get("known_services").await {
            let map = Self::decode_baseline(&v, MAX_SERVICES);
            if !map.is_empty() {
                self.known_services = map.into_keys().collect();
                self.primed_services = true;
            }
        }
        self.sync_feedback(storage).await;
    }

    pub async fn persist_baselines(&self, storage: &Storage) {
        let now = Utc::now();
        let ttl = chrono::Duration::days(BASELINE_TTL_DAYS);
        let fresh_dest: HashMap<_, _> = self
            .seen_process_dest
            .iter()
            .filter(|(_, ts)| now.signed_duration_since(**ts) < ttl)
            .take(MAX_DESTS)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        Self::persist_one(storage, "seen_process_dest", &fresh_dest).await;
        let fresh_procs: HashMap<_, _> = self
            .known_processes
            .iter()
            .filter(|(_, ts)| now.signed_duration_since(**ts) < ttl)
            .take(MAX_PROCS)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        Self::persist_one(storage, "known_processes", &fresh_procs).await;
        let fresh_ports: HashMap<String, chrono::DateTime<Utc>> = self
            .known_ports
            .iter()
            .take(MAX_PORTS)
            .map(|k| (k.encode(), now))
            .collect();
        Self::persist_one(storage, "known_ports", &fresh_ports).await;
        let fresh_svcs: HashMap<String, chrono::DateTime<Utc>> = self
            .known_services
            .iter()
            .take(MAX_SERVICES)
            .map(|k| (k.clone(), now))
            .collect();
        Self::persist_one(storage, "known_services", &fresh_svcs).await;
        // Stale cooldown entries are pruned lazily on next allow().
    }

    async fn persist_one(
        storage: &Storage,
        key: &str,
        value: &HashMap<String, chrono::DateTime<Utc>>,
    ) {
        if let Ok(s) = serde_json::to_string(value)
            && let Err(e) = storage.baseline_set(key, &s).await
        {
            tracing::warn!(error = %e, key, "persist baseline failed");
        }
    }

    /// Returns true when emission is allowed (and stamps the key).
    /// Same key within `window` is suppressed — the re-alert-storm fix.
    fn allow(&mut self, key: &str, window: Duration) -> bool {
        let now = Instant::now();
        if self.cooldowns.len() > 4096 {
            self.cooldowns
                .retain(|_, t| now.duration_since(*t) < Duration::from_secs(86400));
        }
        match self.cooldowns.get(key) {
            Some(&t) if now.duration_since(t) < window => false,
            _ => {
                self.cooldowns.insert(key.to_string(), now);
                true
            }
        }
    }

    /// Refresh the suppression set from user feedback (false positives,
    /// acknowledged suppressions). Cached; re-synced at most every 60s.
    async fn sync_feedback(&mut self, storage: &Storage) {
        let now = Instant::now();
        if self
            .last_feedback_sync
            .is_some_and(|t| now.duration_since(t) < Duration::from_secs(60))
        {
            return;
        }
        self.last_feedback_sync = Some(now);
        match storage.suppressed_keys().await {
            Ok(keys) => {
                self.suppressed = keys.into_iter().collect();
            }
            Err(e) => tracing::warn!(error = %e, "feedback sync failed"),
        }
    }

    fn is_suppressed(&self, rule: &RuleId, key: &str) -> bool {
        self.suppressed
            .contains(&format!("{}|{key}", rule.as_str()))
            || self.suppressed.contains(&format!("{}|*", rule.as_str()))
    }

    pub async fn observe_processes(
        &mut self,
        bus: &EventBus,
        storage: &Storage,
        processes: &[ProcessInfo],
    ) {
        let primed = !self.known_processes.is_empty();
        let now = Utc::now();
        for p in processes {
            let key = p.name.to_lowercase();
            let is_new = !self.known_processes.contains_key(&key);
            // Refresh last-seen on every observation so TTL decay only
            // forgets truly idle entries.
            self.known_processes.insert(key, now);
            if primed
                && is_new
                && let Some(path) = &p.path
                && unusual_path(path)
            {
                emit(
                    bus,
                    storage,
                    RuleId::ProcessStartedUnusualPath,
                    vec!["new_process".into(), "started_from_unusual_path".into()],
                    Some(ProcessRef {
                        name: p.name.clone(),
                        pid: p.pid,
                        path: p.path.clone(),
                    }),
                    None,
                    None,
                    true,
                    serde_json::json!({ "path": path }),
                    format!("Process started from unusual path: {}", p.name),
                )
                .await;
            }
        }
    }

    pub async fn observe_ports(
        &mut self,
        bus: &EventBus,
        storage: &Storage,
        ports: &[ListeningPort],
    ) {
        let mut current = HashSet::new();
        for p in ports {
            current.insert(PortKey {
                proto: p.protocol.clone(),
                port: p.port,
                addr: p.address.clone(),
            });
        }
        if !self.primed_ports {
            self.known_ports = current;
            self.primed_ports = true;
            return;
        }
        let now = Utc::now();
        for p in ports {
            let key = PortKey {
                proto: p.protocol.clone(),
                port: p.port,
                addr: p.address.clone(),
            };
            if !self.known_ports.contains(&key) {
                if let Err(e) = storage.insert_port_transition(now, "PORT_OPENED", p).await {
                    tracing::warn!(error = %e, "insert_port_transition failed");
                }
                let mut indicators = vec!["listening_port_appeared".into()];
                if p.process_name.is_none() {
                    indicators.push("no_owning_process_resolved".into());
                }
                emit(
                    bus,
                    storage,
                    RuleId::NewListeningPort,
                    indicators,
                    p.process_name.as_ref().map(|n| ProcessRef {
                        name: n.clone(),
                        pid: p.pid.unwrap_or(0),
                        path: p.process_path.clone(),
                    }),
                    Some(format!("{}:{}", p.address, p.port)),
                    None,
                    true,
                    serde_json::json!({ "protocol": p.protocol, "port": p.port }),
                    format!("New listening port {}/{}", p.port, p.protocol),
                )
                .await;
            }
        }
        for old in &self.known_ports {
            if !current.contains(old) {
                let stub = ListeningPort {
                    protocol: old.proto.clone(),
                    address: old.addr.clone(),
                    port: old.port,
                    pid: None,
                    process_name: None,
                    process_path: None,
                };
                if let Err(e) = storage
                    .insert_port_transition(now, "PORT_CLOSED", &stub)
                    .await
                {
                    tracing::warn!(error = %e, "insert_port_transition failed");
                }
                emit(
                    bus,
                    storage,
                    RuleId::PortStateChanged,
                    vec!["listening_port_closed".into()],
                    None,
                    Some(format!("{}:{}", old.addr, old.port)),
                    None,
                    false,
                    serde_json::json!({ "protocol": old.proto, "port": old.port }),
                    format!("Listening port {} closed", old.port),
                )
                .await;
            }
        }
        self.known_ports = current;
    }

    pub async fn observe_connections(
        &mut self,
        bus: &EventBus,
        storage: &Storage,
        conns: &[Connection],
        processes: &[ProcessInfo],
        blocklist: &vigilon_core::BlocklistManager,
    ) {
        let now_i = Instant::now();
        let now = Utc::now();
        let known_paths: HashSet<String> =
            processes.iter().filter_map(|p| p.path.clone()).collect();
        for p in processes {
            self.known_processes.insert(p.name.to_lowercase(), now);
        }

        let mut rate_bucket: HashMap<String, u32> = HashMap::new();
        for c in conns {
            if is_external(&c.remote_addr) {
                let dest = format!("{}:{}", c.remote_addr, c.remote_port);
                self.remote_first_seen
                    .entry(c.remote_addr.clone())
                    .or_insert(now);
                self.conn_window.push_back((now_i, dest.clone()));
                *rate_bucket.entry(dest.clone()).or_default() += 1;

                let proc_key = c.process_name.clone().unwrap_or_else(|| "unknown".into());
                let combo = format!("{proc_key}|{dest}");

                // Direction-agnostic scan detection: one remote SYN-touching
                // many distinct local ports within 20s is a probe pattern,
                // whether the remote is LAN or WAN. ESTABLISHED traffic
                // (browsers, updaters) never enters this path.
                if c.state.to_uppercase().contains("SYN") {
                    // Scope the queue borrow so the cooldown check below can
                    // take &mut self without a double-borrow.
                    let distinct = {
                        let entry = self.scan_targets.entry(c.remote_addr.clone()).or_default();
                        entry.push_back((now_i, c.local_port));
                        while entry
                            .front()
                            .is_some_and(|(t, _)| now_i.duration_since(*t).as_secs() > 20)
                        {
                            entry.pop_front();
                        }
                        entry
                            .iter()
                            .map(|(_, p)| *p)
                            .collect::<std::collections::HashSet<u16>>()
                            .len()
                    };
                    if distinct >= 10
                        && self.allow(
                            &format!("portscan|{}", c.remote_addr),
                            Duration::from_secs(300),
                        )
                    {
                        emit(
                            bus,
                            storage,
                            RuleId::PortScanLikeInbound,
                            vec![
                                "rapid_distinct_or_repeated_inbound_attempts".into(),
                                format!("distinct_ports_in_20s:{distinct}"),
                            ],
                            None,
                            Some(format!("{}:{}", c.local_addr, c.local_port)),
                            Some(c.remote_addr.clone()),
                            true,
                            serde_json::json!({ "count": distinct }),
                            "Unusual inbound connection pattern (heuristic)".into(),
                        )
                        .await;
                        if let Some(entry) = self.scan_targets.get_mut(&c.remote_addr) {
                            entry.clear();
                        }
                    }
                }

                let first = !self.seen_process_dest.contains_key(&combo);
                if first {
                    self.seen_process_dest.insert(combo, now);
                    let mut indicators = vec!["first_external_destination".into()];
                    let new_proc = !self.known_processes.contains_key(&proc_key.to_lowercase())
                        || c.process_name.is_none();
                    if new_proc {
                        indicators.push("new_process".into());
                    }
                    if let Some(path) = &c.process_path
                        && unusual_path(path)
                    {
                        indicators.push("started_from_unusual_path".into());
                    } else if c.process_path.is_none() {
                        indicators.push("executable_path_unknown".into());
                    }
                    if let Some(proc) = processes.iter().find(|p| Some(p.pid) == c.pid)
                        && let Some(st) = proc.start_time
                        && (now - st).num_seconds().abs() < 15
                    {
                        indicators.push("connection_shortly_after_process_start".into());
                    }
                    // Explicit precedence: unknown process wins; otherwise an
                    // executable outside the known inventory AND on an unusual
                    // path is treated as unknown. Everything else is a plain
                    // new-external-connection finding.
                    let unknown_path = c
                        .process_path
                        .as_ref()
                        .is_some_and(|p| !known_paths.contains(p) && unusual_path(p));
                    let rule = if c.process_name.is_none() || unknown_path {
                        if c.process_name.is_none() {
                            RuleId::UnknownProcessNetworkAccess
                        } else {
                            RuleId::NewExternalConnection
                        }
                    } else {
                        RuleId::NewExternalConnection
                    };
                    emit(
                        bus,
                        storage,
                        rule,
                        indicators,
                        c.process_name.as_ref().map(|n| ProcessRef {
                            name: n.clone(),
                            pid: c.pid.unwrap_or(0),
                            path: c.process_path.clone(),
                        }),
                        Some(format!("{}:{}", c.local_addr, c.local_port)),
                        Some(dest.clone()),
                        true,
                        serde_json::json!({ "state": c.state, "protocol": c.protocol }),
                        "New outbound connection".into(),
                    )
                    .await;
                }

                if blocklist.is_blocked(&c.remote_addr).await
                    && !self.is_suppressed(&RuleId::BlocklistedConnection, &c.remote_addr)
                    && self.allow(
                        &format!("blocklisted|{}", c.remote_addr),
                        Duration::from_secs(900),
                    )
                {
                    emit(
                        bus,
                        storage,
                        RuleId::BlocklistedConnection,
                        vec!["blocklisted_ip".into(), "threat_intelligence_match".into()],
                        c.process_name.as_ref().map(|n| ProcessRef {
                            name: n.clone(),
                            pid: c.pid.unwrap_or(0),
                            path: c.process_path.clone(),
                        }),
                        Some(format!("{}:{}", c.local_addr, c.local_port)),
                        Some(dest),
                        true,
                        serde_json::json!({ "remote_ip": c.remote_addr, "remote_port": c.remote_port }),
                        format!("Connection to blocklisted IP {}", c.remote_addr),
                    )
                    .await;
                }
            } else if is_inbound_like(c) {
                let key = c.remote_addr.clone();
                // Scope the queue borrow so the cooldown check below can
                // take &mut self without a double-borrow.
                let hits = {
                    let q = self.inbound_hits.entry(key.clone()).or_default();
                    q.push_back(now_i);
                    while q
                        .front()
                        .is_some_and(|t| now_i.duration_since(*t).as_secs() > 20)
                    {
                        q.pop_front();
                    }
                    q.len()
                };
                if hits >= 12 && self.allow(&format!("portscan|{key}"), Duration::from_secs(300)) {
                    emit(
                        bus,
                        storage,
                        RuleId::PortScanLikeInbound,
                        vec![
                            "rapid_distinct_or_repeated_inbound_attempts".into(),
                            format!("hits_in_20s:{hits}"),
                        ],
                        None,
                        Some(format!("{}:{}", c.local_addr, c.local_port)),
                        Some(c.remote_addr.clone()),
                        true,
                        serde_json::json!({ "count": hits }),
                        "Unusual inbound connection pattern (heuristic)".into(),
                    )
                    .await;
                    if let Some(q) = self.inbound_hits.get_mut(&key) {
                        q.clear();
                    }
                }
            }
        }

        while self
            .conn_window
            .front()
            .is_some_and(|(t, _)| now_i.duration_since(*t).as_secs() > 10)
        {
            self.conn_window.pop_front();
        }
        for (dest, n) in rate_bucket {
            if n >= 20 && self.allow(&format!("rate|{dest}"), Duration::from_secs(300)) {
                emit(
                    bus,
                    storage,
                    RuleId::UnusualConnectionRate,
                    vec!["repeated_connection_attempts".into(), format!("count:{n}")],
                    None,
                    None,
                    Some(dest),
                    true,
                    serde_json::json!({ "count": n }),
                    "Unusual connection rate".into(),
                )
                .await;
            }
        }
        let _ = now;
    }

    pub async fn observe_services(
        &mut self,
        bus: &EventBus,
        storage: &Storage,
        services: &[ServiceInfo],
    ) {
        let now = Utc::now();
        let current: HashSet<String> = services
            .iter()
            .filter(|s| s.status == "running")
            .map(|s| s.name.clone())
            .collect();
        if !self.primed_services {
            self.known_services = current;
            self.primed_services = true;
            for s in services {
                if let Err(e) = storage.insert_service_event(now, s, "seen").await {
                    tracing::warn!(error = %e, "insert_service_event failed");
                }
            }
            return;
        }
        for s in services {
            if s.status == "running" && !self.known_services.contains(&s.name) {
                if let Err(e) = storage.insert_service_event(now, s, "started").await {
                    tracing::warn!(error = %e, "insert_service_event failed");
                }
                emit(
                    bus,
                    storage,
                    RuleId::NewServiceStarted,
                    vec!["service_entered_running_state".into()],
                    s.pid.map(|pid| ProcessRef {
                        name: s.name.clone(),
                        pid,
                        path: None,
                    }),
                    None,
                    None,
                    true,
                    serde_json::json!({ "service": s.name }),
                    format!("New service started: {}", s.name),
                )
                .await;
            }
        }
        self.known_services = current;
    }

    pub async fn observe_firewall(
        &mut self,
        bus: &EventBus,
        storage: &Storage,
        details: &str,
        enabled: bool,
    ) {
        let sig = format!("{enabled}|{details}");
        if let Some(prev) = &self.last_fw
            && prev != &sig
        {
            emit(
                bus,
                storage,
                RuleId::FirewallConfigurationChanged,
                vec!["firewall_state_or_profile_changed".into()],
                None,
                None,
                None,
                true,
                serde_json::json!({ "enabled": enabled, "details": details }),
                "Firewall configuration changed".into(),
            )
            .await;
        }
        self.last_fw = Some(sig);
    }

    pub async fn observe_net_config(
        &mut self,
        bus: &EventBus,
        storage: &Storage,
        ifaces: &[NetworkInterface],
    ) {
        let dns = ifaces
            .iter()
            .flat_map(|i| i.dns.clone())
            .collect::<Vec<_>>();
        let gw = ifaces.iter().find_map(|i| i.gateway.clone());
        if let Some(prev) = &self.last_dns
            && prev != &dns
            && !dns.is_empty()
        {
            emit(
                bus,
                storage,
                RuleId::NetworkConfigChanged,
                vec!["dns_servers_changed".into()],
                None,
                None,
                None,
                true,
                serde_json::json!({ "dns": dns }),
                "DNS configuration changed".into(),
            )
            .await;
        }
        if let Some(prev_gw) = &self.last_gw
            && prev_gw != &gw
            && gw.is_some()
        {
            emit(
                bus,
                storage,
                RuleId::NetworkConfigChanged,
                vec!["default_gateway_changed".into()],
                None,
                None,
                None,
                true,
                serde_json::json!({ "gateway": gw }),
                "Gateway configuration changed".into(),
            )
            .await;
        }
        if !dns.is_empty() {
            self.last_dns = Some(dns);
        }
        self.last_gw = Some(gw);
    }

    pub async fn observe_auth_failures(&mut self, bus: &EventBus, storage: &Storage, n: u32) {
        if n >= 4 {
            emit(
                bus,
                storage,
                RuleId::MultipleFailedLogins,
                vec![format!("failed_login_events:{n}")],
                None,
                None,
                None,
                true,
                serde_json::json!({ "count": n }),
                "Multiple failed logins observed".into(),
            )
            .await;
        }
    }

    pub fn stamp_remote_seen(&self, conns: &mut [Connection]) {
        for c in conns {
            if let Some(fs) = self.remote_first_seen.get(&c.remote_addr) {
                c.first_seen = Some(*fs);
                c.last_seen = Some(Utc::now());
            }
        }
    }

    pub async fn observe_dns_queries(
        &mut self,
        bus: &EventBus,
        storage: &Storage,
        queries: &[vigilon_core::metrics::DnsQuery],
        blocklist: &vigilon_core::BlocklistManager,
    ) {
        self.sync_feedback(storage).await;
        for q in queries {
            let qn = q.query_name.to_lowercase();
            let suspicious_tld = qn.ends_with(".tk")
                || qn.ends_with(".xyz")
                || qn.ends_with(".top")
                || qn.ends_with(".cc")
                || qn.ends_with(".buzz")
                || qn.ends_with(".work");

            if (suspicious_tld || blocklist.is_blocked(&qn).await)
                && !self.is_suppressed(&RuleId::SuspiciousDnsQuery, &qn)
                && self.allow(&format!("dns|{qn}"), Duration::from_secs(3600))
            {
                emit(
                    bus,
                    storage,
                    RuleId::SuspiciousDnsQuery,
                    vec!["suspicious_tld_or_blocklist".into()],
                    None,
                    None,
                    Some(q.query_name.clone()),
                    true,
                    serde_json::json!({
                        "query": q.query_name,
                        "record_type": q.record_type,
                    }),
                    format!("Query to suspicious domain {}", q.query_name),
                )
                .await;
            }
        }
    }

    pub async fn observe_bandwidth_quota(
        &mut self,
        bus: &EventBus,
        storage: &Storage,
        quota_gb: Option<f64>,
        warning_pct: u8,
        total_used_bytes: u64,
    ) {
        let Some(quota) = quota_gb else { return };
        if quota <= 0.0 {
            return;
        }
        let quota_bytes = (quota * 1024.0 * 1024.0 * 1024.0) as u64;
        let pct = (total_used_bytes as f64 / quota_bytes as f64) * 100.0;

        if pct >= 100.0 {
            if !self.allow("quota|exceeded", Duration::from_secs(82800)) {
                return;
            }
            emit(
                bus,
                storage,
                RuleId::BandwidthQuotaExceeded,
                vec!["quota_100_pct_exceeded".into()],
                None,
                None,
                None,
                false,
                serde_json::json!({
                    "quota_gb": quota,
                    "used_gb": (total_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0)),
                    "pct": pct
                }),
                format!("Monthly bandwidth quota exceeded ({:.1}%)", pct),
            )
            .await;
        } else if pct >= warning_pct as f64 {
            if !self.allow("quota|warning", Duration::from_secs(82800)) {
                return;
            }
            emit(
                bus,
                storage,
                RuleId::BandwidthQuotaWarning,
                vec!["quota_warning_threshold".into()],
                None,
                None,
                None,
                false,
                serde_json::json!({
                    "quota_gb": quota,
                    "warning_pct": warning_pct,
                    "pct": pct
                }),
                format!("Monthly bandwidth quota warning ({:.1}%)", pct),
            )
            .await;
        }
    }
}

fn is_external(addr: &str) -> bool {
    vigilon_core::netparse::is_external_addr(addr)
}

fn is_inbound_like(c: &Connection) -> bool {
    c.state.to_uppercase().contains("SYN") && !is_external(&c.local_addr)
}

fn unusual_path(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains("\\temp\\")
        || p.contains("/tmp/")
        || p.contains("downloads")
        || p.contains("appdata\\local\\temp")
}

/// `emit` intentionally takes the full finding context as arguments so
/// every call site stays explicit; the arity is the API.
#[allow(clippy::too_many_arguments)]
async fn emit(
    bus: &EventBus,
    storage: &Storage,
    rule: RuleId,
    indicators: Vec<String>,
    process: Option<ProcessRef>,
    local: Option<String>,
    remote: Option<String>,
    first_seen: bool,
    details: serde_json::Value,
    title: String,
) {
    // Emitters that count things (auth failures, rate buckets, scan hits)
    // stash the real count in `details.count`; score on that when present
    // so CRITICAL/HIGH arms are reachable by measurement, not accident.
    let count = details
        .get("count")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(indicators.len());
    let risk = score(&rule, count, first_seen);
    let event = SecurityEvent {
        event_type: "SECURITY_EVENT".into(),
        id: Uuid::new_v4().to_string(),
        rule,
        risk,
        timestamp: Utc::now(),
        process,
        local,
        remote,
        indicators,
        first_seen,
        details,
        assessment: String::new(),
    };
    let mut event = event;
    event.fill_assessment();
    let event = event;
    let timeline = TimelineEntry {
        id: Uuid::new_v4().to_string(),
        timestamp: event.timestamp,
        kind: event.rule.as_str().into(),
        risk: event.risk,
        title: title.clone(),
        summary: event.why_summary(),
        event_id: Some(event.id.clone()),
    };
    if let Err(e) = storage.insert_security_event(&event).await {
        tracing::error!(error = %e, rule = event.rule.as_str(), "insert_security_event failed");
    }
    if let Err(e) = storage.insert_timeline(&timeline).await {
        tracing::error!(error = %e, "insert_timeline failed");
    }
    bus.publish(BusEvent::Security(event));
    bus.publish(BusEvent::Timeline(timeline));
}

#[cfg(test)]
mod tests {
    use super::*;
    use vigilon_core::metrics::Connection;

    #[test]
    fn loopback_is_not_external() {
        assert!(!is_external("127.0.0.1"));
        assert!(!is_external("192.168.0.2"));
        assert!(is_external("8.8.8.8"));
    }

    #[test]
    fn temp_paths_are_unusual() {
        assert!(unusual_path(r"C:\Users\a\AppData\Local\Temp\x.exe"));
        assert!(!unusual_path(r"C:\Program Files\app\app.exe"));
    }

    fn test_conn(remote: &str) -> Connection {
        Connection {
            protocol: "TCP".into(),
            local_addr: "192.168.1.8".into(),
            local_port: 51234,
            remote_addr: remote.into(),
            remote_port: 443,
            state: "ESTABLISHED".into(),
            pid: Some(1234),
            process_name: Some("test-proc".into()),
            process_path: Some("/usr/bin/test-proc".into()),
            first_seen: None,
            last_seen: None,
            geo: None,
        }
    }

    async fn test_setup() -> (
        DetectionEngine,
        EventBus,
        Storage,
        vigilon_core::BlocklistManager,
    ) {
        let dir = tempfile::tempdir().unwrap();
        // Storage::connect runs migrations; keep the dir alive via leak for
        // the duration of the test (test-only shorthand).
        let dir = Box::leak(Box::new(dir));
        let storage = Storage::connect(&dir.path().join("t.sqlite"))
            .await
            .unwrap();
        let bus = EventBus::new();
        let blocklist = vigilon_core::BlocklistManager::new(dir.path());
        (DetectionEngine::new(), bus, storage, blocklist)
    }

    #[tokio::test]
    async fn first_seen_emits_once_then_dedupes() {
        let (mut engine, bus, storage, blocklist) = test_setup().await;
        let conns = vec![test_conn("93.184.216.34")];
        engine
            .observe_connections(&bus, &storage, &conns, &[], &blocklist)
            .await;
        let first = storage.latest_events(50).await.unwrap();
        assert_eq!(
            first.len(),
            1,
            "first sighting should emit exactly one event"
        );
        assert_eq!(first[0].rule, RuleId::NewExternalConnection);

        // Same connection on the next tick: no new finding.
        engine
            .observe_connections(&bus, &storage, &conns, &[], &blocklist)
            .await;
        let second = storage.latest_events(50).await.unwrap();
        assert_eq!(second.len(), 1, "repeat sighting must not re-emit");
    }

    #[tokio::test]
    async fn blocklisted_connection_emits_once_per_cooldown() {
        let (mut engine, bus, storage, blocklist) = test_setup().await;
        blocklist.add("93.184.216.34".into(), None).await.unwrap();
        let conns = vec![test_conn("93.184.216.34")];
        for _ in 0..3 {
            engine
                .observe_connections(&bus, &storage, &conns, &[], &blocklist)
                .await;
        }
        let events = storage.latest_events(50).await.unwrap();
        let blocked = events
            .iter()
            .filter(|e| e.rule == RuleId::BlocklistedConnection)
            .count();
        assert_eq!(blocked, 1, "blocklist storm must cool down to one emission");
    }

    #[tokio::test]
    async fn quota_warning_emits_once_per_day() {
        let (mut engine, bus, storage, _blocklist) = test_setup().await;
        for _ in 0..3 {
            engine
                .observe_bandwidth_quota(&bus, &storage, Some(1.0), 80, 950_000_000)
                .await;
        }
        let events = storage.latest_events(50).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].rule, RuleId::BandwidthQuotaWarning);
    }

    #[tokio::test]
    async fn suppressed_dns_name_does_not_emit() {
        let (mut engine, bus, storage, blocklist) = test_setup().await;
        storage
            .insert_feedback(
                None,
                "SUSPICIOUS_DNS_QUERY",
                "evil.tk",
                "false_positive",
                None,
            )
            .await
            .unwrap();
        let q = vigilon_core::metrics::DnsQuery {
            timestamp: Utc::now(),
            query_name: "evil.tk".into(),
            record_type: "A".into(),
            response_code: 0,
            process_name: None,
        };
        engine
            .observe_dns_queries(&bus, &storage, &[q], &blocklist)
            .await;
        let events = storage.latest_events(50).await.unwrap();
        assert!(events.is_empty(), "suppressed name must not emit");
    }

    #[tokio::test]
    async fn baselines_survive_reload() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::connect(&dir.path().join("t.sqlite"))
            .await
            .unwrap();
        let bus = EventBus::new();
        let blocklist = vigilon_core::BlocklistManager::new(dir.path());
        let mut engine = DetectionEngine::new();
        engine
            .observe_connections(
                &bus,
                &storage,
                &[test_conn("93.184.216.34")],
                &[],
                &blocklist,
            )
            .await;
        engine.persist_baselines(&storage).await;

        // A fresh engine with persisted baselines stays silent.
        let mut engine2 = DetectionEngine::new();
        engine2.load_baselines(&storage).await;
        engine2
            .observe_connections(
                &bus,
                &storage,
                &[test_conn("93.184.216.34")],
                &[],
                &blocklist,
            )
            .await;
        let events = storage.latest_events(50).await.unwrap();
        assert_eq!(events.len(), 1, "reloaded baseline must not re-fire");
    }
}
