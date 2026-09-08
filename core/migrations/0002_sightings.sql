-- Event-driven connection history: first/last seen per 5-tuple, not per-second dumps.

CREATE TABLE IF NOT EXISTS connection_sightings (
    protocol TEXT NOT NULL,
    local_addr TEXT NOT NULL,
    local_port INTEGER NOT NULL,
    remote_addr TEXT NOT NULL,
    remote_port INTEGER NOT NULL,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    pid INTEGER,
    process_name TEXT,
    process_path TEXT,
    PRIMARY KEY (protocol, local_addr, local_port, remote_addr, remote_port)
);
CREATE INDEX IF NOT EXISTS idx_sight_remote ON connection_sightings(remote_addr);
CREATE INDEX IF NOT EXISTS idx_sight_last ON connection_sightings(last_seen);
