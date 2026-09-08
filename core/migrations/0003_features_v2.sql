-- Migration 0003: Vigilon v2 tables (DNS queries, Bandwidth monthly tracking, Remote agents)

CREATE TABLE IF NOT EXISTS dns_queries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    query_name TEXT NOT NULL,
    record_type TEXT NOT NULL,
    response_code INTEGER NOT NULL,
    process_name TEXT
);
CREATE INDEX IF NOT EXISTS idx_dns_ts ON dns_queries(timestamp);
CREATE INDEX IF NOT EXISTS idx_dns_query ON dns_queries(query_name);

CREATE TABLE IF NOT EXISTS bandwidth_monthly (
    month TEXT PRIMARY KEY,
    rx_bytes INTEGER NOT NULL DEFAULT 0,
    tx_bytes INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS remote_agents (
    id TEXT PRIMARY KEY,
    address TEXT NOT NULL UNIQUE,
    label TEXT,
    last_seen TEXT,
    status TEXT
);
