//! Heuristic detection — explain, never accuse.

use crate::events::{RiskLevel, RuleId};

pub fn score(rule: &RuleId, indicator_count: usize, first_seen: bool) -> RiskLevel {
    match rule {
        RuleId::NewListeningPort => {
            if first_seen {
                RiskLevel::Medium
            } else {
                RiskLevel::Low
            }
        }
        RuleId::NewExternalConnection => {
            if indicator_count >= 3 {
                RiskLevel::Medium
            } else {
                RiskLevel::Low
            }
        }
        RuleId::UnknownProcessNetworkAccess => RiskLevel::High,
        RuleId::MultipleFailedLogins => {
            // `emit` scores on details.count (real failure count).
            if indicator_count >= 5 {
                RiskLevel::High
            } else {
                RiskLevel::Medium
            }
        }
        RuleId::UnusualConnectionRate | RuleId::PortScanLikeInbound => {
            // Scored on the real hit count; CRITICAL reserved for floods.
            if indicator_count >= 50 {
                RiskLevel::Critical
            } else {
                RiskLevel::High
            }
        }
        RuleId::FirewallConfigurationChanged | RuleId::NetworkConfigChanged => RiskLevel::Medium,
        RuleId::NewServiceStarted | RuleId::PortStateChanged => RiskLevel::Low,
        RuleId::ProcessStartedUnusualPath => RiskLevel::Medium,
        RuleId::BlocklistedConnection => RiskLevel::Critical,
        RuleId::BandwidthQuotaWarning | RuleId::SuspiciousDnsQuery => RiskLevel::Medium,
        RuleId::BandwidthQuotaExceeded => RiskLevel::High,
    }
}

pub fn phrasing(risk: RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Info => "Observed and logged.",
        RiskLevel::Low | RiskLevel::Medium => "Suspicious activity detected — worth a look.",
        RiskLevel::High | RiskLevel::Critical => {
            "Suspicious activity detected — investigate promptly. This is a heuristic, not a verdict."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_port_is_medium_when_first_seen() {
        assert_eq!(score(&RuleId::NewListeningPort, 1, true), RiskLevel::Medium);
    }

    #[test]
    fn unknown_process_is_high() {
        assert_eq!(
            score(&RuleId::UnknownProcessNetworkAccess, 1, true),
            RiskLevel::High
        );
    }

    #[test]
    fn rate_counts_drive_severity() {
        // Real hit counts flow through details.count: floods can reach
        // CRITICAL, ordinary bursts stay HIGH.
        assert_eq!(
            score(&RuleId::UnusualConnectionRate, 60, true),
            RiskLevel::Critical
        );
        assert_eq!(
            score(&RuleId::UnusualConnectionRate, 20, true),
            RiskLevel::High
        );
        assert_eq!(
            score(&RuleId::MultipleFailedLogins, 6, true),
            RiskLevel::High
        );
        assert_eq!(
            score(&RuleId::MultipleFailedLogins, 2, true),
            RiskLevel::Medium
        );
    }

    #[test]
    fn phrasing_never_says_attack() {
        for risk in [
            RiskLevel::Info,
            RiskLevel::Low,
            RiskLevel::Medium,
            RiskLevel::High,
            RiskLevel::Critical,
        ] {
            let t = phrasing(risk).to_lowercase();
            assert!(!t.contains("under attack"));
            assert!(!t.contains("you are compromised"));
        }
    }
}
