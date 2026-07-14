# Profiler Toolchain — Phase 1a (tokio-edge diagnostics server + metrics + SSE) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a localhost HTTP diagnostics server on a dedicated tokio thread that exposes neomacs frame + GC/heap metrics as JSON (`GET /metrics`) and a live SSE feed (`GET /live`), off by default, enabled by `--diagnostics-port N`.

**Architecture:** A new leaf crate `neomacs-diagnostics` owns an `axum` server driven by a `tokio` current-thread runtime on its own OS thread. The crate is decoupled from the VM: it reads metrics through a `MetricsProvider` closure supplied by `neomacs-bin`. Producers publish snapshots to process-global atomics (`frame_stats` already does; GC gets a new mirror). The Lisp VM stays synchronous — the diagnostics thread never touches VM state, only reads lock-free published atomics.

**Tech Stack:** Rust 2024, `axum` 0.8, `tokio` 1 (current-thread), `tokio-stream`, `serde`/`serde_json`, `crossbeam` (already in tree). Testing: `tower::ServiceExt::oneshot` against the `Router` (no live socket needed).

## Global Constraints

- Edition `2024`; workspace version `0.0.12`; member crates inherit `version/edition/authors/license/repository` via `.workspace = true` and end with `[lints] workspace = true`.
- Use `cargo nextest run` (debug) for tests; `cargo check` for compile verification. **Never `--release`** for routine builds/tests. Always set a Bash timeout.
- Commit messages: **no Markdown backticks**.
- Diagnostics server binds **`127.0.0.1` only**, **off by default**. No auth/TLS in this phase.
- tokio must **not** appear in `neovm-core` or color any VM API with `async`/`await`. It lives only in `neomacs-diagnostics` and is spawned from `neomacs-bin`.
- This is **Phase 1a**. Out of scope (deferred to the Phase 1b plan): Lisp CPU capture, folded stacks, inferno SVG, `/report`, `/profile/*`, pprof, input-latency instrumentation, Chrome/Perfetto trace.

---

## File Structure

- Create: `neomacs-diagnostics/Cargo.toml` — new leaf crate manifest.
- Create: `neomacs-diagnostics/src/lib.rs` — crate root, re-exports.
- Create: `neomacs-diagnostics/src/metrics.rs` — `MetricsSnapshot` + sub-structs (serde), pure data.
- Create: `neomacs-diagnostics/src/server.rs` — `MetricsProvider` trait, `router()`, `spawn()`, handlers.
- Create: `neomacs-diagnostics/src/server_test.rs` — router integration tests (oneshot).
- Create: `neomacs-diagnostics/src/metrics_test.rs` — snapshot serde tests.
- Create: `neovm-core/src/emacs_core/gc_stats.rs` — process-global GC snapshot atomics + publish/read.
- Modify: `neovm-core/src/emacs_core/mod.rs` — declare `pub mod gc_stats;`.
- Modify: `neovm-core/src/emacs_core/eval.rs:6048` (`update_gc_runtime_stats`) — publish GC snapshot.
- Modify: `neomacs-display-runtime/src/lib.rs` — expose a public `frame_metrics_snapshot()`.
- Modify: `neomacs-display-runtime/src/render_thread/frame_stats.rs` — make `FrameSchedSnapshot` + `snapshot()` reachable publicly.
- Modify: `Cargo.toml` (root) — add crate to `members` + `[workspace.dependencies]`.
- Modify: `neomacs-bin/Cargo.toml` — depend on `neomacs-diagnostics`.
- Modify: `neomacs-bin/src/main.rs` (`StartupOptions` ~153; `parse_startup_options` ~359; `run_gui_evaluator_worker` ~2557; `run` TTY ~2879) — add flag, provider, spawn.
- Modify: `neomacs-bin/src/args.rs:169` (`STANDARD_ARGS`) — register `--diagnostics-port`.

---

## Task 1: Scaffold the `neomacs-diagnostics` crate and wire the workspace

**Files:**
- Create: `neomacs-diagnostics/Cargo.toml`
- Create: `neomacs-diagnostics/src/lib.rs`
- Modify: `Cargo.toml` (root): `members` list (`:2-14`) and `[workspace.dependencies]` internal block (`~:47-52`)

**Interfaces:**
- Produces: crate `neomacs-diagnostics` compiling as a workspace member; empty `lib.rs`.

- [ ] **Step 1: Create the crate manifest**

Create `neomacs-diagnostics/Cargo.toml`:

```toml
[package]
name = "neomacs-diagnostics"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
axum = { version = "0.8", default-features = false, features = ["http1", "json", "tokio", "query"] }
tokio = { version = "1", default-features = false, features = ["rt", "net", "time"] }
tokio-stream = { version = "0.1", default-features = false, features = ["time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing.workspace = true

[dev-dependencies]
tokio = { version = "1", default-features = false, features = ["rt", "net", "time", "macros"] }
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"

[lints]
workspace = true
```

- [ ] **Step 2: Create an empty crate root**

Create `neomacs-diagnostics/src/lib.rs`:

```rust
//! Localhost HTTP diagnostics server for neomacs performance introspection.
//!
//! Runs on a dedicated OS thread with a tokio current-thread runtime. Reads
//! metrics through a `MetricsProvider` supplied by the host binary; never
//! touches VM state directly.
```

- [ ] **Step 3: Add the crate to the workspace members**

In root `Cargo.toml`, add `"neomacs-diagnostics",` to the `members` array (after `"neomacs-renderer-wgpu",`).

- [ ] **Step 4: Declare the internal workspace dependency**

In root `Cargo.toml` `[workspace.dependencies]`, in the internal-crates block, add:

```toml
neomacs-diagnostics = { path = "neomacs-diagnostics", version = "0.0.12", default-features = false }
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p neomacs-diagnostics`
Expected: compiles clean (downloads axum/tokio on first run — allow up to 5 min).

- [ ] **Step 6: Commit**

```bash
git add neomacs-diagnostics/Cargo.toml neomacs-diagnostics/src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat(diagnostics): scaffold neomacs-diagnostics crate"
```

---

## Task 2: Define the metrics snapshot data model

**Files:**
- Create: `neomacs-diagnostics/src/metrics.rs`
- Create: `neomacs-diagnostics/src/metrics_test.rs`
- Modify: `neomacs-diagnostics/src/lib.rs`

**Interfaces:**
- Produces: `MetricsSnapshot { frame: FrameMetrics, gc: GcMetrics }`, all `#[derive(Debug, Clone, Default, PartialEq, Serialize)]`, all fields `pub`. Field names are the JSON contract — later tasks and the Phase 1b plan depend on them exactly.

- [ ] **Step 1: Write the failing serde test**

Create `neomacs-diagnostics/src/metrics_test.rs`:

```rust
use crate::metrics::{FrameMetrics, GcMetrics, MetricsSnapshot};

#[test]
fn snapshot_serializes_stable_json_shape() {
    let snap = MetricsSnapshot {
        frame: FrameMetrics {
            presents: 100,
            scene_commits: 90,
            wakeups: 500,
            last_commit_to_present_us: 1200,
            max_commit_to_present_us: 8000,
            composite_only_frames: 10,
            retained_static_builds: 3,
        },
        gc: GcMetrics {
            collections: 7,
            live_bytes: 4096,
            bytes_since_gc: 512,
            total_allocated_bytes: 1_000_000,
            cons_cells: 200,
            strings: 40,
            vector_cells: 60,
            symbols: 80,
        },
    };
    let v: serde_json::Value = serde_json::to_value(&snap).unwrap();
    assert_eq!(v["frame"]["presents"], 100);
    assert_eq!(v["frame"]["last_commit_to_present_us"], 1200);
    assert_eq!(v["gc"]["collections"], 7);
    assert_eq!(v["gc"]["cons_cells"], 200);
    assert_eq!(MetricsSnapshot::default().frame.presents, 0);
}
```

- [ ] **Step 2: Reference the test module in lib.rs (so it fails to compile → fails)**

Append to `neomacs-diagnostics/src/lib.rs`:

```rust
pub mod metrics;
pub use metrics::MetricsSnapshot;

#[cfg(test)]
mod metrics_test;
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo nextest run -p neomacs-diagnostics snapshot_serializes 2>&1 | tail -20`
Expected: FAIL — `metrics` module / types not found.

- [ ] **Step 4: Implement the data model**

Create `neomacs-diagnostics/src/metrics.rs`:

```rust
use serde::Serialize;

/// A point-in-time snapshot of neomacs performance metrics.
///
/// Field names are the JSON API contract for `/metrics` and `/live`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct MetricsSnapshot {
    pub frame: FrameMetrics,
    pub gc: GcMetrics,
}

/// Render/frame-scheduling counters. Mirrors `FrameSchedSnapshot`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FrameMetrics {
    pub presents: u64,
    pub scene_commits: u64,
    pub wakeups: u64,
    /// Latency from scene commit to present for the last frame (microseconds).
    pub last_commit_to_present_us: u64,
    /// Worst commit-to-present latency observed (microseconds).
    pub max_commit_to_present_us: u64,
    pub composite_only_frames: u64,
    pub retained_static_builds: u64,
}

/// Lisp GC / heap counters. Mirrors the published `GcStatsSnapshot`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct GcMetrics {
    pub collections: u64,
    pub live_bytes: u64,
    pub bytes_since_gc: u64,
    pub total_allocated_bytes: u64,
    pub cons_cells: u64,
    pub strings: u64,
    pub vector_cells: u64,
    pub symbols: u64,
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo nextest run -p neomacs-diagnostics snapshot_serializes 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add neomacs-diagnostics/src/metrics.rs neomacs-diagnostics/src/metrics_test.rs neomacs-diagnostics/src/lib.rs
git commit -m "feat(diagnostics): add MetricsSnapshot data model"
```

---

## Task 3: Build the axum router with `/` and `/metrics`

**Files:**
- Create: `neomacs-diagnostics/src/server.rs`
- Create: `neomacs-diagnostics/src/server_test.rs`
- Modify: `neomacs-diagnostics/src/lib.rs`

**Interfaces:**
- Consumes: `MetricsSnapshot` (Task 2).
- Produces:
  - `pub trait MetricsProvider: Send + Sync + 'static { fn snapshot(&self) -> MetricsSnapshot; }`
  - blanket `impl<F: Fn() -> MetricsSnapshot + Send + Sync + 'static> MetricsProvider for F`
  - `pub fn router(provider: std::sync::Arc<dyn MetricsProvider>) -> axum::Router`

- [ ] **Step 1: Write the failing router test**

Create `neomacs-diagnostics/src/server_test.rs`:

```rust
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // for `oneshot`

use crate::metrics::{FrameMetrics, GcMetrics, MetricsSnapshot};
use crate::server::router;

fn fixed_provider() -> Arc<dyn crate::server::MetricsProvider> {
    Arc::new(|| MetricsSnapshot {
        frame: FrameMetrics { presents: 42, ..Default::default() },
        gc: GcMetrics { collections: 3, ..Default::default() },
    })
}

#[tokio::test]
async fn metrics_route_returns_snapshot_json() {
    let app = router(fixed_provider());
    let resp = app
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["frame"]["presents"], 42);
    assert_eq!(v["gc"]["collections"], 3);
}

#[tokio::test]
async fn index_route_is_self_describing() {
    let app = router(fixed_provider());
    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["name"], "neomacs-diagnostics");
    assert!(v["endpoints"]["/metrics"].is_string());
}
```

- [ ] **Step 2: Reference server module in lib.rs**

Append to `neomacs-diagnostics/src/lib.rs`:

```rust
pub mod server;
pub use server::{MetricsProvider, router};

#[cfg(test)]
mod server_test;
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo nextest run -p neomacs-diagnostics metrics_route 2>&1 | tail -20`
Expected: FAIL — `server` module / `router` not found.

- [ ] **Step 4: Implement the provider trait, state, and router with `/` + `/metrics`**

Create `neomacs-diagnostics/src/server.rs`:

```rust
use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use axum::routing::get;
use axum::Router;

use crate::metrics::MetricsSnapshot;

/// Source of metrics for the server. Implemented for any `Fn` returning a
/// snapshot, so the host binary can supply a closure over its producers.
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
        .with_state(state)
}

async fn index() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "neomacs-diagnostics",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "/metrics": "current performance metrics snapshot (JSON)",
            "/live": "server-sent events stream of metrics (~1 Hz)"
        }
    }))
}

async fn metrics(State(state): State<AppState>) -> Json<MetricsSnapshot> {
    Json(state.provider.snapshot())
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo nextest run -p neomacs-diagnostics 2>&1 | tail -20`
Expected: PASS (`metrics_route_returns_snapshot_json`, `index_route_is_self_describing`, plus Task 2 test).

- [ ] **Step 6: Commit**

```bash
git add neomacs-diagnostics/src/server.rs neomacs-diagnostics/src/server_test.rs neomacs-diagnostics/src/lib.rs
git commit -m "feat(diagnostics): axum router with / and /metrics"
```

---

## Task 4: Add the `/live` SSE stream

**Files:**
- Modify: `neomacs-diagnostics/src/server.rs` (add `live` handler + route)
- Modify: `neomacs-diagnostics/src/server_test.rs` (add SSE test)

**Interfaces:**
- Consumes: `AppState`, `MetricsProvider` (Task 3).
- Produces: `GET /live` route returning `text/event-stream`, emitting one JSON `MetricsSnapshot` per event at ~1 Hz.

- [ ] **Step 1: Write the failing SSE test**

Append to `neomacs-diagnostics/src/server_test.rs`:

```rust
#[tokio::test]
async fn live_route_emits_event_stream() {
    let app = router(fixed_provider());
    let resp = app
        .oneshot(Request::builder().uri("/live").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.starts_with("text/event-stream"), "content-type was {ct}");

    // Read just the first SSE frame, then drop the stream (it is infinite).
    let mut body = resp.into_body().into_data_stream();
    let first = {
        use futures_util_stub::next;
        next(&mut body).await
    }
    .expect("at least one chunk")
    .unwrap();
    let text = String::from_utf8_lossy(&first);
    assert!(text.contains("data:"), "frame was {text}");
    assert!(text.contains("\"presents\":42"), "frame was {text}");
}
```

Note: replace the `futures_util_stub::next` helper with the concrete poll below — add this small helper to the test file (avoids a `futures-util` dev-dep):

```rust
mod futures_util_stub {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    /// Await the next item of an `http_body_util` data stream.
    pub async fn next<S>(stream: &mut S) -> Option<S::Item>
    where
        S: futures_core::Stream + Unpin,
    {
        struct Next<'a, S>(&'a mut S);
        impl<'a, S: futures_core::Stream + Unpin> Future for Next<'a, S> {
            type Output = Option<S::Item>;
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                Pin::new(&mut *self.0).poll_next(cx)
            }
        }
        Next(stream).await
    }
}
```

Add `futures-core = "0.3"` to `[dev-dependencies]` in `neomacs-diagnostics/Cargo.toml`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo nextest run -p neomacs-diagnostics live_route 2>&1 | tail -20`
Expected: FAIL — no `/live` route (404, not `text/event-stream`).

- [ ] **Step 3: Implement the SSE handler and route**

In `neomacs-diagnostics/src/server.rs`, add imports at the top:

```rust
use std::convert::Infallible;
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use tokio_stream::{Stream, StreamExt};
use tokio_stream::wrappers::IntervalStream;
```

Add the `/live` route to the router builder (between `/metrics` and `.with_state`):

```rust
        .route("/live", get(live))
```

Add the handler:

```rust
async fn live(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let provider = state.provider.clone();
    let interval = tokio::time::interval(Duration::from_millis(1000));
    let stream = IntervalStream::new(interval).map(move |_| {
        let snap = provider.snapshot();
        // json_data only fails if the value is not serializable; ours always is.
        Ok(Event::default().json_data(snap).expect("MetricsSnapshot is serializable"))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo nextest run -p neomacs-diagnostics 2>&1 | tail -20`
Expected: PASS (all four tests).

- [ ] **Step 5: Commit**

```bash
git add neomacs-diagnostics/src/server.rs neomacs-diagnostics/src/server_test.rs neomacs-diagnostics/Cargo.toml Cargo.lock
git commit -m "feat(diagnostics): add /live server-sent events metrics stream"
```

---

## Task 5: Publish GC stats to a process-global snapshot in neovm-core

**Files:**
- Create: `neovm-core/src/emacs_core/gc_stats.rs`
- Modify: `neovm-core/src/emacs_core/mod.rs` (add `pub mod gc_stats;`)
- Modify: `neovm-core/src/emacs_core/eval.rs` (`update_gc_runtime_stats`, `:6048`)

**Interfaces:**
- Produces:
  - `pub struct GcStatsSnapshot { pub collections: u64, pub live_bytes: u64, pub bytes_since_gc: u64, pub total_allocated_bytes: u64, pub cons_cells: u64, pub strings: u64, pub vector_cells: u64, pub symbols: u64 }`
  - `pub fn publish(snap: GcStatsSnapshot)`
  - `pub fn snapshot() -> GcStatsSnapshot`
- Consumes (from `TaggedHeap`, existing): `gc_collections()`, `live_bytes()`, `bytes_since_gc()`, `total_allocated_bytes()`, `memory_use_counts_snapshot() -> [u64; 7]` (slots: 0=ConsCells, 2=VectorCells, 3=Symbols, 6=Strings).

- [ ] **Step 1: Write the failing round-trip test**

Create `neovm-core/src/emacs_core/gc_stats.rs` with the test first (implementation stubbed to force failure):

```rust
//! Process-global publication of GC/heap counters so an off-thread reader
//! (the diagnostics server) can sample them lock-free, mirroring the
//! `frame_stats` pattern in the display runtime.

use std::sync::atomic::{AtomicU64, Ordering};

static COLLECTIONS: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static BYTES_SINCE_GC: AtomicU64 = AtomicU64::new(0);
static TOTAL_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static CONS_CELLS: AtomicU64 = AtomicU64::new(0);
static STRINGS: AtomicU64 = AtomicU64::new(0);
static VECTOR_CELLS: AtomicU64 = AtomicU64::new(0);
static SYMBOLS: AtomicU64 = AtomicU64::new(0);

/// A snapshot of published GC/heap counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GcStatsSnapshot {
    pub collections: u64,
    pub live_bytes: u64,
    pub bytes_since_gc: u64,
    pub total_allocated_bytes: u64,
    pub cons_cells: u64,
    pub strings: u64,
    pub vector_cells: u64,
    pub symbols: u64,
}

/// Publish the latest counters (called on the Lisp thread after a GC cycle).
pub fn publish(snap: GcStatsSnapshot) {
    COLLECTIONS.store(snap.collections, Ordering::Relaxed);
    LIVE_BYTES.store(snap.live_bytes, Ordering::Relaxed);
    BYTES_SINCE_GC.store(snap.bytes_since_gc, Ordering::Relaxed);
    TOTAL_ALLOCATED_BYTES.store(snap.total_allocated_bytes, Ordering::Relaxed);
    CONS_CELLS.store(snap.cons_cells, Ordering::Relaxed);
    STRINGS.store(snap.strings, Ordering::Relaxed);
    VECTOR_CELLS.store(snap.vector_cells, Ordering::Relaxed);
    SYMBOLS.store(snap.symbols, Ordering::Relaxed);
}

/// Read the most recently published counters (safe from any thread).
pub fn snapshot() -> GcStatsSnapshot {
    GcStatsSnapshot {
        collections: COLLECTIONS.load(Ordering::Relaxed),
        live_bytes: LIVE_BYTES.load(Ordering::Relaxed),
        bytes_since_gc: BYTES_SINCE_GC.load(Ordering::Relaxed),
        total_allocated_bytes: TOTAL_ALLOCATED_BYTES.load(Ordering::Relaxed),
        cons_cells: CONS_CELLS.load(Ordering::Relaxed),
        strings: STRINGS.load(Ordering::Relaxed),
        vector_cells: VECTOR_CELLS.load(Ordering::Relaxed),
        symbols: SYMBOLS.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_then_snapshot_round_trips() {
        publish(GcStatsSnapshot {
            collections: 11,
            live_bytes: 2048,
            bytes_since_gc: 128,
            total_allocated_bytes: 999,
            cons_cells: 5,
            strings: 6,
            vector_cells: 7,
            symbols: 8,
        });
        let got = snapshot();
        assert_eq!(got.collections, 11);
        assert_eq!(got.symbols, 8);
        assert_eq!(got.live_bytes, 2048);
    }
}
```

- [ ] **Step 2: Declare the module**

In `neovm-core/src/emacs_core/mod.rs`, add alongside the other `pub mod` declarations:

```rust
pub mod gc_stats;
```

- [ ] **Step 3: Run the test to verify it passes (module compiles + round-trips)**

Run: `cargo nextest run -p neovm-core publish_then_snapshot 2>&1 | tail -20`
Expected: PASS. (This task's unit is the publish/read mechanism; wiring it to real GC is the next steps.)

- [ ] **Step 4: Publish real counters after each GC cycle**

In `neovm-core/src/emacs_core/eval.rs`, locate `fn update_gc_runtime_stats` (around `:6048`). At the end of that function body, append:

```rust
        let counts = self.tagged_heap.memory_use_counts_snapshot();
        crate::emacs_core::gc_stats::publish(crate::emacs_core::gc_stats::GcStatsSnapshot {
            collections: self.tagged_heap.gc_collections() as u64,
            live_bytes: self.tagged_heap.live_bytes() as u64,
            bytes_since_gc: self.tagged_heap.bytes_since_gc() as u64,
            total_allocated_bytes: self.tagged_heap.total_allocated_bytes(),
            cons_cells: counts[0],
            vector_cells: counts[2],
            symbols: counts[3],
            strings: counts[6],
        });
```

(If `update_gc_runtime_stats` takes `&self` rather than `&mut self`, these getters are all `&self`, so no signature change is needed. Verify `self.tagged_heap` is reachable there; it is the evaluator's heap field.)

- [ ] **Step 5: Verify the crate still builds with the wiring**

Run: `cargo check -p neovm-core 2>&1 | tail -20`
Expected: compiles clean.

- [ ] **Step 6: Commit**

```bash
git add neovm-core/src/emacs_core/gc_stats.rs neovm-core/src/emacs_core/mod.rs neovm-core/src/emacs_core/eval.rs
git commit -m "feat(diagnostics): publish GC heap counters to a process-global snapshot"
```

---

## Task 6: Expose the frame-stats snapshot publicly from the display runtime

**Files:**
- Modify: `neomacs-display-runtime/src/render_thread/frame_stats.rs` (visibility of `FrameSchedSnapshot` + `snapshot()`)
- Modify: `neomacs-display-runtime/src/lib.rs` (public re-export)

**Interfaces:**
- Produces: `neomacs_display_runtime::frame_metrics_snapshot() -> FrameSchedSnapshot`, with `FrameSchedSnapshot` public and its fields `pub`.

- [ ] **Step 1: Make the snapshot type and getter public**

In `neomacs-display-runtime/src/render_thread/frame_stats.rs`:
- Change `pub(crate) struct FrameSchedSnapshot` (`:100`) to `pub struct FrameSchedSnapshot`, and ensure each field (`:101-114`) is `pub`.
- Change `pub(crate) fn snapshot()` (`:117`) to `pub fn snapshot()`.

- [ ] **Step 2: Add a public re-export from the crate root**

In `neomacs-display-runtime/src/lib.rs`, add a public function (place near other public re-exports):

```rust
/// Read the current process-global frame-scheduling counters.
///
/// Safe from any thread — the counters are relaxed atomics.
pub fn frame_metrics_snapshot() -> crate::render_thread::frame_stats::FrameSchedSnapshot {
    crate::render_thread::frame_stats::snapshot()
}
```

If `render_thread` or `frame_stats` is not already reachable on that path, add the minimal `pub(crate)`/`pub use` needed. Verify the module path matches how `render_thread` is declared in `lib.rs`.

- [ ] **Step 3: Write a smoke test for the public getter**

Create `neomacs-display-runtime/src/frame_metrics_pub_test.rs`:

```rust
#[test]
fn frame_metrics_snapshot_is_publicly_callable() {
    // Before any rendering the counters are zero; the point is that the
    // public getter compiles and returns the snapshot type off-thread.
    let snap = crate::frame_metrics_snapshot();
    let _ = snap.presents;
    let _ = snap.last_commit_to_present_us;
}
```

Reference it in `neomacs-display-runtime/src/lib.rs`:

```rust
#[cfg(test)]
mod frame_metrics_pub_test;
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo nextest run -p neomacs-display-runtime frame_metrics_snapshot_is_publicly 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add neomacs-display-runtime/src/render_thread/frame_stats.rs neomacs-display-runtime/src/lib.rs neomacs-display-runtime/src/frame_metrics_pub_test.rs
git commit -m "feat(diagnostics): expose frame_metrics_snapshot from display runtime"
```

---

## Task 7: Add the `spawn` entry point to the diagnostics crate

**Files:**
- Modify: `neomacs-diagnostics/src/server.rs` (add `DiagnosticsConfig` + `spawn`)
- Modify: `neomacs-diagnostics/src/lib.rs` (re-export)
- Modify: `neomacs-diagnostics/src/server_test.rs` (end-to-end socket test)

**Interfaces:**
- Consumes: `router()`, `MetricsProvider`.
- Produces: `pub struct DiagnosticsConfig { pub port: u16 }` and `pub fn spawn(config: DiagnosticsConfig, provider: Arc<dyn MetricsProvider>) -> std::io::Result<std::thread::JoinHandle<()>>`.

- [ ] **Step 1: Write the failing end-to-end test (real socket on port 0 → OS-assigned)**

Because `spawn` binds a fixed port, add a testable internal that binds a provided `TcpListener`. Append to `neomacs-diagnostics/src/server_test.rs`:

```rust
#[tokio::test]
async fn serve_on_listener_answers_metrics_over_tcp() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(fixed_provider());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let text = String::from_utf8_lossy(&buf);
    assert!(text.contains("200 OK"), "response was {text}");
    assert!(text.contains("\"presents\":42"), "response was {text}");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo nextest run -p neomacs-diagnostics serve_on_listener 2>&1 | tail -20`
Expected: FAIL initially only if `axum::serve` import is missing; if it already passes, that is acceptable — this test also guards Task 7's runtime wiring. Proceed to implement `spawn`.

- [ ] **Step 3: Implement `DiagnosticsConfig` and `spawn`**

Append to `neomacs-diagnostics/src/server.rs`:

```rust
use std::thread::{self, JoinHandle};

/// Configuration for the diagnostics server.
pub struct DiagnosticsConfig {
    /// TCP port to bind on `127.0.0.1`.
    pub port: u16,
}

/// Spawn the diagnostics server on a dedicated OS thread running a
/// current-thread tokio runtime. Binds `127.0.0.1:<port>` only.
///
/// Returns the thread handle. Bind/serve errors are logged, not panicked, so a
/// diagnostics failure never brings down the editor.
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
```

- [ ] **Step 4: Re-export from lib.rs**

Update the `pub use` in `neomacs-diagnostics/src/lib.rs`:

```rust
pub use server::{DiagnosticsConfig, MetricsProvider, router, spawn};
```

- [ ] **Step 5: Run all crate tests**

Run: `cargo nextest run -p neomacs-diagnostics 2>&1 | tail -20`
Expected: PASS (all tests).

- [ ] **Step 6: Commit**

```bash
git add neomacs-diagnostics/src/server.rs neomacs-diagnostics/src/lib.rs neomacs-diagnostics/src/server_test.rs
git commit -m "feat(diagnostics): add spawn() entry point on a dedicated thread"
```

---

## Task 8: Wire `--diagnostics-port` and start the server in neomacs-bin

**Files:**
- Modify: `neomacs-bin/Cargo.toml` (add dep)
- Modify: `neomacs-bin/src/args.rs:169` (`STANDARD_ARGS`)
- Modify: `neomacs-bin/src/main.rs`: `StartupOptions` (`~:153`); `parse_startup_options` (`~:359`); GUI worker (`run_gui_evaluator_worker`, before `recursive_edit()` at `:2583`); TTY path in `run` (before `recursive_edit()` at `:2903`).

**Interfaces:**
- Consumes: `neomacs_diagnostics::{spawn, DiagnosticsConfig, MetricsSnapshot}`, `neomacs_diagnostics::metrics::{FrameMetrics, GcMetrics}`, `neomacs_display_runtime::frame_metrics_snapshot`, `neovm_core::emacs_core::gc_stats`.
- Produces: server started iff `--diagnostics-port` given; a `build_metrics_snapshot()` helper mapping producers → `MetricsSnapshot`.

- [ ] **Step 1: Add the crate dependency**

In `neomacs-bin/Cargo.toml` `[dependencies]`, add:

```toml
neomacs-diagnostics.workspace = true
```

- [ ] **Step 2: Register the CLI flag**

In `neomacs-bin/src/args.rs`, add a row to `STANDARD_ARGS` (`:169`), following an existing value-taking entry's shape:

```rust
    StandardArg { name: "-diagnostics-port", longname: Some("--diagnostics-port"), priority: 35, nargs: 1 },
```

(Pick a `priority` consistent with neighboring non-critical options; `35` is illustrative — match the surrounding table's convention so ordering stays stable.)

- [ ] **Step 3: Add the option field and parse it**

In `neomacs-bin/src/main.rs`, add to `struct StartupOptions` (`:153`):

```rust
    diagnostics_port: Option<u16>,
```

Initialize it to `None` wherever `StartupOptions` is constructed (default). In `parse_startup_options` (`:359`), add an arm mirroring the `-t` handling (`:594`):

```rust
        if let ArgMatch::Value(v) =
            argmatch(&parsed, &mut idx, "-diagnostics-port", Some("--diagnostics-port"), 0, true)
        {
            match v.parse::<u16>() {
                Ok(port) => options.diagnostics_port = Some(port),
                Err(_) => eprintln!("neomacs: invalid --diagnostics-port value: {v}"),
            }
            continue;
        }
```

(Match the exact `argmatch`/loop idiom already used in this function; the key is `options.diagnostics_port = Some(port)`.)

- [ ] **Step 4: Add the metrics-provider helper**

In `neomacs-bin/src/main.rs`, add a free function:

```rust
/// Assemble a diagnostics metrics snapshot from the live producers.
///
/// Reads only process-global published atomics — safe from the diagnostics
/// thread with no VM access.
fn build_metrics_snapshot() -> neomacs_diagnostics::MetricsSnapshot {
    use neomacs_diagnostics::metrics::{FrameMetrics, GcMetrics};

    let f = neomacs_display_runtime::frame_metrics_snapshot();
    let g = neovm_core::emacs_core::gc_stats::snapshot();

    neomacs_diagnostics::MetricsSnapshot {
        frame: FrameMetrics {
            presents: f.presents,
            scene_commits: f.scene_commits,
            wakeups: f.wakeups,
            last_commit_to_present_us: f.last_commit_to_present_us,
            max_commit_to_present_us: f.max_commit_to_present_us,
            composite_only_frames: f.composite_only_frames,
            retained_static_builds: f.retained_static_builds,
        },
        gc: GcMetrics {
            collections: g.collections,
            live_bytes: g.live_bytes,
            bytes_since_gc: g.bytes_since_gc,
            total_allocated_bytes: g.total_allocated_bytes,
            cons_cells: g.cons_cells,
            strings: g.strings,
            vector_cells: g.vector_cells,
            symbols: g.symbols,
        },
    }
}
```

(Confirm `FrameSchedSnapshot`'s field names match `f.*` here; they are the fields exposed in Task 6. If a field name differs, adjust to the real name.)

- [ ] **Step 5: Add a start helper and call it from both frontends**

Add a helper in `neomacs-bin/src/main.rs`:

```rust
/// Start the diagnostics HTTP server if a port was requested. Best-effort:
/// a failure is logged and ignored so it never blocks editor startup.
fn maybe_start_diagnostics(options: &StartupOptions) {
    let Some(port) = options.diagnostics_port else { return };
    let provider = std::sync::Arc::new(build_metrics_snapshot);
    match neomacs_diagnostics::spawn(neomacs_diagnostics::DiagnosticsConfig { port }, provider) {
        Ok(_handle) => tracing::info!("diagnostics server requested on 127.0.0.1:{port}"),
        Err(e) => tracing::error!("failed to start diagnostics server: {e}"),
    }
}
```

Call `maybe_start_diagnostics(&startup)` in **both** places, just before the main loop:
- GUI: in `run_gui_evaluator_worker`, immediately before `let exit_status = evaluator.recursive_edit();` (`:2583`).
- TTY: in `run`, immediately before `evaluator.recursive_edit()` (`:2903`).

(Confirm the in-scope binding holding `StartupOptions` is named `startup` at each site; the exploration shows `parse_startup_options()` result flows through `run`. Use whatever local name is in scope.)

- [ ] **Step 6: Verify the whole workspace builds**

Run: `cargo check 2>&1 | tail -30`
Expected: compiles clean across the workspace.

- [ ] **Step 7: Manual end-to-end verification**

Build and run neomacs in batch/TTY with the flag, then curl it from another shell:

```bash
cargo build 2>&1 | tail -5
./target/debug/neomacs --diagnostics-port 9099 &
sleep 3
curl -s http://127.0.0.1:9099/ | head -c 400; echo
curl -s http://127.0.0.1:9099/metrics | head -c 400; echo
# stream two live frames then stop:
timeout 3 curl -s http://127.0.0.1:9099/live | head -c 400; echo
kill %1 2>/dev/null
```

Expected: `/` returns the self-describing JSON; `/metrics` returns `{"frame":{...},"gc":{...}}`; `/live` emits `data: {...}` frames. (If run under a headless environment where the GUI won't start, use the TTY/batch path.)

- [ ] **Step 8: Commit**

```bash
git add neomacs-bin/Cargo.toml neomacs-bin/src/args.rs neomacs-bin/src/main.rs Cargo.lock
git commit -m "feat(diagnostics): start server on --diagnostics-port, off by default"
```

---

## Task 9: Full-suite regression check

**Files:** none (verification only)

- [ ] **Step 1: Build the workspace fresh**

Run: `cargo check --workspace 2>&1 | tail -20`
Expected: clean.

- [ ] **Step 2: Run the diagnostics + touched-crate tests**

Run (timeout 900000):
```bash
cargo nextest run -p neomacs-diagnostics -p neovm-core -p neomacs-display-runtime 2>&1 | tail -30
```
Expected: all pass; no new failures in neovm-core / display-runtime from the visibility and GC-publish changes.

- [ ] **Step 3: Confirm clippy is clean for the new crate**

Run: `cargo clippy -p neomacs-diagnostics -- -D warnings 2>&1 | tail -20`
Expected: no warnings (workspace lints treat clippy warnings as errors in CI).

- [ ] **Step 4: Commit any fixups**

```bash
git add -A
git commit -m "test(diagnostics): phase 1a regression pass" --allow-empty
```

---

## Self-Review

**Spec coverage (Phase 1a subset of the toolchain spec):**
- New `neomacs-diagnostics` crate on axum/tokio-edge, own thread, 127.0.0.1, off by default → Tasks 1, 7, 8. ✓
- tokio boundary (no async in neovm-core; channels/atomics only) → GC exposed via lock-free atomics (Task 5), frame via existing atomics (Task 6); server reads via provider closure (Tasks 3, 8). ✓
- `/metrics` snapshot + `/live` SSE → Tasks 3, 4. ✓
- Frame/render timing signal + GC/heap signal → Tasks 5, 6, 8. ✓
- Self-describing `/` → Task 3. ✓
- Enablement flag `--diagnostics-port` → Task 8. ✓
- **Deferred to Phase 1b (documented, not gaps):** Lisp CPU capture, folded/SVG/`/report`, pprof, event-loop/input latency instrumentation, Chrome/Perfetto trace, `/diff`, native-layer tools. Input-latency and Lisp-capture both require core event-loop seams and get their own plan.

**Placeholder scan:** No TBD/TODO. The few "confirm the exact local name / field name" notes are verification instructions against real, mapped file:line anchors, not missing content — each has concrete fallback code.

**Type consistency:** `MetricsSnapshot`/`FrameMetrics`/`GcMetrics` field names are identical across Task 2 (definition), Task 3/4 (handlers), Task 8 (mapping). `GcStatsSnapshot` fields (Task 5) map 1:1 into `GcMetrics` (Task 8). `frame_metrics_snapshot()` (Task 6) feeds `FrameMetrics` (Task 8). `MetricsProvider`/`router`/`spawn`/`DiagnosticsConfig` signatures match between definition (Tasks 3, 7) and use (Task 8).
