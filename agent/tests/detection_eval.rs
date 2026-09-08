//! Offline detection eval harness (audit roadmap step 4).
//!
//! Scripted benign + malicious replays against `DetectionEngine`.
//! Each scenario asserts the *desired* contract: which rule must fire,
//! how often, and the maximum acceptable risk. Threshold changes in
//! `engine.rs` / `detection.rs` must keep this file green — update the
//! expectations below deliberately, never silently.
//!
//! The summary table prints with `cargo test -p vigilon-agent -- --nocapture`.
//! Future work: track precision/recall/FP-per-day across runs; feed the
//! `feedback` table verdicts back in as labeled data.

use chrono::Utc;
use vigilon_agent::engine::DetectionEngine;
use vigilon_core::events::{RiskLevel, RuleId, SecurityEvent};
use vigilon_core::metrics::{Connection, DnsQuery, ProcessInfo};
use vigilon_core::{BlocklistManager, EventBus, Storage};

fn rank(risk: RiskLevel) -> u8 {
    match risk {
        RiskLevel::Info => 0,
        RiskLevel::Low => 1,
        RiskLevel::Medium => 2,
        RiskLevel::High => 3,
        RiskLevel::Critical => 4,
    }
}

struct World {
    engine: DetectionEngine,
    bus: EventBus,
    storage: Storage,
    blocklist: BlocklistManager,
    // TempDir must stay alive for the SQLite file.
    _dir: tempfile::TempDir,
}

impl World {
    async fn fresh() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::connect(&dir.path().join("eval.sqlite"))
            .await
            .unwrap();
        Self {
            engine: DetectionEngine::new(),
            bus: EventBus::new(),
            storage,
            blocklist: BlocklistManager::new(dir.path()),
            _dir: dir,
        }
    }

    async fn new_events(&self, before: usize) -> Vec<SecurityEvent> {
        let mut all = self.storage.latest_events(500).await.unwrap();
        all.truncate(all.len().saturating_sub(before));
        all
    }

    async fn event_count(&self) -> usize {
        self.storage.latest_events(500).await.unwrap().len()
    }
}

fn conn(remote: &str, remote_port: u16, state: &str, proc: Option<&str>) -> Connection {
    conn_on_port(remote, remote_port, 51234, state, proc)
}

fn conn_on_port(
    remote: &str,
    remote_port: u16,
    local_port: u16,
    state: &str,
    proc: Option<&str>,
) -> Connection {
    Connection {
        protocol: "TCP".into(),
        local_addr: "192.168.1.8".into(),
        local_port,
        remote_addr: remote.into(),
        remote_port,
        state: state.into(),
        pid: Some(4242),
        process_name: proc.map(str::to_string),
        process_path: proc.map(|p| format!("/usr/bin/{p}")),
        first_seen: None,
        last_seen: None,
        geo: None,
    }
}

fn proc(name: &str, path: Option<&str>) -> ProcessInfo {
    ProcessInfo {
        pid: 4242,
        name: name.into(),
        path: path.map(str::to_string),
        cpu_percent: 1.0,
        memory_bytes: 1024,
        status: "running".into(),
        start_time: None,
        connection_count: 0,
        parent_pid: None,
    }
}

fn dns_query(name: &str) -> DnsQuery {
    DnsQuery {
        timestamp: Utc::now(),
        query_name: name.into(),
        record_type: "A".into(),
        response_code: 0,
        process_name: None,
    }
}

struct Outcome {
    scenario: &'static str,
    kind: &'static str,
    expected: &'static str,
    got_rules: Vec<String>,
    pass: bool,
}

#[tokio::test]
async fn detection_eval_suite() {
    let mut report: Vec<Outcome> = Vec::new();

    // ── Benign 1: browser burst (19 parallel conns, just under the rate rule)
    {
        let mut w = World::fresh().await;
        let before = w.event_count().await;
        let conns: Vec<Connection> = (0..19)
            .map(|_| conn("151.101.1.1", 443, "ESTABLISHED", Some("browser")))
            .collect();
        w.engine
            .observe_connections(&w.bus, &w.storage, &conns, &[], &w.blocklist)
            .await;
        let got = w.new_events(before).await;
        let rules: Vec<String> = got.iter().map(|e| e.rule.as_str().to_string()).collect();
        let pass = got.len() == 1
            && got[0].rule == RuleId::NewExternalConnection
            && !rules.iter().any(|r| r == "UNUSUAL_CONNECTION_RATE");
        report.push(Outcome {
            scenario: "benign_browser_burst_19",
            kind: "benign",
            expected: "exactly 1 NEW_EXTERNAL_CONNECTION, no rate finding",
            got_rules: rules,
            pass,
        });
    }

    // ── Benign 2: ordinary DNS stays silent
    {
        let w = World::fresh().await;
        let before = w.event_count().await;
        let qs = vec![dns_query("example.com"), dns_query("intranet.local")];
        // Immutable borrow gymnastics: observe needs &mut engine.
        let mut w = w;
        w.engine
            .observe_dns_queries(&w.bus, &w.storage, &qs, &w.blocklist)
            .await;
        let got = w.new_events(before).await;
        report.push(Outcome {
            scenario: "benign_dns",
            kind: "benign",
            expected: "0 events",
            got_rules: got.iter().map(|e| e.rule.as_str().to_string()).collect(),
            pass: got.is_empty(),
        });
    }

    // ── Benign 3 (documented): installer from Downloads fires, capped at MEDIUM
    {
        let mut w = World::fresh().await;
        w.engine
            .observe_processes(&w.bus, &w.storage, &[proc("app", Some("/usr/bin/app"))])
            .await;
        let before = w.event_count().await;
        w.engine
            .observe_processes(
                &w.bus,
                &w.storage,
                &[proc("setup", Some("C:\\Users\\a\\Downloads\\setup.exe"))],
            )
            .await;
        let got = w.new_events(before).await;
        let pass = got.len() == 1
            && got[0].rule == RuleId::ProcessStartedUnusualPath
            && rank(got[0].risk) <= rank(RiskLevel::Medium);
        report.push(Outcome {
            scenario: "benign_installer_downloads",
            kind: "benign(documented)",
            expected: "exactly 1 PROCESS_STARTED_UNUSUAL_PATH, risk <= MEDIUM",
            got_rules: got.iter().map(|e| e.rule.as_str().to_string()).collect(),
            pass,
        });
    }

    // ── Malicious 1: inbound scan (15 SYNs against distinct local ports)
    {
        let mut w = World::fresh().await;
        let before = w.event_count().await;
        // A WAN scanner probing our ports: distinct LOCAL ports, one remote
        // source port, unattributed (no owning process on a half-open SYN).
        let probed = [
            22, 80, 443, 3000, 3306, 5432, 6379, 8080, 8443, 9000, 27017, 11211, 5900, 21, 25,
        ];
        let conns: Vec<Connection> = probed
            .iter()
            .map(|p| conn_on_port("203.0.113.9", 51234, *p, "SYN_RECEIVED", None))
            .collect();
        w.engine
            .observe_connections(&w.bus, &w.storage, &conns, &[], &w.blocklist)
            .await;
        let got = w.new_events(before).await;
        let scans: Vec<&SecurityEvent> = got
            .iter()
            .filter(|e| e.rule == RuleId::PortScanLikeInbound)
            .collect();
        let pass = scans.len() == 1 && scans[0].risk == RiskLevel::High;
        report.push(Outcome {
            scenario: "malicious_port_scan",
            kind: "malicious",
            expected: "exactly 1 PORT_SCAN_LIKE_INBOUND, HIGH",
            got_rules: got.iter().map(|e| e.rule.as_str().to_string()).collect(),
            pass,
        });
    }

    // ── Malicious 2: beaconing (25 conns to one dest in a tick)
    {
        let mut w = World::fresh().await;
        let before = w.event_count().await;
        let conns: Vec<Connection> = (0..25)
            .map(|_| conn("198.51.100.7", 443, "ESTABLISHED", Some("agent")))
            .collect();
        w.engine
            .observe_connections(&w.bus, &w.storage, &conns, &[], &w.blocklist)
            .await;
        let got = w.new_events(before).await;
        let rate = got
            .iter()
            .filter(|e| e.rule == RuleId::UnusualConnectionRate)
            .count();
        report.push(Outcome {
            scenario: "malicious_beacon_25",
            kind: "malicious",
            expected: "exactly 1 UNUSUAL_CONNECTION_RATE",
            got_rules: got.iter().map(|e| e.rule.as_str().to_string()).collect(),
            pass: rate == 1,
        });
    }

    // ── Malicious 3: blocklisted IP, sustained across ticks → single CRITICAL
    {
        let mut w = World::fresh().await;
        w.blocklist.add("203.0.113.66".into(), None).await.unwrap();
        let before = w.event_count().await;
        for _ in 0..3 {
            w.engine
                .observe_connections(
                    &w.bus,
                    &w.storage,
                    &[conn("203.0.113.66", 443, "ESTABLISHED", Some("agent"))],
                    &[],
                    &w.blocklist,
                )
                .await;
        }
        let got = w.new_events(before).await;
        let blocked: Vec<&SecurityEvent> = got
            .iter()
            .filter(|e| e.rule == RuleId::BlocklistedConnection)
            .collect();
        let pass = blocked.len() == 1 && blocked[0].risk == RiskLevel::Critical;
        report.push(Outcome {
            scenario: "malicious_blocklisted_ip",
            kind: "malicious",
            expected: "exactly 1 BLOCKLISTED_CONNECTION, CRITICAL",
            got_rules: got.iter().map(|e| e.rule.as_str().to_string()).collect(),
            pass,
        });
    }

    // ── Malicious 4: shady TLD lookup
    {
        let mut w = World::fresh().await;
        let before = w.event_count().await;
        w.engine
            .observe_dns_queries(&w.bus, &w.storage, &[dns_query("payload.tk")], &w.blocklist)
            .await;
        let got = w.new_events(before).await;
        let pass = got.len() == 1 && got[0].rule == RuleId::SuspiciousDnsQuery;
        report.push(Outcome {
            scenario: "malicious_dns_tk",
            kind: "malicious",
            expected: "exactly 1 SUSPICIOUS_DNS_QUERY",
            got_rules: got.iter().map(|e| e.rule.as_str().to_string()).collect(),
            pass,
        });
    }

    // ── Malicious 5: quota exceeded, sustained → single HIGH
    {
        let mut w = World::fresh().await;
        let before = w.event_count().await;
        for _ in 0..3 {
            w.engine
                .observe_bandwidth_quota(&w.bus, &w.storage, Some(1.0), 80, 1_200_000_000)
                .await;
        }
        let got = w.new_events(before).await;
        let pass = got.len() == 1 && got[0].rule == RuleId::BandwidthQuotaExceeded;
        report.push(Outcome {
            scenario: "quota_exceeded",
            kind: "policy",
            expected: "exactly 1 BANDWIDTH_QUOTA_EXCEEDED",
            got_rules: got.iter().map(|e| e.rule.as_str().to_string()).collect(),
            pass,
        });
    }

    // ── Auth failures: real counts drive severity
    {
        let mut w = World::fresh().await;
        let before = w.event_count().await;
        w.engine.observe_auth_failures(&w.bus, &w.storage, 6).await;
        let got = w.new_events(before).await;
        let pass = got.len() == 1
            && got[0].rule == RuleId::MultipleFailedLogins
            && got[0].risk == RiskLevel::High;
        report.push(Outcome {
            scenario: "auth_failures_6",
            kind: "malicious",
            expected: "exactly 1 MULTIPLE_FAILED_LOGINS, HIGH",
            got_rules: got.iter().map(|e| e.rule.as_str().to_string()).collect(),
            pass,
        });
    }

    // ── Reboot resilience: persisted baselines don't re-fire
    {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::connect(&dir.path().join("eval.sqlite"))
            .await
            .unwrap();
        let bus = EventBus::new();
        let blocklist = BlocklistManager::new(dir.path());
        let mut e1 = DetectionEngine::new();
        e1.observe_connections(
            &bus,
            &storage,
            &[conn("93.184.216.34", 443, "ESTABLISHED", Some("svc"))],
            &[],
            &blocklist,
        )
        .await;
        e1.persist_baselines(&storage).await;
        let before = storage.latest_events(500).await.unwrap().len();
        let mut e2 = DetectionEngine::new();
        e2.load_baselines(&storage).await;
        e2.observe_connections(
            &bus,
            &storage,
            &[conn("93.184.216.34", 443, "ESTABLISHED", Some("svc"))],
            &[],
            &blocklist,
        )
        .await;
        let after = storage.latest_events(500).await.unwrap().len();
        report.push(Outcome {
            scenario: "reboot_no_refire",
            kind: "robustness",
            expected: "0 new events after reload",
            got_rules: vec![format!("delta={}", after.saturating_sub(before))],
            pass: after == before,
        });
    }

    // ── Report + gate
    println!("\n=== Vigilon detection eval ===");
    println!("{:<32} {:<18} {:<8} rules", "scenario", "kind", "result");
    let mut fails = 0;
    for o in &report {
        println!(
            "{:<32} {:<18} {:<8} {:?}  (want: {})",
            o.scenario,
            o.kind,
            if o.pass { "PASS" } else { "FAIL" },
            o.got_rules,
            o.expected
        );
        if !o.pass {
            fails += 1;
        }
    }
    let malicious_total = report.iter().filter(|o| o.kind == "malicious").count();
    let malicious_hit = report
        .iter()
        .filter(|o| o.kind == "malicious" && o.pass)
        .count();
    println!("malicious recall: {malicious_hit}/{malicious_total}");
    assert_eq!(
        fails, 0,
        "{fails} eval scenario(s) violated the detection contract"
    );
}
