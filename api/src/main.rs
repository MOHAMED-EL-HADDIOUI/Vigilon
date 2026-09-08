use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use vigilon_agent::AgentRuntime;
use vigilon_api::{AppState, router};
use vigilon_core::blocklist::BlocklistManager;
use vigilon_core::{AgentConfig, EventBus, Storage};
use vigilon_platform::GeoIpResolver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let data_dir = data_dir();
    let db_path = data_dir.join("vigilon.sqlite");
    tracing::info!(path = %db_path.display(), "opening local SQLite store (WAL)");

    let storage = Storage::connect(&db_path).await.context("sqlite")?;
    let bus = EventBus::new();
    let config = AgentConfig::from_env();
    let blocklist = BlocklistManager::new(&data_dir);
    let geoip = GeoIpResolver::new(&data_dir);
    let notifier = Arc::new(vigilon_agent::notifier::Notifier::new(
        config.notifications_enabled,
        config.notification_min_risk.clone(),
    ));
    let http_client = reqwest::Client::builder().build()?;

    let runtime = AgentRuntime::new_full(
        bus.clone(),
        storage.clone(),
        config.clone(),
        blocklist.clone(),
        geoip.clone(),
        notifier.clone(),
    );

    let state = AppState {
        storage: runtime.storage.clone(),
        bus: runtime.bus.clone(),
        live: runtime.live.clone(),
        config: config.clone(),
        blocklist: blocklist.clone(),
        http_client,
    };
    runtime.spawn();

    let bind = config.bind.clone();
    let addr: SocketAddr = bind.parse().context("bind addr")?;
    if !addr.ip().is_loopback() {
        tracing::warn!(
            %addr,
            "binding a non-loopback address with no authentication: \
             only do this behind a trusted reverse proxy or firewall. \
             Set VIGILON_BIND=127.0.0.1:8745 to keep the API local-only."
        );
    }
    let app = router(state, frontend_dir());
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "Vigilon is local-first — listening (no outbound telemetry)");
    axum::serve(listener, app).await?;
    Ok(())
}

fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("VIGILON_DATA") {
        return PathBuf::from(p);
    }
    dirs_data()
}

fn dirs_data() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(base) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(base).join("Vigilon");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/vigilon");
    }
    PathBuf::from("./data")
}

fn frontend_dir() -> PathBuf {
    if let Ok(p) = std::env::var("VIGILON_FRONTEND") {
        return PathBuf::from(p);
    }
    let candidates = [
        PathBuf::from("frontend/dist"),
        PathBuf::from("../frontend/dist"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("frontend/dist")))
            .unwrap_or_else(|| PathBuf::from("frontend/dist")),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("frontend/dist"))
}
