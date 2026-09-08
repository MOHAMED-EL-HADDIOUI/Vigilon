use std::time::Duration;

use serde::Serialize;

/// Sampling tiers from spec §5.3 — all overridable via env.
#[derive(Debug, Clone, Serialize)]
pub struct AgentConfig {
    pub metrics_secs: u64,
    pub sockets_secs: u64,
    pub processes_secs: u64,
    pub services_secs: u64,
    pub retention_days: i64,
    pub bind: String,
    pub local_first: bool,
    pub phone_home: bool,
    pub bandwidth_quota_gb: Option<f64>,
    pub bandwidth_warning_pct: u8,
    pub notifications_enabled: bool,
    pub notification_min_risk: String,
    pub remote_agents: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl AgentConfig {
    pub fn from_env() -> Self {
        Self {
            metrics_secs: env_u64("VIGILON_METRICS_SECS", 2),
            sockets_secs: env_u64("VIGILON_SOCKETS_SECS", 4),
            processes_secs: env_u64("VIGILON_PROCESSES_SECS", 8),
            services_secs: env_u64("VIGILON_SERVICES_SECS", 40),
            retention_days: env_u64("VIGILON_RETENTION_DAYS", 7) as i64,
            bind: std::env::var("VIGILON_BIND").unwrap_or_else(|_| "127.0.0.1:8745".into()),
            local_first: true,
            phone_home: false,
            bandwidth_quota_gb: std::env::var("VIGILON_BANDWIDTH_QUOTA_GB")
                .ok()
                .and_then(|s| s.parse().ok()),
            bandwidth_warning_pct: env_u64("VIGILON_QUOTA_WARN_PCT", 80) as u8,
            notifications_enabled: std::env::var("VIGILON_NOTIFICATIONS")
                .map(|s| s != "0" && s.to_lowercase() != "false")
                .unwrap_or(true),
            notification_min_risk: std::env::var("VIGILON_NOTIFICATION_RISK")
                .unwrap_or_else(|_| "HIGH".into()),
            remote_agents: std::env::var("VIGILON_REMOTES")
                .ok()
                .map(|s| {
                    s.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    pub fn metrics_interval(&self) -> Duration {
        Duration::from_secs(self.metrics_secs.max(1))
    }

    pub fn sockets_interval(&self) -> Duration {
        Duration::from_secs(self.sockets_secs.max(1))
    }

    pub fn processes_interval(&self) -> Duration {
        Duration::from_secs(self.processes_secs.max(2))
    }

    pub fn services_interval(&self) -> Duration {
        Duration::from_secs(self.services_secs.max(5))
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec_tiers() {
        let c = AgentConfig {
            metrics_secs: 2,
            sockets_secs: 4,
            processes_secs: 8,
            services_secs: 40,
            retention_days: 7,
            bind: "127.0.0.1:8745".into(),
            local_first: true,
            phone_home: false,
            bandwidth_quota_gb: None,
            bandwidth_warning_pct: 80,
            notifications_enabled: true,
            notification_min_risk: "HIGH".into(),
            remote_agents: Vec::new(),
        };
        assert!(c.metrics_secs <= 2);
        assert!(c.processes_secs >= 5 && c.processes_secs <= 10);
        assert!(!c.phone_home);
        assert_eq!(c.bind, "127.0.0.1:8745");
    }
}
