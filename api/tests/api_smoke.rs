use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use vigilon_agent::runtime::LiveState;
use vigilon_api::{AppState, router};
use vigilon_core::config::AgentConfig;
use vigilon_core::{EventBus, Storage};

async fn spawn_test_server() -> (SocketAddr, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db_path = tmp.path().join("test.sqlite");
    let storage = Storage::connect(&db_path).await.expect("storage");
    let bus = EventBus::new();
    let config = AgentConfig::default();
    let live = Arc::new(RwLock::new(LiveState::default()));

    let state = AppState {
        storage,
        bus,
        live,
        config,
        blocklist: vigilon_core::blocklist::BlocklistManager::new(tmp.path()),
        http_client: reqwest::Client::new(),
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let app = router(state, PathBuf::from("nonexistent_dist"));

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    (addr, tmp)
}

async fn http_post(addr: SocketAddr, path: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.expect("connect");
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path,
        addr,
        body.len(),
        body
    );
    stream.write_all(req.as_bytes()).await.expect("write");

    let mut res = Vec::new();
    stream.read_to_end(&mut res).await.expect("read");
    let response = String::from_utf8_lossy(&res).to_string();
    parse_response(&response)
}

fn parse_response(response: &str) -> (u16, String) {
    let first_line = response.lines().next().unwrap_or("");
    let status_code: u16 = first_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let body = response.split("\r\n\r\n").nth(1).unwrap_or("").to_string();

    (status_code, body)
}

async fn http_get(addr: SocketAddr, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.expect("connect");
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, addr
    );
    stream.write_all(req.as_bytes()).await.expect("write");

    let mut res = Vec::new();
    stream.read_to_end(&mut res).await.expect("read");
    let response = String::from_utf8_lossy(&res).to_string();

    parse_response(&response)
}

#[tokio::test]
async fn test_api_health_endpoint() {
    let (addr, _tmp) = spawn_test_server().await;
    let (status, body) = http_get(addr, "/api/health").await;
    assert_eq!(status, 200);
    assert!(body.contains(r#""ok":true"#));
    assert!(body.contains(r#""name":"vigilon""#));
}

#[tokio::test]
async fn test_api_config_endpoint() {
    let (addr, _tmp) = spawn_test_server().await;
    let (status, body) = http_get(addr, "/api/config").await;
    assert_eq!(status, 200);
    assert!(body.contains(r#""local_first":true"#));
    assert!(body.contains(r#""phone_home":false"#));
}

#[tokio::test]
async fn test_api_core_endpoints_return_ok() {
    let (addr, _tmp) = spawn_test_server().await;

    let endpoints = [
        "/api/ports",
        "/api/connections",
        "/api/processes",
        "/api/services",
        "/api/events",
        "/api/timeline",
        "/api/topology",
        "/api/interfaces",
        "/api/changes?hours=24",
    ];

    for ep in endpoints {
        let (status, body) = http_get(addr, ep).await;
        assert_eq!(status, 200, "endpoint {} failed with body: {}", ep, body);
        assert!(!body.is_empty(), "endpoint {} returned empty body", ep);
    }
}

#[tokio::test]
async fn test_fleet_proxy_rejects_unlisted_host() {
    // Default config has no remote_agents: any proxied host must be 403.
    let (addr, _tmp) = spawn_test_server().await;
    let (status, body) = http_get(addr, "/api/fleet/169.254.169.254/snapshot").await;
    assert_eq!(status, 403, "unlisted fleet host must be forbidden: {body}");
    assert!(body.contains("not a configured remote agent"));
}

#[tokio::test]
async fn test_feedback_validation() {
    let (addr, _tmp) = spawn_test_server().await;
    let (bad_status, _) = http_post(
        addr,
        "/api/feedback",
        r#"{"rule":"SUSPICIOUS_DNS_QUERY","key":"x.tk","verdict":"maybe"}"#,
    )
    .await;
    assert_eq!(bad_status, 400, "unknown verdict must be rejected");
    let (ok_status, ok_body) = http_post(
        addr,
        "/api/feedback",
        r#"{"rule":"SUSPICIOUS_DNS_QUERY","key":"x.tk","verdict":"false_positive"}"#,
    )
    .await;
    assert_eq!(ok_status, 200, "valid feedback must be accepted: {ok_body}");
    assert!(ok_body.contains(r#""ok":true"#));
}

#[tokio::test]
async fn test_blocklist_rejects_garbage() {
    let (addr, _tmp) = spawn_test_server().await;
    let (status, _) = http_post(addr, "/api/blocklist", r#"{"ip":"not an ip"}"#).await;
    assert_eq!(status, 422, "non-IP blocklist entries must be rejected");
    let (v6_status, _) = http_post(addr, "/api/blocklist", r#"{"ip":"2001:db8::1"}"#).await;
    assert_eq!(v6_status, 201, "IPv6 blocklist entries must be accepted");
}

#[tokio::test]
async fn test_list_endpoints_accept_limit() {
    let (addr, _tmp) = spawn_test_server().await;
    for ep in [
        "/api/events?limit=5",
        "/api/timeline?limit=5",
        "/api/alerts?limit=5",
    ] {
        let (status, _) = http_get(addr, ep).await;
        assert_eq!(status, 200, "endpoint {ep} must accept ?limit=");
    }
}
