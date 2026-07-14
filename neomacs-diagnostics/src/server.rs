//! The axum HTTP server: routes, handlers, and the dedicated-thread entry point.

use std::convert::Infallible;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

use crate::metrics::MetricsSnapshot;

/// Source of metrics for the server.
///
/// Implemented for any `Fn` returning a snapshot, so the host binary can supply
/// a closure over its producers without this crate depending on the VM.
pub trait MetricsProvider: Send + Sync + 'static {
    fn snapshot(&self) -> MetricsSnapshot;
}

impl<F> MetricsProvider for F
where
    F: Fn() -> MetricsSnapshot + Send + Sync + 'static,
{
    fn snapshot(&self) -> MetricsSnapshot {
        self()
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) provider: Arc<dyn MetricsProvider>,
}

/// Build the diagnostics HTTP router.
pub fn router(provider: Arc<dyn MetricsProvider>) -> Router {
    let state = AppState { provider };
    Router::new()
        .route("/", get(index))
        .route("/metrics", get(metrics))
        .route("/live", get(live))
        .with_state(state)
}

/// Self-describing index so an agent can navigate with no prior knowledge.
async fn index() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "neomacs-diagnostics",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "/": "this index",
            "/metrics": "current performance metrics snapshot (JSON)",
            "/live": "server-sent events stream of metrics (~1 Hz)"
        }
    }))
}

/// Current metrics snapshot as JSON.
async fn metrics(State(state): State<AppState>) -> Json<MetricsSnapshot> {
    Json(state.provider.snapshot())
}

/// Server-sent events stream: one JSON snapshot per event at ~1 Hz.
///
/// `tokio::time::interval` yields its first tick immediately, so a subscriber
/// receives a frame right away rather than after the first interval.
async fn live(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let provider = state.provider.clone();
    let interval = tokio::time::interval(Duration::from_millis(1000));
    let stream = IntervalStream::new(interval).map(move |_| {
        let snap = provider.snapshot();
        // `json_data` only fails for non-serializable values; ours always is.
        Ok(Event::default()
            .json_data(snap)
            .expect("MetricsSnapshot is serializable"))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Configuration for the diagnostics server.
pub struct DiagnosticsConfig {
    /// TCP port to bind on `127.0.0.1`.
    pub port: u16,
}

/// Parse a TCP port from a string, rejecting empty, zero, and out-of-range
/// values. Used to interpret the `NEOMACS_DIAGNOSTICS_PORT` env var.
pub fn port_from_str(raw: &str) -> Option<u16> {
    match raw.trim().parse::<u16>() {
        Ok(port) if port != 0 => Some(port),
        _ => None,
    }
}

/// Spawn the diagnostics server on a dedicated OS thread running a
/// current-thread tokio runtime. Binds `127.0.0.1:<port>` only.
///
/// Bind/serve errors are logged, not panicked, so a diagnostics failure never
/// brings down the editor.
pub fn spawn(
    config: DiagnosticsConfig,
    provider: Arc<dyn MetricsProvider>,
) -> std::io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("neomacs-diagnostics".to_string())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("diagnostics: failed to build tokio runtime: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], config.port));
                let listener = match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!("diagnostics: bind {addr} failed: {e}");
                        return;
                    }
                };
                tracing::info!("neomacs diagnostics listening on http://{addr}");
                let app = router(provider);
                if let Err(e) = axum::serve(listener, app).await {
                    tracing::error!("diagnostics: server error: {e}");
                }
            });
        })
}
