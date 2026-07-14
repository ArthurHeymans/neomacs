//! The axum HTTP server: routes, handlers, and the dedicated-thread entry point.

use std::convert::Infallible;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
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

/// Drives an on-demand Lisp CPU profile capture on the Lisp thread.
///
/// The host binary implements this by sending tasks over the eval-thread
/// channel and waking the Lisp thread. Both methods are synchronous (they block
/// on cross-thread channels), so handlers call them via `spawn_blocking`.
pub trait ProfileController: Send + Sync + 'static {
    /// Begin CPU sampling at `interval_ns` (fire-and-forget).
    fn start(&self, interval_ns: u64);
    /// Stop sampling and return Brendan-Gregg folded stacks, or an error string
    /// (e.g. no live Lisp thread, or a timeout).
    fn stop_and_fold(&self) -> Result<String, String>;
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) provider: Arc<dyn MetricsProvider>,
    pub(crate) profiler: Option<Arc<dyn ProfileController>>,
    /// Serializes captures so two concurrent `/profile` requests can't race the
    /// single shared profiler (double-start / stop-after-stop).
    pub(crate) capture_lock: Arc<tokio::sync::Mutex<()>>,
}

/// Build the diagnostics HTTP router.
pub fn router(
    provider: Arc<dyn MetricsProvider>,
    profiler: Option<Arc<dyn ProfileController>>,
) -> Router {
    let state = AppState {
        provider,
        profiler,
        capture_lock: Arc::new(tokio::sync::Mutex::new(())),
    };
    Router::new()
        .route("/", get(index))
        .route("/metrics", get(metrics))
        .route("/live", get(live))
        .route("/profile/lisp.folded", get(profile_folded))
        .route("/profile/lisp.svg", get(profile_svg))
        .route("/profile/lisp.pprof", get(profile_pprof))
        .route("/profile/lisp/callers", get(callers))
        .route("/report", get(report))
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
            "/live": "server-sent events stream of metrics (~1 Hz)",
            "/profile/lisp.folded?secs=N": "capture N s of Lisp CPU as folded stacks (text)",
            "/profile/lisp.svg?secs=N": "the same capture rendered as an SVG flamegraph",
            "/profile/lisp.pprof?secs=N": "the same capture as pprof protobuf (go tool pprof)",
            "/profile/lisp/callers?fn=NAME&secs=N": "callers/callees of NAME (JSON)",
            "/report?secs=N&top=K&sort=self|total": "ranked top-K CPU hotspots (JSON)"
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

/// Query parameters shared by the capture endpoints.
#[derive(serde::Deserialize)]
struct CaptureParams {
    secs: Option<u64>,
    top: Option<usize>,
    sort: Option<String>,
    #[serde(rename = "fn")]
    function: Option<String>,
}

/// Run one serialized capture: acquire the capture lock, start sampling, wait
/// `secs`, then stop and return folded stacks. `503` when no Lisp thread.
async fn do_capture(state: &AppState, secs: u64) -> Result<String, (StatusCode, String)> {
    let Some(ctrl) = state.profiler.clone() else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "no live Lisp thread (batch mode or diagnostics disabled)".to_string(),
        ));
    };
    let secs = secs.clamp(1, 60);
    let interval_ns = 1_000_000; // 1 ms sampling

    let _guard = state.capture_lock.lock().await;

    // start / stop_and_fold block on crossbeam channels; keep them off the
    // async runtime thread so /metrics and /live stay responsive.
    let c = ctrl.clone();
    let _ = tokio::task::spawn_blocking(move || c.start(interval_ns)).await;

    tokio::time::sleep(Duration::from_secs(secs)).await;

    let c = ctrl.clone();
    match tokio::task::spawn_blocking(move || c.stop_and_fold()).await {
        Ok(Ok(folded)) => Ok(folded),
        Ok(Err(e)) => Err((StatusCode::SERVICE_UNAVAILABLE, e)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn profile_folded(State(state): State<AppState>, Query(p): Query<CaptureParams>) -> Response {
    match do_capture(&state, p.secs.unwrap_or(5)).await {
        Ok(folded) => (
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            folded,
        )
            .into_response(),
        Err((code, msg)) => (code, msg).into_response(),
    }
}

async fn profile_svg(State(state): State<AppState>, Query(p): Query<CaptureParams>) -> Response {
    match do_capture(&state, p.secs.unwrap_or(5)).await {
        Ok(folded) => match crate::flamegraph::folded_to_svg(&folded, "Neomacs Lisp CPU") {
            Ok(svg) => ([(header::CONTENT_TYPE, "image/svg+xml")], svg).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        },
        Err((code, msg)) => (code, msg).into_response(),
    }
}

async fn profile_pprof(State(state): State<AppState>, Query(p): Query<CaptureParams>) -> Response {
    match do_capture(&state, p.secs.unwrap_or(5)).await {
        Ok(folded) => {
            let pb = crate::pprof::folded_to_pprof(&folded);
            ([(header::CONTENT_TYPE, "application/octet-stream")], pb).into_response()
        }
        Err((code, msg)) => (code, msg).into_response(),
    }
}

async fn callers(State(state): State<AppState>, Query(p): Query<CaptureParams>) -> Response {
    let Some(func) = p.function.clone() else {
        return (
            StatusCode::BAD_REQUEST,
            "missing required query parameter ?fn=FUNCTION\n",
        )
            .into_response();
    };
    match do_capture(&state, p.secs.unwrap_or(5)).await {
        Ok(folded) => {
            Json(crate::report::callers_report_from_folded(&folded, &func)).into_response()
        }
        Err((code, msg)) => (code, msg).into_response(),
    }
}

async fn report(State(state): State<AppState>, Query(p): Query<CaptureParams>) -> Response {
    let secs = p.secs.unwrap_or(5);
    match do_capture(&state, secs).await {
        Ok(folded) => {
            let top = p.top.unwrap_or(20).clamp(1, 1000);
            let sort_by_self = p.sort.as_deref() != Some("total");
            let rep = crate::report::cpu_report_from_folded(&folded, top, sort_by_self);
            Json(serde_json::json!({
                "window_secs": secs.clamp(1, 60),
                "sort": if sort_by_self { "self" } else { "total" },
                "report": rep,
            }))
            .into_response()
        }
        Err((code, msg)) => (code, msg).into_response(),
    }
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
/// `profiler` is `None` in batch/headless (no Lisp thread) — the capture
/// endpoints then return `503`. Bind/serve errors are logged, not panicked, so
/// a diagnostics failure never brings down the editor.
pub fn spawn(
    config: DiagnosticsConfig,
    provider: Arc<dyn MetricsProvider>,
    profiler: Option<Arc<dyn ProfileController>>,
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
                let app = router(provider, profiler);
                if let Err(e) = axum::serve(listener, app).await {
                    tracing::error!("diagnostics: server error: {e}");
                }
            });
        })
}
