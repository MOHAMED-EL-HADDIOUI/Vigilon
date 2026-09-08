use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel: Option<String>,
    pub cpu_model: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub base_frequency_mhz: Option<f32>,
    pub motherboard: Option<String>,
    pub bios_version: Option<String>,
    pub total_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub usage_percent: f32,
    pub per_core: Vec<f32>,
    pub frequency_mhz: Option<f32>,
    pub temperature_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub name: String,
    pub usage_percent: Option<f32>,
    pub memory_total_bytes: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub temperature_c: Option<f32>,
    pub processes: Vec<GpuProcess>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuProcess {
    pub pid: u32,
    pub name: Option<String>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub read_bps: f64,
    pub write_bps: f64,
    pub smart_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub present: bool,
    pub state: String,
    pub percentage: Option<f32>,
    pub health_percent: Option<f32>,
    pub time_to_empty_sec: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub kind: String,
    pub mac: Option<String>,
    pub ips: Vec<String>,
    pub is_up: bool,
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub rx_total_bytes: u64,
    pub tx_total_bytes: u64,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningPort {
    pub protocol: String,
    pub address: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub process_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub country_code: String,
    pub country_name: String,
    pub city: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub protocol: String,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub process_path: Option<String>,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub geo: Option<GeoLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessBandwidth {
    pub pid: u32,
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQuery {
    pub timestamp: DateTime<Utc>,
    pub query_name: String,
    pub record_type: String,
    pub response_code: u16,
    pub process_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthQuotaInfo {
    pub quota_gb: Option<f64>,
    pub used_rx_bytes: u64,
    pub used_tx_bytes: u64,
    pub month: String,
    pub warning_pct: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
    pub start_time: Option<DateTime<Utc>>,
    pub connection_count: u32,
    pub parent_pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub status: String,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallStatus {
    pub enabled: bool,
    pub profile: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub timestamp: DateTime<Utc>,
    pub device: DeviceInfo,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub gpus: Vec<GpuMetrics>,
    pub disks: Vec<DiskMetrics>,
    pub battery: Option<BatteryInfo>,
    pub network_rx_bps: f64,
    pub network_tx_bps: f64,
    pub firewall: Option<FirewallStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub gpu_usage: Option<f32>,
    pub disk_read_bps: f64,
    pub disk_write_bps: f64,
    pub network_rx_bps: f64,
    pub network_tx_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsCacheEntry {
    pub name: String,
    pub record_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub animated: bool,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraph {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}
