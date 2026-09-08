-- WAL is enabled at connect time; this is the v1 local-first schema.

CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    os_name TEXT,
    os_version TEXT,
    kernel TEXT,
    cpu_model TEXT,
    physical_cores INTEGER,
    logical_cores INTEGER,
    total_memory_bytes INTEGER,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS system_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    cpu_usage REAL,
    ram_usage REAL,
    gpu_usage REAL,
    gpu_memory REAL,
    disk_read REAL,
    disk_write REAL,
    network_rx REAL,
    network_tx REAL
);
CREATE INDEX IF NOT EXISTS idx_snapshots_ts ON system_snapshots(timestamp);

CREATE TABLE IF NOT EXISTS cpu_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    usage_percent REAL NOT NULL,
    frequency_mhz REAL,
    per_core_json TEXT,
    temperature_c REAL
);

CREATE TABLE IF NOT EXISTS memory_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    total_bytes INTEGER NOT NULL,
    used_bytes INTEGER NOT NULL,
    available_bytes INTEGER NOT NULL,
    swap_total_bytes INTEGER,
    swap_used_bytes INTEGER
);

CREATE TABLE IF NOT EXISTS gpu_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    name TEXT,
    usage_percent REAL,
    memory_total_bytes INTEGER,
    memory_used_bytes INTEGER,
    temperature_c REAL,
    processes_json TEXT
);

CREATE TABLE IF NOT EXISTS disk_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    name TEXT,
    mount_point TEXT,
    total_bytes INTEGER,
    used_bytes INTEGER,
    read_bps REAL,
    write_bps REAL,
    smart_status TEXT
);

CREATE TABLE IF NOT EXISTS network_interfaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    name TEXT NOT NULL,
    mac TEXT,
    ips_json TEXT,
    is_up INTEGER,
    rx_bps REAL,
    tx_bps REAL,
    kind TEXT
);

CREATE TABLE IF NOT EXISTS connections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    protocol TEXT,
    local_addr TEXT,
    local_port INTEGER,
    remote_addr TEXT,
    remote_port INTEGER,
    state TEXT,
    pid INTEGER,
    process_name TEXT,
    process_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_conn_ts ON connections(timestamp);
CREATE INDEX IF NOT EXISTS idx_conn_remote ON connections(remote_addr);

CREATE TABLE IF NOT EXISTS ports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    protocol TEXT,
    port INTEGER,
    address TEXT,
    pid INTEGER,
    process_name TEXT,
    process_path TEXT
);

CREATE TABLE IF NOT EXISTS processes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    pid INTEGER,
    name TEXT,
    path TEXT,
    cpu_percent REAL,
    memory_bytes INTEGER,
    status TEXT,
    start_time TEXT,
    connection_count INTEGER
);

CREATE TABLE IF NOT EXISTS services (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    name TEXT,
    display_name TEXT,
    status TEXT,
    pid INTEGER,
    event_type TEXT
);

CREATE TABLE IF NOT EXISTS security_events (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    rule TEXT NOT NULL,
    risk TEXT NOT NULL,
    process_name TEXT,
    process_pid INTEGER,
    process_path TEXT,
    local_endpoint TEXT,
    remote_endpoint TEXT,
    indicators_json TEXT NOT NULL,
    details_json TEXT,
    first_seen INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_ts ON security_events(timestamp);

CREATE TABLE IF NOT EXISTS alerts (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    risk TEXT NOT NULL,
    title TEXT NOT NULL,
    acknowledged INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS timeline (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    kind TEXT NOT NULL,
    risk TEXT NOT NULL,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    event_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_timeline_ts ON timeline(timestamp);

CREATE TABLE IF NOT EXISTS baselines (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
