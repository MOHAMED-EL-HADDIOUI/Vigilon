use std::path::Path;

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

use crate::events::{Alert, RiskLevel, RuleId, SecurityEvent, TimelineEntry};
use crate::metrics::{
    Connection, DeviceInfo, HistoryPoint, ListeningPort, ProcessInfo, ServiceInfo, Snapshot,
};

#[derive(Clone)]
pub struct Storage {
    pool: Pool<Sqlite>,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl Storage {
    pub async fn connect(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn upsert_device(&self, d: &DeviceInfo) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO devices (id, hostname, os_name, os_version, kernel, cpu_model, physical_cores, logical_cores, total_memory_bytes, first_seen, last_seen)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET hostname=excluded.hostname, os_name=excluded.os_name, os_version=excluded.os_version, last_seen=excluded.last_seen, total_memory_bytes=excluded.total_memory_bytes",
        )
        .bind(&d.id)
        .bind(&d.hostname)
        .bind(&d.os_name)
        .bind(&d.os_version)
        .bind(&d.kernel)
        .bind(&d.cpu_model)
        .bind(d.physical_cores as i64)
        .bind(d.logical_cores as i64)
        .bind(d.total_memory_bytes as i64)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_snapshot(&self, s: &Snapshot) -> Result<(), StorageError> {
        let ts = s.timestamp.to_rfc3339();
        let ram_pct = if s.memory.total_bytes == 0 {
            0.0
        } else {
            s.memory.used_bytes as f32 / s.memory.total_bytes as f32 * 100.0
        };
        let gpu = s.gpus.first().and_then(|g| g.usage_percent);
        let disk_read: f64 = s.disks.iter().map(|d| d.read_bps).sum();
        let disk_write: f64 = s.disks.iter().map(|d| d.write_bps).sum();

        // One transaction for the whole snapshot fan-out (system + cpu +
        // memory + N gpu + M disk rows) instead of N+M+3 autocommits.
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO system_snapshots (device_id, timestamp, cpu_usage, ram_usage, gpu_usage, gpu_memory, disk_read, disk_write, network_rx, network_tx)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&s.device.id)
        .bind(&ts)
        .bind(s.cpu.usage_percent)
        .bind(ram_pct)
        .bind(gpu)
        .bind(s.gpus.first().and_then(|g| g.memory_used_bytes.map(|v| v as f64)))
        .bind(disk_read)
        .bind(disk_write)
        .bind(s.network_rx_bps)
        .bind(s.network_tx_bps)
        .execute(&mut *tx)
        .await?;

        let cores = serde_json::to_string(&s.cpu.per_core).unwrap_or_else(|_| "[]".into());
        sqlx::query(
            "INSERT INTO cpu_metrics (timestamp, usage_percent, frequency_mhz, per_core_json, temperature_c) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&ts)
        .bind(s.cpu.usage_percent)
        .bind(s.cpu.frequency_mhz)
        .bind(cores)
        .bind(s.cpu.temperature_c)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO memory_metrics (timestamp, total_bytes, used_bytes, available_bytes, swap_total_bytes, swap_used_bytes) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&ts)
        .bind(s.memory.total_bytes as i64)
        .bind(s.memory.used_bytes as i64)
        .bind(s.memory.available_bytes as i64)
        .bind(s.memory.swap_total_bytes as i64)
        .bind(s.memory.swap_used_bytes as i64)
        .execute(&mut *tx)
        .await?;

        for g in &s.gpus {
            let procs = serde_json::to_string(&g.processes).unwrap_or_else(|_| "[]".into());
            sqlx::query(
                "INSERT INTO gpu_metrics (timestamp, name, usage_percent, memory_total_bytes, memory_used_bytes, temperature_c, processes_json) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&ts)
            .bind(&g.name)
            .bind(g.usage_percent)
            .bind(g.memory_total_bytes.map(|v| v as i64))
            .bind(g.memory_used_bytes.map(|v| v as i64))
            .bind(g.temperature_c)
            .bind(procs)
            .execute(&mut *tx)
            .await?;
        }

        for d in &s.disks {
            sqlx::query(
                "INSERT INTO disk_metrics (timestamp, name, mount_point, total_bytes, used_bytes, read_bps, write_bps, smart_status) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&ts)
            .bind(&d.name)
            .bind(&d.mount_point)
            .bind(d.total_bytes as i64)
            .bind(d.used_bytes as i64)
            .bind(d.read_bps)
            .bind(d.write_bps)
            .bind(&d.smart_status)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn insert_interfaces(
        &self,
        ts: DateTime<Utc>,
        items: &[crate::metrics::NetworkInterface],
    ) -> Result<(), StorageError> {
        let ts = ts.to_rfc3339();
        for i in items {
            let ips = serde_json::to_string(&i.ips).unwrap_or_else(|_| "[]".into());
            sqlx::query(
                "INSERT INTO network_interfaces (timestamp, name, mac, ips_json, is_up, rx_bps, tx_bps, kind) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&ts)
            .bind(&i.name)
            .bind(&i.mac)
            .bind(ips)
            .bind(i.is_up as i64)
            .bind(i.rx_bps)
            .bind(i.tx_bps)
            .bind(&i.kind)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Insert a history row only for newly observed 5-tuples; refresh last_seen otherwise.
    /// The whole tick batch runs in a single transaction, and first-seen
    /// detection is a single `INSERT..SELECT WHERE NOT EXISTS` per
    /// connection — no SELECT-then-INSERT round trips.
    pub async fn upsert_connection_sightings(
        &self,
        ts: DateTime<Utc>,
        items: &[Connection],
    ) -> Result<u32, StorageError> {
        let ts = ts.to_rfc3339();
        let mut tx = self.pool.begin().await?;
        let mut new_count = 0u32;
        for c in items.iter().take(500) {
            let inserted = sqlx::query(
                "INSERT INTO connections (timestamp, protocol, local_addr, local_port, remote_addr, remote_port, state, pid, process_name, process_path)
                 SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                 WHERE NOT EXISTS (
                   SELECT 1 FROM connection_sightings
                   WHERE protocol=? AND local_addr=? AND local_port=? AND remote_addr=? AND remote_port=?
                 )",
            )
            .bind(&ts)
            .bind(&c.protocol)
            .bind(&c.local_addr)
            .bind(c.local_port as i64)
            .bind(&c.remote_addr)
            .bind(c.remote_port as i64)
            .bind(&c.state)
            .bind(c.pid.map(|p| p as i64))
            .bind(&c.process_name)
            .bind(&c.process_path)
            .bind(&c.protocol)
            .bind(&c.local_addr)
            .bind(c.local_port as i64)
            .bind(&c.remote_addr)
            .bind(c.remote_port as i64)
            .execute(&mut *tx)
            .await?;
            new_count += inserted.rows_affected() as u32;
            sqlx::query(
                "INSERT INTO connection_sightings (protocol, local_addr, local_port, remote_addr, remote_port, first_seen, last_seen, pid, process_name, process_path)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(protocol, local_addr, local_port, remote_addr, remote_port) DO UPDATE SET
                   last_seen=excluded.last_seen, pid=excluded.pid, process_name=excluded.process_name, process_path=excluded.process_path",
            )
            .bind(&c.protocol)
            .bind(&c.local_addr)
            .bind(c.local_port as i64)
            .bind(&c.remote_addr)
            .bind(c.remote_port as i64)
            .bind(&ts)
            .bind(&ts)
            .bind(c.pid.map(|p| p as i64))
            .bind(&c.process_name)
            .bind(&c.process_path)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(new_count)
    }

    pub async fn insert_connections(
        &self,
        ts: DateTime<Utc>,
        items: &[Connection],
    ) -> Result<(), StorageError> {
        let _ = self.upsert_connection_sightings(ts, items).await?;
        Ok(())
    }

    pub async fn insert_port_transition(
        &self,
        ts: DateTime<Utc>,
        event_type: &str,
        p: &ListeningPort,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO ports (timestamp, event_type, protocol, port, address, pid, process_name, process_path) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(ts.to_rfc3339())
        .bind(event_type)
        .bind(&p.protocol)
        .bind(p.port as i64)
        .bind(&p.address)
        .bind(p.pid.map(|v| v as i64))
        .bind(&p.process_name)
        .bind(&p.process_path)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_processes(
        &self,
        ts: DateTime<Utc>,
        items: &[ProcessInfo],
    ) -> Result<(), StorageError> {
        let ts = ts.to_rfc3339();
        let mut tx = self.pool.begin().await?;
        // Bound matches the collector cap (`MAX_PROCESSES` in platform).
        for p in items.iter().take(250) {
            sqlx::query(
                "INSERT INTO processes (timestamp, pid, name, path, cpu_percent, memory_bytes, status, start_time, connection_count) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&ts)
            .bind(p.pid as i64)
            .bind(&p.name)
            .bind(&p.path)
            .bind(p.cpu_percent)
            .bind(p.memory_bytes as i64)
            .bind(&p.status)
            .bind(p.start_time.map(|t| t.to_rfc3339()))
            .bind(p.connection_count as i64)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn insert_service_event(
        &self,
        ts: DateTime<Utc>,
        s: &ServiceInfo,
        event_type: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO services (timestamp, name, display_name, status, pid, event_type) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(ts.to_rfc3339())
        .bind(&s.name)
        .bind(&s.display_name)
        .bind(&s.status)
        .bind(s.pid.map(|p| p as i64))
        .bind(event_type)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_security_event(&self, e: &SecurityEvent) -> Result<(), StorageError> {
        let mut e = e.clone();
        e.fill_assessment();
        let e = &e;
        let indicators = serde_json::to_string(&e.indicators).unwrap_or_else(|_| "[]".into());
        let details = e.details.to_string();
        sqlx::query(
            "INSERT INTO security_events (id, timestamp, rule, risk, process_name, process_pid, process_path, local_endpoint, remote_endpoint, indicators_json, details_json, first_seen)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&e.id)
        .bind(e.timestamp.to_rfc3339())
        .bind(e.rule.as_str())
        .bind(e.risk.as_str())
        .bind(e.process.as_ref().map(|p| p.name.as_str()))
        .bind(e.process.as_ref().map(|p| p.pid as i64))
        .bind(e.process.as_ref().and_then(|p| p.path.clone()))
        .bind(&e.local)
        .bind(&e.remote)
        .bind(indicators)
        .bind(details)
        .bind(e.first_seen as i64)
        .execute(&self.pool)
        .await?;

        let alert = Alert {
            id: Uuid::new_v4().to_string(),
            event_id: e.id.clone(),
            timestamp: e.timestamp,
            risk: e.risk,
            title: format!("{} ({})", e.rule.as_str(), e.risk.as_str()),
            acknowledged: false,
        };
        sqlx::query(
            "INSERT INTO alerts (id, event_id, timestamp, risk, title, acknowledged) VALUES (?, ?, ?, ?, ?, 0)",
        )
        .bind(&alert.id)
        .bind(&alert.event_id)
        .bind(alert.timestamp.to_rfc3339())
        .bind(alert.risk.as_str())
        .bind(&alert.title)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_timeline(&self, t: &TimelineEntry) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO timeline (id, timestamp, kind, risk, title, summary, event_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&t.id)
        .bind(t.timestamp.to_rfc3339())
        .bind(&t.kind)
        .bind(t.risk.as_str())
        .bind(&t.title)
        .bind(&t.summary)
        .bind(&t.event_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn baseline_get(&self, key: &str) -> Result<Option<String>, StorageError> {
        let row = sqlx::query("SELECT value FROM baselines WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    pub async fn baseline_set(&self, key: &str, value: &str) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO baselines (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn history(&self, hours: i64) -> Result<Vec<HistoryPoint>, StorageError> {
        let since = (Utc::now() - chrono::Duration::hours(hours)).to_rfc3339();
        // Downsample in SQL: bucket rows into at most ~2000 points so a
        // 7-day window (300k+ rows at 2s cadence) never materializes fully.
        let stride: i64 = match hours {
            h if h <= 6 => 1,
            h if h <= 24 => 4,
            h if h <= 72 => 12,
            _ => 30,
        };
        let rows = sqlx::query(
            "SELECT timestamp, cpu_usage, ram_usage, gpu_usage, disk_read, disk_write, network_rx, network_tx
             FROM system_snapshots WHERE timestamp >= ? AND (rowid % ? = 0)
             ORDER BY timestamp ASC LIMIT 5000",
        )
        .bind(since)
        .bind(stride)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ts: String = r.get("timestamp");
                Some(HistoryPoint {
                    timestamp: DateTime::parse_from_rfc3339(&ts).ok()?.with_timezone(&Utc),
                    cpu_usage: r.get("cpu_usage"),
                    ram_usage: r.get("ram_usage"),
                    gpu_usage: r.get("gpu_usage"),
                    disk_read_bps: r.get("disk_read"),
                    disk_write_bps: r.get("disk_write"),
                    network_rx_bps: r.get("network_rx"),
                    network_tx_bps: r.get("network_tx"),
                })
            })
            .collect())
    }

    pub async fn latest_events(&self, limit: i64) -> Result<Vec<SecurityEvent>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, timestamp, rule, risk, process_name, process_pid, process_path, local_endpoint, remote_endpoint, indicators_json, details_json, first_seen
             FROM security_events ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().filter_map(|r| row_to_event(&r)).collect())
    }

    pub async fn event_by_id(&self, id: &str) -> Result<Option<SecurityEvent>, StorageError> {
        let row = sqlx::query(
            "SELECT id, timestamp, rule, risk, process_name, process_pid, process_path, local_endpoint, remote_endpoint, indicators_json, details_json, first_seen
             FROM security_events WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| row_to_event(&r)))
    }

    pub async fn timeline(&self, limit: i64) -> Result<Vec<TimelineEntry>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, timestamp, kind, risk, title, summary, event_id FROM timeline ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ts: String = r.get("timestamp");
                let risk_s: String = r.get("risk");
                Some(TimelineEntry {
                    id: r.get("id"),
                    timestamp: DateTime::parse_from_rfc3339(&ts).ok()?.with_timezone(&Utc),
                    kind: r.get("kind"),
                    risk: parse_risk(&risk_s),
                    title: r.get("title"),
                    summary: r.get("summary"),
                    event_id: r.get("event_id"),
                })
            })
            .collect())
    }

    pub async fn alerts(&self, limit: i64) -> Result<Vec<Alert>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, event_id, timestamp, risk, title, acknowledged FROM alerts ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let ts: String = r.get("timestamp");
                let risk_s: String = r.get("risk");
                let ack: i64 = r.get("acknowledged");
                Some(Alert {
                    id: r.get("id"),
                    event_id: r.get("event_id"),
                    timestamp: DateTime::parse_from_rfc3339(&ts).ok()?.with_timezone(&Utc),
                    risk: parse_risk(&risk_s),
                    title: r.get("title"),
                    acknowledged: ack != 0,
                })
            })
            .collect())
    }

    pub async fn prune(&self, days: i64) -> Result<(), StorageError> {
        let bound = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        // Every table carrying a timestamp participates in retention —
        // previously security_events/alerts/timeline/services/ports/dns and
        // the sightings table grew without bound.
        for table in [
            "system_snapshots",
            "cpu_metrics",
            "memory_metrics",
            "gpu_metrics",
            "disk_metrics",
            "network_interfaces",
            "connections",
            "processes",
            "security_events",
            "alerts",
            "timeline",
            "services",
            "ports",
            "dns_queries",
        ] {
            let q = format!("DELETE FROM {table} WHERE timestamp < ?");
            sqlx::query(&q).bind(&bound).execute(&self.pool).await?;
        }
        // Sightings keyed by last_seen, not insertion time.
        sqlx::query("DELETE FROM connection_sightings WHERE last_seen < ?")
            .bind(&bound)
            .execute(&self.pool)
            .await?;
        // Keep at most ~13 monthly bandwidth buckets; baselines are small
        // key/value rows managed by the engine's own decay.
        sqlx::query(
            "DELETE FROM bandwidth_monthly WHERE month NOT IN (
               SELECT month FROM bandwidth_monthly ORDER BY month DESC LIMIT 13
             )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remote_ips_since(
        &self,
        hours: i64,
    ) -> Result<Vec<(String, String, String)>, StorageError> {
        let since = (Utc::now() - chrono::Duration::hours(hours)).to_rfc3339();
        let rows = sqlx::query(
            "SELECT remote_addr, MIN(first_seen) as first_seen, MAX(last_seen) as last_seen
             FROM connection_sightings WHERE last_seen >= ? AND remote_addr != '' AND remote_addr NOT LIKE '127.%'
             GROUP BY remote_addr ORDER BY last_seen DESC LIMIT 200",
        )
        .bind(&since)
        .fetch_all(&self.pool)
        .await?;
        let rows = if rows.is_empty() {
            sqlx::query(
                "SELECT remote_addr, MIN(timestamp) as first_seen, MAX(timestamp) as last_seen
                 FROM connections WHERE timestamp >= ? AND remote_addr != '' AND remote_addr NOT LIKE '127.%'
                 GROUP BY remote_addr ORDER BY last_seen DESC LIMIT 200",
            )
            .bind(&since)
            .fetch_all(&self.pool)
            .await?
        } else {
            rows
        };
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get::<String, _>("remote_addr"),
                    r.get::<String, _>("first_seen"),
                    r.get::<String, _>("last_seen"),
                )
            })
            .collect())
    }

    pub async fn port_history(&self, port: u16) -> Result<Vec<serde_json::Value>, StorageError> {
        let rows = sqlx::query(
            "SELECT timestamp, event_type, protocol, address, pid, process_name, process_path
             FROM ports WHERE port = ? ORDER BY timestamp DESC LIMIT 100",
        )
        .bind(port as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "timestamp": r.get::<String, _>("timestamp"),
                    "event_type": r.get::<String, _>("event_type"),
                    "protocol": r.get::<String, _>("protocol"),
                    "address": r.get::<String, _>("address"),
                    "pid": r.get::<Option<i64>, _>("pid"),
                    "process_name": r.get::<Option<String>, _>("process_name"),
                    "process_path": r.get::<Option<String>, _>("process_path"),
                })
            })
            .collect())
    }

    pub async fn gpu_history(&self, hours: i64) -> Result<Vec<serde_json::Value>, StorageError> {
        let since = (Utc::now() - chrono::Duration::hours(hours)).to_rfc3339();
        let rows = sqlx::query(
            "SELECT timestamp, name, usage_percent, memory_used_bytes, processes_json
             FROM gpu_metrics WHERE timestamp >= ? ORDER BY timestamp ASC LIMIT 5000",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "timestamp": r.get::<String, _>("timestamp"),
                    "name": r.get::<Option<String>, _>("name"),
                    "usage_percent": r.get::<Option<f32>, _>("usage_percent"),
                    "memory_used_bytes": r.get::<Option<i64>, _>("memory_used_bytes"),
                    "processes_json": r.get::<Option<String>, _>("processes_json"),
                })
            })
            .collect())
    }

    pub async fn service_first_seen(&self, name: &str) -> Result<Option<String>, StorageError> {
        let row = sqlx::query("SELECT MIN(timestamp) as ts FROM services WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.and_then(|r| r.get::<Option<String>, _>("ts")))
    }

    pub async fn acknowledge_alert(&self, id: &str) -> Result<bool, StorageError> {
        let res = sqlx::query("UPDATE alerts SET acknowledged = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn change_summary(&self, hours: i64) -> Result<serde_json::Value, StorageError> {
        let since = (Utc::now() - chrono::Duration::hours(hours)).to_rfc3339();
        let ports: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ports WHERE timestamp >= ?")
            .bind(&since)
            .fetch_one(&self.pool)
            .await?;
        let events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM security_events WHERE timestamp >= ?")
                .bind(&since)
                .fetch_one(&self.pool)
                .await?;
        let services: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM services WHERE timestamp >= ?")
                .bind(&since)
                .fetch_one(&self.pool)
                .await?;
        let remotes: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT remote_addr) FROM connection_sightings WHERE last_seen >= ?",
        )
        .bind(&since)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        Ok(serde_json::json!({
            "since": since,
            "hours": hours,
            "port_transitions": ports,
            "security_events": events,
            "service_events": services,
            "distinct_remotes": remotes,
        }))
    }

    pub async fn correlate(&self, timestamp: &str) -> Result<serde_json::Value, StorageError> {
        let ts = DateTime::parse_from_rfc3339(timestamp)
            .map(|t| t.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let lo = (ts - chrono::Duration::minutes(10)).to_rfc3339();
        let hi = (ts + chrono::Duration::minutes(10)).to_rfc3339();
        let events = sqlx::query(
            "SELECT id, timestamp, rule, risk, process_name, process_pid, process_path, local_endpoint, remote_endpoint, indicators_json, details_json, first_seen
             FROM security_events WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp DESC LIMIT 40",
        )
        .bind(&lo)
        .bind(&hi)
        .fetch_all(&self.pool)
        .await?;
        let ports = sqlx::query(
            "SELECT timestamp, event_type, protocol, port, address, pid, process_name FROM ports
             WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp DESC LIMIT 40",
        )
        .bind(&lo)
        .bind(&hi)
        .fetch_all(&self.pool)
        .await?;
        let conns = sqlx::query(
            "SELECT timestamp, protocol, local_addr, local_port, remote_addr, remote_port, pid, process_name FROM connections
             WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp DESC LIMIT 40",
        )
        .bind(&lo)
        .bind(&hi)
        .fetch_all(&self.pool)
        .await?;
        Ok(serde_json::json!({
            "timestamp": timestamp,
            "events": events.iter().filter_map(row_to_event).collect::<Vec<_>>(),
            "ports": ports.iter().map(|r| serde_json::json!({
                "timestamp": r.get::<String, _>("timestamp"),
                "event_type": r.get::<String, _>("event_type"),
                "protocol": r.get::<String, _>("protocol"),
                "port": r.get::<i64, _>("port"),
                "address": r.get::<String, _>("address"),
                "pid": r.get::<Option<i64>, _>("pid"),
                "process_name": r.get::<Option<String>, _>("process_name"),
            })).collect::<Vec<_>>(),
            "connections": conns.iter().map(|r| serde_json::json!({
                "timestamp": r.get::<String, _>("timestamp"),
                "protocol": r.get::<String, _>("protocol"),
                "local": format!("{}:{}", r.get::<String, _>("local_addr"), r.get::<i64, _>("local_port")),
                "remote": format!("{}:{}", r.get::<String, _>("remote_addr"), r.get::<i64, _>("remote_port")),
                "pid": r.get::<Option<i64>, _>("pid"),
                "process_name": r.get::<Option<String>, _>("process_name"),
            })).collect::<Vec<_>>(),
        }))
    }

    pub async fn insert_dns_queries(
        &self,
        queries: &[crate::metrics::DnsQuery],
    ) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        for q in queries {
            sqlx::query(
                "INSERT INTO dns_queries (timestamp, query_name, record_type, response_code, process_name)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(q.timestamp.to_rfc3339())
            .bind(&q.query_name)
            .bind(&q.record_type)
            .bind(q.response_code as i64)
            .bind(&q.process_name)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn recent_dns_queries(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::metrics::DnsQuery>, StorageError> {
        let rows = sqlx::query(
            "SELECT timestamp, query_name, record_type, response_code, process_name
             FROM dns_queries ORDER BY id DESC LIMIT ?",
        )
        .bind(limit.clamp(1, 1000))
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::new();
        for r in rows {
            let ts: String = r.get("timestamp");
            if let Ok(dt) = DateTime::parse_from_rfc3339(&ts) {
                let rc: i64 = r.get("response_code");
                out.push(crate::metrics::DnsQuery {
                    timestamp: dt.with_timezone(&Utc),
                    query_name: r.get("query_name"),
                    record_type: r.get("record_type"),
                    response_code: rc as u16,
                    process_name: r.get("process_name"),
                });
            }
        }
        Ok(out)
    }

    pub async fn top_dns_domains(
        &self,
        hours: i64,
        limit: i64,
    ) -> Result<Vec<(String, i64)>, StorageError> {
        let since = (Utc::now() - chrono::Duration::hours(hours.max(1))).to_rfc3339();
        let rows = sqlx::query(
            "SELECT query_name, COUNT(*) as cnt
             FROM dns_queries WHERE timestamp >= ?
             GROUP BY query_name ORDER BY cnt DESC LIMIT ?",
        )
        .bind(since)
        .bind(limit.clamp(1, 100))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| (r.get("query_name"), r.get("cnt")))
            .collect())
    }

    pub async fn update_bandwidth(
        &self,
        month: &str,
        rx_delta: u64,
        tx_delta: u64,
    ) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO bandwidth_monthly (month, rx_bytes, tx_bytes, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(month) DO UPDATE SET
                rx_bytes = rx_bytes + excluded.rx_bytes,
                tx_bytes = tx_bytes + excluded.tx_bytes,
                updated_at = excluded.updated_at",
        )
        .bind(month)
        .bind(rx_delta as i64)
        .bind(tx_delta as i64)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_monthly_bandwidth(&self, month: &str) -> Result<(u64, u64), StorageError> {
        let row = sqlx::query("SELECT rx_bytes, tx_bytes FROM bandwidth_monthly WHERE month = ?")
            .bind(month)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            let rx: i64 = r.get("rx_bytes");
            let tx: i64 = r.get("tx_bytes");
            Ok((rx.max(0) as u64, tx.max(0) as u64))
        } else {
            Ok((0, 0))
        }
    }

    pub async fn get_connection_history(
        &self,
        remote_addr: &str,
    ) -> Result<Vec<Connection>, StorageError> {
        let rows = sqlx::query(
            "SELECT protocol, local_addr, local_port, remote_addr, remote_port, first_seen, last_seen, pid, process_name, process_path
             FROM connection_sightings WHERE remote_addr = ?
             ORDER BY last_seen DESC LIMIT 100",
        )
        .bind(remote_addr)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::new();
        for r in rows {
            let fs: String = r.get("first_seen");
            let ls: String = r.get("last_seen");
            let pid: Option<i64> = r.get("pid");
            let lp: i64 = r.get("local_port");
            let rp: i64 = r.get("remote_port");
            out.push(Connection {
                protocol: r.get("protocol"),
                local_addr: r.get("local_addr"),
                local_port: lp as u16,
                remote_addr: r.get("remote_addr"),
                remote_port: rp as u16,
                state: "OBSERVED".into(),
                pid: pid.map(|p| p as u32),
                process_name: r.get("process_name"),
                process_path: r.get("process_path"),
                first_seen: DateTime::parse_from_rfc3339(&fs)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc)),
                last_seen: DateTime::parse_from_rfc3339(&ls)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc)),
                geo: None,
            });
        }
        Ok(out)
    }

    /// Record user feedback on a finding (`false_positive`, `suppressed`,
    /// `confirmed`). The engine loads suppressing verdicts and stops
    /// re-emitting matching findings — this closes the feedback loop.
    pub async fn insert_feedback(
        &self,
        event_id: Option<&str>,
        rule: &str,
        key: &str,
        verdict: &str,
        note: Option<&str>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO feedback (timestamp, event_id, rule, key, verdict, note)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(event_id)
        .bind(rule)
        .bind(key)
        .bind(verdict)
        .bind(note)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Keys the engine should suppress, as `RULE|key` strings.
    pub async fn suppressed_keys(&self) -> Result<Vec<String>, StorageError> {
        let rows = sqlx::query(
            "SELECT rule, key FROM feedback WHERE verdict IN ('false_positive', 'suppressed')",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                format!(
                    "{}|{}",
                    r.get::<String, _>("rule"),
                    r.get::<String, _>("key")
                )
            })
            .collect())
    }
}

fn parse_risk(s: &str) -> RiskLevel {
    match s {
        "LOW" => RiskLevel::Low,
        "MEDIUM" => RiskLevel::Medium,
        "HIGH" => RiskLevel::High,
        "CRITICAL" => RiskLevel::Critical,
        _ => RiskLevel::Info,
    }
}

fn parse_rule(s: &str) -> Option<RuleId> {
    let rule = match s {
        "NEW_LISTENING_PORT" => RuleId::NewListeningPort,
        "NEW_EXTERNAL_CONNECTION" => RuleId::NewExternalConnection,
        "PORT_STATE_CHANGED" => RuleId::PortStateChanged,
        "UNKNOWN_PROCESS_NETWORK_ACCESS" => RuleId::UnknownProcessNetworkAccess,
        "MULTIPLE_FAILED_LOGINS" => RuleId::MultipleFailedLogins,
        "UNUSUAL_CONNECTION_RATE" => RuleId::UnusualConnectionRate,
        "NEW_SERVICE_STARTED" => RuleId::NewServiceStarted,
        "FIREWALL_CONFIGURATION_CHANGED" => RuleId::FirewallConfigurationChanged,
        "PROCESS_STARTED_UNUSUAL_PATH" => RuleId::ProcessStartedUnusualPath,
        "NETWORK_CONFIG_CHANGED" => RuleId::NetworkConfigChanged,
        "PORT_SCAN_LIKE_INBOUND" => RuleId::PortScanLikeInbound,
        "BLOCKLISTED_CONNECTION" => RuleId::BlocklistedConnection,
        "BANDWIDTH_QUOTA_WARNING" => RuleId::BandwidthQuotaWarning,
        "BANDWIDTH_QUOTA_EXCEEDED" => RuleId::BandwidthQuotaExceeded,
        "SUSPICIOUS_DNS_QUERY" => RuleId::SuspiciousDnsQuery,
        // Never mislabel history: unknown rule strings are skipped by the
        // caller with a warning instead of posing as another rule.
        _ => return None,
    };
    Some(rule)
}

fn row_to_event(r: &sqlx::sqlite::SqliteRow) -> Option<SecurityEvent> {
    let ts: String = r.get("timestamp");
    let rule: String = r.get("rule");
    let risk: String = r.get("risk");
    let indicators_json: String = r.get("indicators_json");
    let details_json: String = r.get("details_json");
    let process_name: Option<String> = r.get("process_name");
    let process_pid: Option<i64> = r.get("process_pid");
    let first_seen: i64 = r.get("first_seen");
    let mut ev = SecurityEvent {
        event_type: "SECURITY_EVENT".into(),
        id: r.get("id"),
        rule: parse_rule(&rule)?,
        risk: parse_risk(&risk),
        timestamp: DateTime::parse_from_rfc3339(&ts).ok()?.with_timezone(&Utc),
        process: process_name.map(|name| crate::events::ProcessRef {
            name,
            pid: process_pid.unwrap_or(0) as u32,
            path: r.get("process_path"),
        }),
        local: r.get("local_endpoint"),
        remote: r.get("remote_endpoint"),
        indicators: serde_json::from_str(&indicators_json).unwrap_or_default(),
        first_seen: first_seen != 0,
        details: serde_json::from_str(&details_json).unwrap_or(serde_json::Value::Null),
        assessment: String::new(),
    };
    ev.fill_assessment();
    Some(ev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{ProcessRef, RuleId};
    use crate::metrics::DeviceInfo;

    #[tokio::test]
    async fn sqlite_wal_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = Storage::connect(&dir.path().join("t.sqlite"))
            .await
            .unwrap();
        let d = DeviceInfo {
            id: "dev-test".into(),
            hostname: "host".into(),
            os_name: "test".into(),
            os_version: "1".into(),
            kernel: None,
            cpu_model: "cpu".into(),
            physical_cores: 1,
            logical_cores: 2,
            base_frequency_mhz: None,
            motherboard: None,
            bios_version: None,
            total_memory_bytes: 1024,
        };
        store.upsert_device(&d).await.unwrap();
        let ev = SecurityEvent {
            event_type: "SECURITY_EVENT".into(),
            id: "e1".into(),
            rule: RuleId::NewListeningPort,
            risk: RiskLevel::Medium,
            timestamp: Utc::now(),
            process: Some(ProcessRef {
                name: "node".into(),
                pid: 1,
                path: None,
            }),
            local: Some("0.0.0.0:3000".into()),
            remote: None,
            indicators: vec!["listening_port_appeared".into()],
            first_seen: true,
            details: serde_json::json!({}),
            assessment: String::new(),
        };
        store.insert_security_event(&ev).await.unwrap();
        let got = store.event_by_id("e1").await.unwrap().unwrap();
        assert_eq!(got.rule, RuleId::NewListeningPort);
        assert!(got.why_summary().to_lowercase().contains("suspicious"));
        assert!(!got.why_summary().to_lowercase().contains("under attack"));
    }

    fn sighting(remote: &str) -> Connection {
        Connection {
            protocol: "TCP".into(),
            local_addr: "192.168.1.8".into(),
            local_port: 50001,
            remote_addr: remote.into(),
            remote_port: 443,
            state: "ESTABLISHED".into(),
            pid: Some(42),
            process_name: Some("curl".into()),
            process_path: None,
            first_seen: None,
            last_seen: None,
            geo: None,
        }
    }

    #[tokio::test]
    async fn sightings_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = Storage::connect(&dir.path().join("t.sqlite"))
            .await
            .unwrap();
        let items = vec![sighting("93.184.216.34")];
        let n1 = store
            .upsert_connection_sightings(Utc::now(), &items)
            .await
            .unwrap();
        assert_eq!(n1, 1);
        let n2 = store
            .upsert_connection_sightings(Utc::now(), &items)
            .await
            .unwrap();
        assert_eq!(n2, 0, "second tick must report no new sightings");
        let hist = store.get_connection_history("93.184.216.34").await.unwrap();
        assert_eq!(hist.len(), 1, "history table holds one row per 5-tuple");
    }

    #[tokio::test]
    async fn prune_covers_event_tables() {
        let dir = tempfile::tempdir().unwrap();
        let store = Storage::connect(&dir.path().join("t.sqlite"))
            .await
            .unwrap();
        // Seed an old timeline row directly (bypasses the API clock).
        sqlx::query(
            "INSERT INTO timeline (id, timestamp, kind, risk, title, summary, event_id)
             VALUES ('old', '2001-01-01T00:00:00Z', 'TEST', 'INFO', 't', 's', NULL)",
        )
        .execute(&store.pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO dns_queries (timestamp, query_name, record_type, response_code, process_name)
             VALUES ('2001-01-01T00:00:00Z', 'old.example', 'A', 0, NULL)",
        )
        .execute(&store.pool)
        .await
        .unwrap();
        store.prune(7).await.unwrap();
        let timeline = store.timeline(10).await.unwrap();
        assert!(timeline.is_empty(), "prune must clear old timeline rows");
        let dns = store.recent_dns_queries(10).await.unwrap();
        assert!(dns.is_empty(), "prune must clear old dns rows");
    }

    #[tokio::test]
    async fn feedback_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = Storage::connect(&dir.path().join("t.sqlite"))
            .await
            .unwrap();
        store
            .insert_feedback(
                Some("e1"),
                "SUSPICIOUS_DNS_QUERY",
                "evil.tk",
                "false_positive",
                None,
            )
            .await
            .unwrap();
        store
            .insert_feedback(None, "BLOCKLISTED_CONNECTION", "*", "confirmed", None)
            .await
            .unwrap();
        let keys = store.suppressed_keys().await.unwrap();
        // Only suppressing verdicts are returned.
        assert_eq!(keys, vec!["SUSPICIOUS_DNS_QUERY|evil.tk"]);
    }
}
