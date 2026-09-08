export type Risk = "INFO" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";

export type Theme = "dark" | "light" | "system";

export interface DeviceInfo {
  id: string;
  hostname: string;
  os_name: string;
  os_version: string;
  kernel?: string | null;
  cpu_model: string;
  physical_cores: number;
  logical_cores: number;
  base_frequency_mhz?: number | null;
  motherboard?: string | null;
  bios_version?: string | null;
  total_memory_bytes: number;
}

export interface Snapshot {
  timestamp: string;
  device: DeviceInfo;
  cpu: { usage_percent: number; per_core: number[]; frequency_mhz?: number | null; temperature_c?: number | null };
  memory: {
    total_bytes: number;
    used_bytes: number;
    available_bytes: number;
    swap_total_bytes: number;
    swap_used_bytes: number;
  };
  gpus: Array<{
    name: string;
    usage_percent?: number | null;
    memory_total_bytes?: number | null;
    memory_used_bytes?: number | null;
    temperature_c?: number | null;
    processes?: Array<{ pid: number; name?: string | null; memory_bytes?: number | null }>;
  }>;
  disks: Array<{
    name: string;
    mount_point: string;
    file_system: string;
    total_bytes: number;
    used_bytes: number;
    read_bps: number;
    write_bps: number;
    smart_status?: string | null;
  }>;
  battery?: {
    present: boolean;
    state: string;
    percentage?: number | null;
    health_percent?: number | null;
  } | null;
  network_rx_bps: number;
  network_tx_bps: number;
  firewall?: { enabled: boolean; profile?: string | null; details: string } | null;
}

export interface ListeningPort {
  protocol: string;
  address: string;
  port: number;
  pid?: number | null;
  process_name?: string | null;
  process_path?: string | null;
}

export interface GeoLocation {
  country_code: string;
  country_name: string;
  city?: string | null;
}

export interface Connection {
  protocol: string;
  local_addr: string;
  local_port: number;
  remote_addr: string;
  remote_port: number;
  state: string;
  pid?: number | null;
  process_name?: string | null;
  process_path?: string | null;
  first_seen?: string | null;
  last_seen?: string | null;
  geo?: GeoLocation | null;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  path?: string | null;
  cpu_percent: number;
  memory_bytes: number;
  status: string;
  start_time?: string | null;
  connection_count: number;
  parent_pid?: number | null;
}

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | { [key: string]: JsonValue }
  | JsonValue[];

export interface SecurityEvent {
  type?: string;
  id: string;
  rule: string;
  risk: Risk;
  timestamp: string;
  process?: { name: string; pid: number; path?: string | null };
  local?: string | null;
  remote?: string | null;
  indicators: string[];
  first_seen: boolean;
  details?: JsonValue;
  assessment?: string;
}

export interface TimelineEntry {
  id: string;
  timestamp: string;
  kind: string;
  risk: Risk;
  title: string;
  summary: string;
  event_id?: string | null;
}

export interface HistoryPoint {
  timestamp: string;
  cpu_usage: number;
  ram_usage: number;
  gpu_usage?: number | null;
  disk_read_bps: number;
  disk_write_bps: number;
  network_rx_bps: number;
  network_tx_bps: number;
}

export interface TopologyGraph {
  nodes: Array<{ id: string; kind: string; label: string; subtitle?: string | null }>;
  edges: Array<{ id: string; source: string; target: string; animated: boolean; label?: string | null }>;
}

export interface NetworkInterface {
  name: string;
  kind: string;
  mac?: string | null;
  ips: string[];
  is_up: boolean;
  rx_bps: number;
  tx_bps: number;
  rx_total_bytes: number;
  tx_total_bytes: number;
  gateway?: string | null;
  dns: string[];
}

export interface ServiceInfo {
  name: string;
  display_name?: string | null;
  status: string;
  pid?: number | null;
}

export interface PortTransition {
  event_type: string;
  port: number;
  timestamp: string;
  process_name?: string | null;
}

export interface ChangesSummary {
  since: string;
  hours: number;
  port_transitions: number;
  security_events: number;
  service_events: number;
  distinct_remotes: number;
}

export interface RemoteIpSighting {
  ip: string;
  first_seen: string;
  last_seen: string;
}

export interface DnsCacheEntry {
  name: string;
  record_type: string;
}

export interface CorrelatedActivity {
  timestamp: string;
  events: SecurityEvent[];
  ports: PortTransition[];
  connections: Array<{ local: string; remote: string; process_name?: string | null }>;
}

export interface BackendMeta {
  ok: boolean;
  name: string;
  local_first: boolean;
  phone_home: boolean;
}

export interface ServiceFirstSeen {
  first_seen: string | null;
}

export interface RemoteEndpointGroup {
  addr: string;
  count: number;
  ports: number[];
  processes: string[];
}

export type ProcessSortKey = keyof ProcessInfo;

export interface ProcessBandwidth {
  pid: number;
  name: string;
  rx_bytes: number;
  tx_bytes: number;
}

export interface DnsQuery {
  timestamp: string;
  query_name: string;
  record_type: string;
  response_code: number;
  process_name?: string | null;
}

export interface BandwidthQuotaInfo {
  quota_gb?: number | null;
  used_rx_bytes: number;
  used_tx_bytes: number;
  month: string;
  warning_pct: number;
}

export interface BlocklistEntry {
  ip: string;
  comment?: string | null;
  added_at: string;
}

export interface FleetAgent {
  address: string;
  status: string;
}

export interface TopDnsDomain {
  domain: string;
  count: number;
}

export type WsEvent =
  | ({ type: "METRICS" } & Snapshot)
  | ({ type: "SECURITY_EVENT" } & SecurityEvent)
  | ({ type: "TIMELINE" } & TimelineEntry)
  | { type: "PORTS"; timestamp: string; items: ListeningPort[] }
  | { type: "CONNECTIONS"; timestamp: string; items: Connection[] }
  | { type: "PROCESSES"; timestamp: string; items: ProcessInfo[] }
  | { type: "INTERFACES"; timestamp: string; items: NetworkInterface[] }
  | { type: "SERVICES"; timestamp: string; items: ServiceInfo[] }
  | { type: "DNS_CACHE"; timestamp: string; items: DnsCacheEntry[] }
  | { type: "PROCESS_BANDWIDTH"; timestamp: string; items: ProcessBandwidth[] }
  | { type: "DNS_QUERIES"; timestamp: string; items: DnsQuery[] };

