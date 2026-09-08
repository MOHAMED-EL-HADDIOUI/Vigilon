use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }

    pub fn language(self) -> &'static str {
        match self {
            Self::Info => "Normal activity, logged for the timeline.",
            Self::Low | Self::Medium => "New or unusual activity — worth a look.",
            Self::High | Self::Critical => {
                "Multiple independent indicators align — investigate promptly."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleId {
    NewListeningPort,
    NewExternalConnection,
    PortStateChanged,
    UnknownProcessNetworkAccess,
    MultipleFailedLogins,
    UnusualConnectionRate,
    NewServiceStarted,
    FirewallConfigurationChanged,
    ProcessStartedUnusualPath,
    NetworkConfigChanged,
    PortScanLikeInbound,
    BlocklistedConnection,
    BandwidthQuotaWarning,
    BandwidthQuotaExceeded,
    SuspiciousDnsQuery,
}

impl RuleId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NewListeningPort => "NEW_LISTENING_PORT",
            Self::NewExternalConnection => "NEW_EXTERNAL_CONNECTION",
            Self::PortStateChanged => "PORT_STATE_CHANGED",
            Self::UnknownProcessNetworkAccess => "UNKNOWN_PROCESS_NETWORK_ACCESS",
            Self::MultipleFailedLogins => "MULTIPLE_FAILED_LOGINS",
            Self::UnusualConnectionRate => "UNUSUAL_CONNECTION_RATE",
            Self::NewServiceStarted => "NEW_SERVICE_STARTED",
            Self::FirewallConfigurationChanged => "FIREWALL_CONFIGURATION_CHANGED",
            Self::ProcessStartedUnusualPath => "PROCESS_STARTED_UNUSUAL_PATH",
            Self::NetworkConfigChanged => "NETWORK_CONFIG_CHANGED",
            Self::PortScanLikeInbound => "PORT_SCAN_LIKE_INBOUND",
            Self::BlocklistedConnection => "BLOCKLISTED_CONNECTION",
            Self::BandwidthQuotaWarning => "BANDWIDTH_QUOTA_WARNING",
            Self::BandwidthQuotaExceeded => "BANDWIDTH_QUOTA_EXCEEDED",
            Self::SuspiciousDnsQuery => "SUSPICIOUS_DNS_QUERY",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRef {
    pub name: String,
    pub pid: u32,
    pub path: Option<String>,
}

fn default_event_type() -> String {
    "SECURITY_EVENT".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    #[serde(rename = "type", default = "default_event_type")]
    pub event_type: String,
    pub id: String,
    pub rule: RuleId,
    pub risk: RiskLevel,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    pub indicators: Vec<String>,
    pub first_seen: bool,
    #[serde(default)]
    pub details: serde_json::Value,
    /// Human language for the risk level — never an "attack" claim.
    #[serde(default)]
    pub assessment: String,
}

impl SecurityEvent {
    pub fn fill_assessment(&mut self) {
        if self.event_type.is_empty() {
            self.event_type = default_event_type();
        }
        if self.assessment.is_empty() {
            self.assessment = format!("{} — {}", self.risk.as_str(), self.risk.language());
        }
    }

    pub fn why_summary(&self) -> String {
        format!(
            "Suspicious activity detected (heuristic). Assessment: {}. {}",
            self.risk.as_str(),
            self.risk.language()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "type")]
pub enum BusEvent {
    #[serde(rename = "METRICS")]
    Metrics(crate::metrics::Snapshot),
    #[serde(rename = "SECURITY_EVENT")]
    Security(SecurityEvent),
    #[serde(rename = "PORTS")]
    Ports {
        timestamp: DateTime<Utc>,
        items: Vec<crate::metrics::ListeningPort>,
    },
    #[serde(rename = "CONNECTIONS")]
    Connections {
        timestamp: DateTime<Utc>,
        items: Vec<crate::metrics::Connection>,
    },
    #[serde(rename = "PROCESSES")]
    Processes {
        timestamp: DateTime<Utc>,
        items: Vec<crate::metrics::ProcessInfo>,
    },
    #[serde(rename = "INTERFACES")]
    Interfaces {
        timestamp: DateTime<Utc>,
        items: Vec<crate::metrics::NetworkInterface>,
    },
    #[serde(rename = "SERVICES")]
    Services {
        timestamp: DateTime<Utc>,
        items: Vec<crate::metrics::ServiceInfo>,
    },
    #[serde(rename = "TIMELINE")]
    Timeline(TimelineEntry),
    #[serde(rename = "DNS_CACHE")]
    DnsCache {
        timestamp: DateTime<Utc>,
        items: Vec<crate::metrics::DnsCacheEntry>,
    },
    #[serde(rename = "PROCESS_BANDWIDTH")]
    ProcessBandwidth {
        timestamp: DateTime<Utc>,
        items: Vec<crate::metrics::ProcessBandwidth>,
    },
    #[serde(rename = "DNS_QUERIES")]
    DnsQueries {
        timestamp: DateTime<Utc>,
        items: Vec<crate::metrics::DnsQuery>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub kind: String,
    pub risk: RiskLevel,
    pub title: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub risk: RiskLevel,
    pub title: String,
    pub acknowledged: bool,
}
