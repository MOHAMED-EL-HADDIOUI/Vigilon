mod battery;
pub mod dns;
pub mod dns_logger;
mod firewall;
pub mod geoip;
mod hardware;
mod net;
mod processes;
mod services;

pub use geoip::GeoIpResolver;

use vigilon_core::metrics::{
    BatteryInfo, Connection, CpuMetrics, DeviceInfo, DiskMetrics, DnsCacheEntry, DnsQuery,
    FirewallStatus, GpuMetrics, ListeningPort, MemoryMetrics, NetworkInterface, ProcessBandwidth,
    ProcessInfo, ServiceInfo,
};

/// Shared collector surface. Linux, Windows, and macOS implement the same methods
/// with native APIs underneath — internals are not forced into one code path.
pub trait Collectors {
    fn device(&mut self) -> DeviceInfo;
    fn cpu_memory(&mut self) -> (CpuMetrics, MemoryMetrics);
    fn gpus(&self) -> Vec<GpuMetrics>;
    fn disks(&mut self) -> Vec<DiskMetrics>;
    fn battery(&self) -> Option<BatteryInfo>;
    fn interfaces(&mut self) -> Vec<NetworkInterface>;
    fn sockets(&mut self) -> (Vec<ListeningPort>, Vec<Connection>);
    fn process_list(&mut self) -> Vec<ProcessInfo>;
    fn services(&self) -> Vec<ServiceInfo>;
    fn firewall(&self) -> Option<FirewallStatus>;
    fn auth_failed_recent(&self) -> u32;
    fn dns_cache(&self) -> Vec<DnsCacheEntry>;
}

pub struct Platform {
    hardware: hardware::HardwareCollector,
    net: net::NetCollector,
    processes: processes::ProcessCollector,
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform {
    pub fn new() -> Self {
        Self {
            hardware: hardware::HardwareCollector::new(),
            net: net::NetCollector::new(),
            processes: processes::ProcessCollector::new(),
        }
    }

    pub fn device(&mut self) -> DeviceInfo {
        self.hardware.device()
    }

    pub fn cpu_memory(&mut self) -> (CpuMetrics, MemoryMetrics) {
        self.hardware.cpu_memory()
    }

    pub fn gpus(&self) -> Vec<GpuMetrics> {
        hardware::gpus()
    }

    pub fn disks(&mut self) -> Vec<DiskMetrics> {
        self.hardware.disks()
    }

    pub fn battery(&self) -> Option<BatteryInfo> {
        battery::read()
    }

    pub fn interfaces(&mut self) -> Vec<NetworkInterface> {
        self.net.interfaces()
    }

    pub fn sockets(&mut self) -> (Vec<ListeningPort>, Vec<Connection>) {
        self.net.sockets(&mut self.processes)
    }

    pub fn process_list(&mut self) -> Vec<ProcessInfo> {
        self.processes.list()
    }

    pub fn attach_connection_counts(
        &mut self,
        processes: &mut [ProcessInfo],
        conns: &[Connection],
    ) {
        processes::attach_counts(processes, conns);
    }

    pub fn services(&self) -> Vec<ServiceInfo> {
        services::list()
    }

    pub fn firewall(&self) -> Option<FirewallStatus> {
        firewall::status()
    }

    pub fn auth_failed_recent(&self) -> u32 {
        services::failed_logins_hint()
    }

    pub fn dns_and_gateway(&self) -> (Vec<String>, Option<String>) {
        net::dns_and_gateway()
    }

    pub fn dns_cache(&self) -> Vec<DnsCacheEntry> {
        dns::cache_snapshot()
    }

    pub fn dns_queries(&self) -> Vec<DnsQuery> {
        dns::query_snapshot()
    }

    pub fn process_bandwidth(
        &self,
        conns: &[Connection],
        rx_bps: f64,
        tx_bps: f64,
    ) -> Vec<ProcessBandwidth> {
        use std::collections::HashMap;
        let mut map: HashMap<u32, (String, u32)> = HashMap::new();
        let mut total_conns = 0u32;
        for c in conns {
            if let Some(pid) = c.pid {
                let name = c
                    .process_name
                    .clone()
                    .unwrap_or_else(|| format!("PID {pid}"));
                let entry = map.entry(pid).or_insert((name, 0));
                entry.1 += 1;
                total_conns += 1;
            }
        }

        if total_conns == 0 {
            return Vec::new();
        }

        let mut out: Vec<ProcessBandwidth> = map
            .into_iter()
            .map(|(pid, (name, count))| {
                let frac = (count as f64) / (total_conns as f64);
                ProcessBandwidth {
                    pid,
                    name,
                    rx_bytes: (rx_bps * frac) as u64,
                    tx_bytes: (tx_bps * frac) as u64,
                }
            })
            .collect();

        out.sort_by_key(|b| std::cmp::Reverse(b.rx_bytes + b.tx_bytes));
        out
    }
}

impl Collectors for Platform {
    fn device(&mut self) -> DeviceInfo {
        Platform::device(self)
    }
    fn cpu_memory(&mut self) -> (CpuMetrics, MemoryMetrics) {
        Platform::cpu_memory(self)
    }
    fn gpus(&self) -> Vec<GpuMetrics> {
        Platform::gpus(self)
    }
    fn disks(&mut self) -> Vec<DiskMetrics> {
        Platform::disks(self)
    }
    fn battery(&self) -> Option<BatteryInfo> {
        Platform::battery(self)
    }
    fn interfaces(&mut self) -> Vec<NetworkInterface> {
        Platform::interfaces(self)
    }
    fn sockets(&mut self) -> (Vec<ListeningPort>, Vec<Connection>) {
        Platform::sockets(self)
    }
    fn process_list(&mut self) -> Vec<ProcessInfo> {
        Platform::process_list(self)
    }
    fn services(&self) -> Vec<ServiceInfo> {
        Platform::services(self)
    }
    fn firewall(&self) -> Option<FirewallStatus> {
        Platform::firewall(self)
    }
    fn auth_failed_recent(&self) -> u32 {
        Platform::auth_failed_recent(self)
    }
    fn dns_cache(&self) -> Vec<DnsCacheEntry> {
        Platform::dns_cache(self)
    }
}
