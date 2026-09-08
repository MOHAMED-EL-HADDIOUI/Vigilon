use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::{HeaderValue, Method};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use vigilon_agent::runtime::LiveState;
use vigilon_core::blocklist::{BlocklistEntry, BlocklistManager};
use vigilon_core::config::AgentConfig;
use vigilon_core::metrics::{TopologyEdge, TopologyGraph, TopologyNode};
use vigilon_core::{EventBus, Storage};

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub bus: EventBus,
    pub live: Arc<RwLock<LiveState>>,
    pub config: AgentConfig,
    pub blocklist: BlocklistManager,
    pub http_client: reqwest::Client,
}

pub fn router(state: AppState, frontend: PathBuf) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://127.0.0.1:5173"),
            HeaderValue::from_static("http://localhost:8745"),
            HeaderValue::from_static("http://127.0.0.1:8745"),
        ]))
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        // Explicit allowlist instead of `Any`: the dashboard only ever
        // sends JSON + standard fetch headers. Reflecting arbitrary
        // headers widens the trust surface for allowed origins.
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::AUTHORIZATION,
        ]);

    let api = Router::new()
        .route("/health", get(health))
        .route("/config", get(config))
        .route("/metrics/latest", get(metrics_latest))
        .route("/metrics/history", get(metrics_history))
        .route("/metrics/gpu", get(gpu_history))
        .route("/device", get(device))
        .route("/processes", get(processes))
        .route("/ports", get(ports))
        .route("/ports/history", get(port_history))
        .route("/connections", get(connections))
        .route("/interfaces", get(interfaces))
        .route("/services", get(services))
        .route("/services/first-seen", get(service_first))
        .route("/events", get(events))
        .route("/events/{id}", get(event_by_id))
        .route("/alerts", get(alerts))
        .route("/alerts/{id}/ack", post(ack_alert))
        .route("/feedback", post(submit_feedback))
        .route("/timeline", get(timeline))
        .route("/topology", get(topology))
        .route("/remotes", get(remotes))
        .route("/changes", get(changes))
        .route("/dns", get(dns))
        .route("/correlate", get(correlate))
        .route("/export/events", get(export_events))
        .route("/export/connections", get(export_connections))
        .route("/export/metrics", get(export_metrics))
        .route("/export/ports", get(export_ports))
        .route("/blocklist", get(get_blocklist).post(add_blocklist))
        .route("/blocklist/{ip}", delete(delete_blocklist))
        .route("/bandwidth", get(get_bandwidth))
        .route("/dns/queries", get(get_dns_queries))
        .route("/dns/top", get(get_top_dns))
        .route("/process-bandwidth", get(get_process_bandwidth))
        .route(
            "/connections/{remote_addr}/history",
            get(connection_history),
        )
        .route("/fleet/agents", get(fleet_agents))
        .route("/fleet/{host}/snapshot", get(fleet_snapshot))
        .route("/fleet/{host}/health", get(fleet_health))
        .route("/ws", get(ws_handler));

    Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(frontend))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "name": "vigilon",
        "local_first": true,
        "phone_home": false
    }))
}

async fn config(State(s): State<AppState>) -> Json<AgentConfig> {
    Json(s.config.clone())
}

async fn metrics_latest(State(s): State<AppState>) -> impl IntoResponse {
    let live = s.live.read().await;
    Json(live.snapshot.clone())
}

#[derive(Deserialize)]
struct Hours {
    #[serde(default = "default_hours")]
    hours: i64,
}

/// Uniform JSON error envelope. Internal details go to the server log;
/// clients get a stable shape with a request id for correlation.
struct ApiError {
    status: axum::http::StatusCode,
    message: String,
    request_id: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": self.message,
                "request_id": self.request_id,
            })),
        )
            .into_response()
    }
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    let request_id = uuid::Uuid::new_v4().to_string();
    tracing::error!(%request_id, error = %e, "api handler failed");
    ApiError {
        status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        message: "internal error".into(),
        request_id,
    }
}

/// Shared `?limit=` pagination guard: defaults per endpoint, clamped 1..=1000.
#[derive(Deserialize)]
struct Page {
    limit: Option<i64>,
}

impl Page {
    fn limit(&self, default: i64) -> i64 {
        self.limit.unwrap_or(default).clamp(1, 1000)
    }
}

fn default_hours() -> i64 {
    24
}

async fn metrics_history(State(s): State<AppState>, Query(q): Query<Hours>) -> impl IntoResponse {
    match s.storage.history(q.hours.clamp(1, 168)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn device(State(s): State<AppState>) -> impl IntoResponse {
    let live = s.live.read().await;
    Json(live.snapshot.as_ref().map(|x| x.device.clone()))
}

async fn processes(State(s): State<AppState>) -> Json<Vec<vigilon_core::metrics::ProcessInfo>> {
    Json(s.live.read().await.processes.clone())
}

async fn ports(State(s): State<AppState>) -> Json<Vec<vigilon_core::metrics::ListeningPort>> {
    Json(s.live.read().await.ports.clone())
}

async fn connections(State(s): State<AppState>) -> Json<Vec<vigilon_core::metrics::Connection>> {
    Json(s.live.read().await.connections.clone())
}

async fn interfaces(
    State(s): State<AppState>,
) -> Json<Vec<vigilon_core::metrics::NetworkInterface>> {
    Json(s.live.read().await.interfaces.clone())
}

async fn services(State(s): State<AppState>) -> Json<Vec<vigilon_core::metrics::ServiceInfo>> {
    Json(s.live.read().await.services.clone())
}

async fn events(State(s): State<AppState>, Query(q): Query<Page>) -> impl IntoResponse {
    match s.storage.latest_events(q.limit(200)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn event_by_id(State(s): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match s.storage.event_by_id(&id).await {
        Ok(Some(v)) => Json(v).into_response(),
        Ok(None) => ApiError {
            status: axum::http::StatusCode::NOT_FOUND,
            message: "event not found".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
        .into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn alerts(State(s): State<AppState>, Query(q): Query<Page>) -> impl IntoResponse {
    match s.storage.alerts(q.limit(100)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

#[derive(Deserialize)]
struct FeedbackPayload {
    event_id: Option<String>,
    rule: String,
    #[serde(default)]
    key: String,
    verdict: String,
    note: Option<String>,
}

async fn submit_feedback(
    State(s): State<AppState>,
    Json(p): Json<FeedbackPayload>,
) -> impl IntoResponse {
    const ALLOWED: &[&str] = &["false_positive", "suppressed", "confirmed"];
    if !ALLOWED.contains(&p.verdict.as_str()) {
        return ApiError {
            status: axum::http::StatusCode::BAD_REQUEST,
            message: "verdict must be false_positive, suppressed, or confirmed".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
        .into_response();
    }
    if p.rule.trim().is_empty() || p.rule.len() > 64 || p.key.len() > 512 {
        return ApiError {
            status: axum::http::StatusCode::BAD_REQUEST,
            message: "invalid rule or key".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
        .into_response();
    }
    match s
        .storage
        .insert_feedback(
            p.event_id.as_deref(),
            p.rule.trim(),
            p.key.trim(),
            &p.verdict,
            p.note.as_deref(),
        )
        .await
    {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn timeline(State(s): State<AppState>, Query(q): Query<Page>) -> impl IntoResponse {
    match s.storage.timeline(q.limit(300)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn remotes(State(s): State<AppState>) -> impl IntoResponse {
    match s.storage.remote_ips_since(24).await {
        Ok(rows) => Json(
            rows.into_iter()
                .map(|(ip, first, last)| serde_json::json!({ "ip": ip, "first_seen": first, "last_seen": last }))
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn dns(State(s): State<AppState>) -> Json<Vec<vigilon_core::metrics::DnsCacheEntry>> {
    Json(s.live.read().await.dns.clone())
}

#[derive(Deserialize)]
struct PortQ {
    port: u16,
}

async fn port_history(State(s): State<AppState>, Query(q): Query<PortQ>) -> impl IntoResponse {
    match s.storage.port_history(q.port).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn gpu_history(State(s): State<AppState>, Query(q): Query<Hours>) -> impl IntoResponse {
    match s.storage.gpu_history(q.hours.clamp(1, 168)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

#[derive(Deserialize)]
struct NameQ {
    name: String,
}

async fn service_first(State(s): State<AppState>, Query(q): Query<NameQ>) -> impl IntoResponse {
    match s.storage.service_first_seen(&q.name).await {
        Ok(v) => Json(serde_json::json!({ "name": q.name, "first_seen": v })).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn changes(State(s): State<AppState>, Query(q): Query<Hours>) -> impl IntoResponse {
    match s.storage.change_summary(q.hours.clamp(1, 168)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn ack_alert(State(s): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match s.storage.acknowledge_alert(&id).await {
        Ok(true) => Json(serde_json::json!({ "ok": true, "id": id })).into_response(),
        Ok(false) => ApiError {
            status: axum::http::StatusCode::NOT_FOUND,
            message: "alert not found".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
        .into_response(),
        Err(e) => internal(e).into_response(),
    }
}

#[derive(Deserialize)]
struct TsQ {
    timestamp: String,
}

async fn correlate(State(s): State<AppState>, Query(q): Query<TsQ>) -> impl IntoResponse {
    match s.storage.correlate(&q.timestamp).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn topology(State(s): State<AppState>) -> Json<TopologyGraph> {
    let live = s.live.read().await;
    Json(build_topology(&live))
}

pub fn build_topology(live: &LiveState) -> TopologyGraph {
    let mut nodes = vec![
        TopologyNode {
            id: "internet".into(),
            kind: "internet".into(),
            label: "INTERNET".into(),
            subtitle: None,
        },
        TopologyNode {
            id: "router".into(),
            kind: "router".into(),
            label: "Router".into(),
            subtitle: live.interfaces.iter().find_map(|i| i.gateway.clone()),
        },
    ];
    let ip = live
        .interfaces
        .iter()
        .find(|i| i.kind != "loopback" && !i.ips.is_empty())
        .and_then(|i| i.ips.first().cloned())
        .unwrap_or_else(|| "local".into());
    let host = live
        .snapshot
        .as_ref()
        .map(|s| s.device.hostname.clone())
        .unwrap_or_else(|| "MY PC".into());
    nodes.push(TopologyNode {
        id: "pc".into(),
        kind: "host".into(),
        label: host,
        subtitle: Some(ip),
    });
    let mut edges = vec![
        TopologyEdge {
            id: "e-int-r".into(),
            source: "internet".into(),
            target: "router".into(),
            animated: true,
            label: None,
        },
        TopologyEdge {
            id: "e-r-pc".into(),
            source: "router".into(),
            target: "pc".into(),
            animated: true,
            label: None,
        },
    ];
    for p in live.ports.iter().filter(|p| p.protocol == "TCP").take(18) {
        let id = format!("port-{}-{}", p.protocol, p.port);
        nodes.push(TopologyNode {
            id: id.clone(),
            kind: "port".into(),
            label: format!(":{}", p.port),
            subtitle: p.process_name.clone(),
        });
        edges.push(TopologyEdge {
            id: format!("e-pc-{id}"),
            source: "pc".into(),
            target: id,
            animated: false,
            label: Some(p.protocol.clone()),
        });
    }
    let ext = live
        .connections
        .iter()
        .filter(|c| vigilon_core::netparse::is_external_addr(&c.remote_addr))
        .take(12);
    for (i, c) in ext.enumerate() {
        let id = format!("ext-{i}");
        nodes.push(TopologyNode {
            id: id.clone(),
            kind: "remote".into(),
            label: format!("{}:{}", c.remote_addr, c.remote_port),
            subtitle: c.process_name.clone(),
        });
        edges.push(TopologyEdge {
            id: format!("e-pc-{id}"),
            source: "pc".into(),
            target: id,
            animated: true,
            label: c.process_name.clone(),
        });
    }
    TopologyGraph { nodes, edges }
}

async fn ws_handler(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, s))
}

async fn handle_socket(mut socket: WebSocket, s: AppState) {
    let mut rx = s.bus.subscribe();
    {
        let live = s.live.read().await;
        if let Some(snap) = &live.snapshot {
            let msg = vigilon_core::events::BusEvent::Metrics(snap.clone());
            if let Ok(txt) = serde_json::to_string(&msg) {
                let _ = socket.send(Message::Text(txt.into())).await;
            }
        }
    }
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(p))) => {
                        let _ = socket.send(Message::Pong(p)).await;
                    }
                    _ => {}
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        if let Ok(txt) = serde_json::to_string(&ev)
                            && socket.send(Message::Text(txt.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::debug!(skipped = n, "ws client lagged; skipping missed events");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct ExportParams {
    format: Option<String>,
    hours: Option<i64>,
}

fn export_response<T: serde::Serialize>(
    records: &[T],
    filename: &str,
    format: Option<&str>,
) -> axum::response::Response {
    let fmt = format.unwrap_or("json").to_lowercase();
    if fmt == "csv" {
        match crate::export::to_csv(records) {
            Ok(csv) => {
                let mut resp = axum::response::Response::new(axum::body::Body::from(csv));
                resp.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    HeaderValue::from_static("text/csv; charset=utf-8"),
                );
                let cd = format!("attachment; filename=\"{}.csv\"", filename);
                if let Ok(hv) = HeaderValue::from_str(&cd) {
                    resp.headers_mut()
                        .insert(axum::http::header::CONTENT_DISPOSITION, hv);
                }
                resp
            }
            Err(e) => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
        }
    } else {
        match serde_json::to_string_pretty(records) {
            Ok(json) => {
                let mut resp = axum::response::Response::new(axum::body::Body::from(json));
                resp.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json; charset=utf-8"),
                );
                let cd = format!("attachment; filename=\"{}.json\"", filename);
                if let Ok(hv) = HeaderValue::from_str(&cd) {
                    resp.headers_mut()
                        .insert(axum::http::header::CONTENT_DISPOSITION, hv);
                }
                resp
            }
            Err(e) => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
        }
    }
}

async fn export_events(
    State(s): State<AppState>,
    Query(q): Query<ExportParams>,
) -> impl IntoResponse {
    let items = s.storage.latest_events(500).await.unwrap_or_default();
    export_response(&items, "vigilon-security-events", q.format.as_deref())
}

async fn export_connections(
    State(s): State<AppState>,
    Query(q): Query<ExportParams>,
) -> impl IntoResponse {
    let items = s.live.read().await.connections.clone();
    export_response(&items, "vigilon-connections", q.format.as_deref())
}

async fn export_metrics(
    State(s): State<AppState>,
    Query(q): Query<ExportParams>,
) -> impl IntoResponse {
    let items = s
        .storage
        .history(q.hours.unwrap_or(24).clamp(1, 168))
        .await
        .unwrap_or_default();
    export_response(&items, "vigilon-metrics-history", q.format.as_deref())
}

async fn export_ports(
    State(s): State<AppState>,
    Query(q): Query<ExportParams>,
) -> impl IntoResponse {
    let items = s.live.read().await.ports.clone();
    export_response(&items, "vigilon-listening-ports", q.format.as_deref())
}

#[derive(Deserialize)]
struct AddBlocklistPayload {
    ip: String,
    comment: Option<String>,
}

async fn get_blocklist(State(s): State<AppState>) -> Json<Vec<BlocklistEntry>> {
    Json(s.blocklist.list().await)
}

async fn add_blocklist(
    State(s): State<AppState>,
    Json(payload): Json<AddBlocklistPayload>,
) -> impl IntoResponse {
    match s.blocklist.add(payload.ip, payload.comment).await {
        Ok(_) => axum::http::StatusCode::CREATED.into_response(),
        Err(vigilon_core::blocklist::BlocklistError::InvalidIp(what)) => (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": format!("invalid IP address: {what}") })),
        )
            .into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn delete_blocklist(State(s): State<AppState>, Path(ip): Path<String>) -> impl IntoResponse {
    match s.blocklist.remove(&ip).await {
        Ok(true) => axum::http::StatusCode::OK.into_response(),
        Ok(false) => ApiError {
            status: axum::http::StatusCode::NOT_FOUND,
            message: "blocklist entry not found".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
        .into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn get_bandwidth(State(s): State<AppState>) -> impl IntoResponse {
    let month = chrono::Utc::now().format("%Y-%m").to_string();
    let (used_rx_bytes, used_tx_bytes) = s
        .storage
        .get_monthly_bandwidth(&month)
        .await
        .unwrap_or((0, 0));
    Json(vigilon_core::metrics::BandwidthQuotaInfo {
        quota_gb: s.config.bandwidth_quota_gb,
        used_rx_bytes,
        used_tx_bytes,
        month,
        warning_pct: s.config.bandwidth_warning_pct,
    })
}

#[derive(Deserialize)]
struct DnsLimit {
    limit: Option<i64>,
}

async fn get_dns_queries(
    State(s): State<AppState>,
    Query(q): Query<DnsLimit>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100);
    match s.storage.recent_dns_queries(limit).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

#[derive(Deserialize)]
struct TopDnsQuery {
    hours: Option<i64>,
    limit: Option<i64>,
}

async fn get_top_dns(State(s): State<AppState>, Query(q): Query<TopDnsQuery>) -> impl IntoResponse {
    let hours = q.hours.unwrap_or(24);
    let limit = q.limit.unwrap_or(20);
    match s.storage.top_dns_domains(hours, limit).await {
        Ok(v) => Json(
            v.into_iter()
                .map(|(d, c)| serde_json::json!({ "domain": d, "count": c }))
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn get_process_bandwidth(State(s): State<AppState>) -> impl IntoResponse {
    Json(s.live.read().await.process_bandwidth.clone())
}

async fn connection_history(
    State(s): State<AppState>,
    Path(remote_addr): Path<String>,
) -> impl IntoResponse {
    let clean = vigilon_core::netparse::strip_port(&remote_addr);
    if clean.parse::<std::net::IpAddr>().is_err() {
        return ApiError {
            status: axum::http::StatusCode::BAD_REQUEST,
            message: "invalid IP address".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
        .into_response();
    }
    match s.storage.get_connection_history(clean).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => internal(e).into_response(),
    }
}

async fn fleet_agents(State(s): State<AppState>) -> impl IntoResponse {
    let mut out = Vec::new();
    for addr in &s.config.remote_agents {
        let url = format!("http://{}/api/health", addr);
        let status = match s
            .http_client
            .get(&url)
            .timeout(std::time::Duration::from_millis(1500))
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => "online",
            _ => "offline",
        };
        out.push(serde_json::json!({
            "address": addr,
            "status": status,
        }));
    }
    Json(out)
}

async fn fleet_snapshot(State(s): State<AppState>, Path(host): Path<String>) -> impl IntoResponse {
    if let Err(e) = check_fleet_host(&s.config, &host) {
        return e.into_response();
    }
    let url = format!("http://{}/api/metrics/latest", host);
    match s
        .http_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            if let Ok(v) = res.json::<serde_json::Value>().await {
                Json(v).into_response()
            } else {
                bad_gateway("invalid JSON from remote agent")
            }
        }
        _ => bad_gateway("failed to reach remote agent"),
    }
}

/// SSRF guard: only proxy to hosts explicitly listed in `remote_agents`,
/// and reject anything that could smuggle credentials, paths, or queries
/// into the outbound URL (e.g. `evil@`, `/`, `?`, `#`, whitespace).
fn check_fleet_host(config: &AgentConfig, host: &str) -> Result<(), ApiError> {
    let forbidden = ['@', '/', '?', '#', '\\', ' ', '\t', '\n', '\r'];
    if host.is_empty()
        || host.len() > 253
        || host
            .chars()
            .any(|c| forbidden.contains(&c) || c.is_control())
    {
        return Err(ApiError {
            status: axum::http::StatusCode::BAD_REQUEST,
            message: "invalid remote agent address".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
        });
    }
    if !config.remote_agents.iter().any(|a| a == host) {
        tracing::warn!(%host, "rejected fleet proxy to unlisted host");
        return Err(ApiError {
            status: axum::http::StatusCode::FORBIDDEN,
            message: "host is not a configured remote agent".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
        });
    }
    Ok(())
}

async fn fleet_health(State(s): State<AppState>, Path(host): Path<String>) -> impl IntoResponse {
    if let Err(e) = check_fleet_host(&s.config, &host) {
        return e.into_response();
    }
    let url = format!("http://{}/api/health", host);
    match s
        .http_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            if let Ok(v) = res.json::<serde_json::Value>().await {
                Json(v).into_response()
            } else {
                bad_gateway("invalid JSON from remote agent")
            }
        }
        _ => bad_gateway("failed to reach remote agent"),
    }
}

fn bad_gateway(message: &str) -> axum::response::Response {
    ApiError {
        status: axum::http::StatusCode::BAD_GATEWAY,
        message: message.into(),
        request_id: uuid::Uuid::new_v4().to_string(),
    }
    .into_response()
}
