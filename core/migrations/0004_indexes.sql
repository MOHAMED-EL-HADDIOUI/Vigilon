-- Migration 0004: indexes for the hot read paths
-- (port history, service first-seen, change summary, correlate windows,
-- history scans) so point lookups stop full-scanning timestamped tables.

CREATE INDEX IF NOT EXISTS idx_ports_port_ts ON ports(port, timestamp);
CREATE INDEX IF NOT EXISTS idx_services_name_ts ON services(name, timestamp);
CREATE INDEX IF NOT EXISTS idx_processes_ts ON processes(timestamp);
CREATE INDEX IF NOT EXISTS idx_alerts_ts ON alerts(timestamp);
CREATE INDEX IF NOT EXISTS idx_cpu_metrics_ts ON cpu_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_memory_metrics_ts ON memory_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_gpu_metrics_ts ON gpu_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_disk_metrics_ts ON disk_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_network_interfaces_ts ON network_interfaces(timestamp);
CREATE INDEX IF NOT EXISTS idx_system_snapshots_ts ON system_snapshots(timestamp);
CREATE INDEX IF NOT EXISTS idx_security_events_ts ON security_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_timeline_ts ON timeline(timestamp);
CREATE INDEX IF NOT EXISTS idx_connections_ts ON connections(timestamp);
