# Neomacs Profiler Toolchain: Cross-Layer Performance Introspection over HTTP

## Problem

Investigating neomacs performance today means attaching external native
profilers (perf, samply) to the process. Those tools see the Rust binary — the
bytecode dispatch loop, native allocations, `Evaluator::eval_form` — but they
are **blind to the Lisp/application layer**. They cannot answer the questions
that actually matter: *which elisp `defun` is hot, why does typing feel laggy,
which redisplay path blew the frame budget, how much did my change help.*

Neomacs already has the raw material — a Lisp sampling profiler (`profiler.el`
port, CPU + memory), a render-side profiler, frame timing (`FrameCoordinator`),
and GC/heap counters (neovm-gc) — but there is no way to *get at it* from
outside a running instance, and no unified way to analyze it. In particular,
an **AI agent** that is asked to "investigate performance" has no surface it can
query.

## Approach

Build a **cross-layer, multi-audience performance investigation toolchain**: a
single in-process capture engine spanning the Lisp **and** native layers, driven
from either Lisp or HTTP, that projects one capture into three consumers — an
**AI agent** (structured JSON + pprof CLI), a **human in a browser**
(flamegraph / timeline), and a **human in the editor** (native
`profiler-report` buffer). On top of the raw data it adds server-side ranking,
distributions, A/B diffing, and cross-layer correlation.

This is not a metrics port. It is an investigation engine with the **AI agent
as a first-class consumer**.

The HTTP server is delivered on `axum`/`tokio` running as a **contained
IO-reactor edge** — see "tokio boundary" below. This deliberately doubles as
neomacs's first, low-risk adoption of tokio, piloting the channel-bridge
pattern that a future async-IO subsystem (network processes, LSP transports,
`url.el`) will reuse.

### Non-goals (this spec)

- No always-on/remote monitoring, auth beyond loopback binding, or TLS. The
  endpoint is a localhost developer/agent tool, **off by default**.
- No reimplementation of native profilers (perf/DHAT/heaptrack). Those are
  *documented and interoperated with*, not rebuilt.
- No replacement of the native Emacs `profiler-report` buffer — the HTTP
  surface is *additive*.
- tokio does **not** enter the core VM. It lives behind channels (see below).

## Prior art (validates the shape)

- **GNU Emacs** — `profiler.el` (SIGPROF sampling, CPU+mem, tree buffer) +
  `elp.el` (deterministic, per-function counts) + `benchmark.el`.
- **SBCL** — `sb-sprof` (sampling, call-graph, `:alloc` mode) + `sb-profile`
  (deterministic); `cl-flamegraph` wraps it to folded → FlameGraph.
- **Clojure `clj-async-profiler`** — the direct architectural precedent: an
  in-process profiler that dumps **collapsed/folded stacks** and **serves an
  interactive flamegraph over a local HTTP server** (`serve-ui 8080`), plus
  **diffgraphs** comparing two captures. Our design is this, plus agent-JSON and
  pprof tiers.
- **Racket `profile` / Guile `statprof`** — statistical samplers reporting
  self/total (cumulative).

Universal patterns adopted: **self vs total** as the reporting vocabulary;
**CPU + allocation** always paired; **folded → flamegraph** as interchange;
**both a sampling and a deterministic mode**; **serving profiling over HTTP is
a proven design**, not a novel risk.

## Architecture — five layers

```
┌─ Capture ──────────────────────────────────────────────────────────┐
│  Lisp layer:  profiler.el sampling (CPU+mem) · elp.el deterministic │
│               · frame timing · GC/heap · event-loop/input latency   │
│  Native layer: pprof-rs (CPU) · jemalloc-prof / dhat (heap)         │
└────────────────────────────────────────────────────────────────────┘
        │ two primitives: aggregated stack-samples  |  timeline/counters
        ▼
┌─ Control ──────────────────────────────────────────────────────────┐
│  In-editor/Lisp: M-x profiler-*, elp-instrument-*                   │
│  HTTP: POST /capture?signal=&secs=  (drives Lisp profiler via chan) │
└────────────────────────────────────────────────────────────────────┘
        ▼
┌─ Transport ── axum on dedicated tokio current-thread runtime ───────┐
│  own OS thread · 127.0.0.1 · off by default · channel-bridged       │
└────────────────────────────────────────────────────────────────────┘
        ▼
┌─ Projection (one capture → three audiences) ───────────────────────┐
│  Agent:  /report /profile?top=N&sort= /callers /metrics            │
│          /trace/summary /profile/*.pprof /diff  · self-describing / │
│  Browser: *.svg (inferno) · *.folded (speedscope) · trace/*.json    │
│           (Perfetto) · /live (SSE)                                  │
│  Editor:  native profiler-report buffer + elp tables               │
└────────────────────────────────────────────────────────────────────┘
        ▼
┌─ Intelligence ─────────────────────────────────────────────────────┐
│  server-side ranking · distributions (p50/95/99) · A/B diff ·      │
│  ★ cross-layer correlation on a shared clock                        │
└────────────────────────────────────────────────────────────────────┘
```

### The tokio boundary (hard rule)

The Lisp VM (`neovm-core`) stays **single-threaded and synchronous**, exactly
like GNU Emacs. tokio/axum run on a **dedicated OS thread with a current-thread
runtime**. The diagnostics thread never touches VM state directly; it
communicates over channels, identical to the existing layout-thread ↔
render-thread split:

- **Metrics/counters** (frame timing, GC, latency): producers push samples into
  lock-light shared registries (`Arc<...>` snapshots or an MPSC drain). The HTTP
  thread reads snapshots. No VM involvement.
- **Lisp captures** (`profiler.el`, `elp`): must run *on the Lisp thread*
  (`profiler_cpu_start/stop`, `profiler_cpu_log`). The HTTP thread sends a
  request over a channel; the Lisp event loop services it at a safe point and
  replies with the capture. tokio only ever sees the reply on the channel.

tokio must never color `neovm-core` APIs with `async`/`await`.

### Two data shapes → one model each

- **Aggregated stack samples**: `stack → {self, total}` counts. Source: Lisp
  CPU/mem profiler, render profiler, pprof-rs. Flamegraph-shaped; no time axis.
- **Timeline events / counters**: `span{start,dur}` and `counter(t)`. Source:
  frame timing, GC pauses, event-loop/input latency, heap-size/FPS over time.
  Perfetto/Chrome-trace-shaped.

Every projection is a rendering of one of these two models.

## Signals (all four)

1. **Lisp profiler (CPU + memory)** — existing `profiler.el` port; sampling.
   Plus a **deterministic `elp.el` mode** (per-function exact counts) exposed
   through the same surface (`?mode=deterministic`). *Known gap to close.*
2. **Frame / render timing** — `FrameCoordinator`: FPS, frame-build time, GPU
   render time, dropped/coalesced frames, damage. Primary live-feed source.
3. **GC / heap memory** — neovm-gc: heap size, live cons/vector/string counts,
   GC pause durations/frequency, allocation rate.
4. **Event-loop / input latency** — keypress→redisplay latency, command
   execution time, input queue depth. The "why does it feel laggy" signal.

## Surfaces

### Agent (primary) — structured, ranked, token-bounded JSON

- `GET /` — self-describing: lists endpoints + schema so an agent navigates with
  zero prior knowledge.
- `GET /report` — the front door. One call: ranked digest across all four
  signals (top CPU hotspots self%/total%/samples, top allocators, GC
  count/total/p95/p99, frames over budget, worst input latencies). Distributions,
  not averages. Token-bounded by default.
- `GET /profile/lisp?top=N&sort=self|total&mode=sampling|deterministic` — top-N
  hotspot rows.
- `GET /profile/lisp/callers?fn=NAME` — drill-down.
- `GET /metrics` — current numeric snapshot (heap, fps, gc, latency) flat JSON.
- `GET /trace/summary?window=5s` — aggregated timeline stats (frame p50/95/99,
  GC histogram, worst latencies) instead of a multi-MB raw trace.
- `GET /profile/{lisp,native}.pprof` — pprof protobuf for `go tool pprof` CLI.
- `GET /diff?before=A&after=B` — regression comparison of two stored captures.
- `POST /capture?signal=&secs=` — start/collect a capture.

Every GET is token-bounded by default — a request never returns 50k lines
unless explicitly asked (`top=all`).

### Human (browser) — rendering projections of the same capture

- `GET /profile/lisp.svg` — inferno-rendered flamegraph, zero external tooling.
- `GET /profile/lisp.folded` — collapsed stacks → speedscope.
- `GET /trace/timeline.json` — full Chrome/Perfetto trace over a capture window.
- `GET /live` — SSE feed of frame/GC/latency counters, watch in real time.

### Human (editor) — unique to neomacs

Route the same capture into the native `profiler-report` buffer and `elp`
tables. No other Lisp offers all four surfaces from one capture; this is the
toolchain's differentiator.

## Capture formats & the pprof unifier

| Shape | Format | Consumer |
|-------|--------|----------|
| Aggregated | **folded stacks** (primitive) | speedscope; `inferno` → SVG (a *view*, not a separate format) |
| Aggregated | **pprof protobuf** | `go tool pprof` CLI (agent power analysis); speedscope |
| Aggregated | **ranked JSON** | agent `/report`, `/profile` |
| Timeline | **Chrome/Perfetto JSON** | Perfetto UI (human) |
| Timeline | **summary JSON** | agent `/trace/summary`, `/live` |

**The unifier:** the hand-built Lisp pprof, `pprof-rs` (native CPU), and
jemalloc-prof (native heap) **all emit pprof** → `go tool pprof` is the *one*
CLI an agent uses across Lisp CPU, native CPU, and native heap.

**Dropped:** server-side SVG collapses to an inferno view of folded (not a
separate path). Firefox-Profiler format (`fxprof-processed-profile`) is a
possible future human option but not required — Perfetto covers the timeline.

## Native layer

No single native tool does CPU + memory; use a pair, split in-process vs
external:

- **CPU, in-process:** `pprof` / pprof-rs → `/profile/native.svg|.pprof`. Lets
  the agent pivot from Lisp to native when `/report` shows time is in native
  code (GC/layout/render), which the Lisp poll-sampler structurally cannot see.
- **Memory, in-process:** `tikv-jemallocator` `profiling` feature as the
  always-shippable option (jemalloc as allocator = perf win; prof toggled at
  runtime, near-zero overhead off; emits pprof). `dhat` behind a
  `--features dhat-heap` profiling build for deep allocation-site dives.
- **External, documented (zero code):** `samply` → Firefox Profiler and
  `cargo flamegraph` (CPU); `heaptrack` (mem); `iai-callgrind` (deterministic
  CI regression).

**SIGPROF finding (verified, `neovm-core/.../profiler.rs:281-290`):** the Lisp
CPU profiler is **not** signal-based — `profiler_cpu_start` records
`thread_cpu_time_ns()` and samples the Lisp backtrace cooperatively at eval
safe-points when a CPU-time budget elapses. The only SIGPROF in the tree is
subprocess signal handling. Therefore:

1. **No contention** — pprof-rs (SIGPROF) can run concurrently with the Lisp
   profiler (cooperative poll). Both capturable over one window → cross-layer
   correlation is implementable, not aspirational.
2. **Complementary coverage** — the Lisp sampler advances only at eval
   safe-points, so it cannot attribute pure-native time (render/GC/layout);
   pprof-rs fills exactly that blind spot.

**Caveats:** the global allocator is a singleton (jemalloc-prof and dhat cannot
both be active — ship jemalloc, keep dhat a separate build). Native layer is
strongest on Linux (`pprof-rs` Unix-strong, jemalloc/heaptrack Linux-centric;
`samply` is cross-platform).

## The intelligence layer

- **Server-side pre-aggregation** — agents never parse raw traces.
- **Distributions, not averages** — the Rust Performance Book's variance caution
  (wall-time is high-variance); `/report` emits p50/p95/p99 + sample counts.
- **A/B diff** — clj-async-profiler-style diffgraphs; "did my change help." The
  single most valuable op for an investigating agent.
- **★ Cross-layer correlation on a shared clock** — the reason to build this.
  Because neomacs owns the frame timeline *and* the Lisp samples on one clock,
  `/report` can say: *"the 40ms frame at t=3.2s was 80% in
  `redisplay → jit-lock → font-lock-fontify-region`."* Native profilers
  structurally cannot produce that Lisp-attributed, timeline-correlated
  statement.

## Crate structure & dependencies

New workspace crate **`neomacs-diagnostics`**:

- Owns the axum server, tokio runtime thread, projection/rendering, and the
  channel protocol to the Lisp thread and metric producers.
- Depends on: `axum` + `tokio` (current-thread), `serde`/`serde_json`,
  `inferno` (folded → SVG), a small pprof encoder (`prost` + pprof proto).
- Wired into `neomacs-bin` behind a flag; enabled off by default.

Dependency maturity (verified 2026-07-14, per the "recent + mature" bar):

| Crate | Latest | Downloads | Role |
|-------|--------|-----------|------|
| `axum` | 0.8.9 (Apr 2026) | 25M+ | HTTP + native SSE |
| `tokio` | current | — | reactor (edge only) |
| `inferno` | 0.12.7 (Jul 2026) | 45M | folded → SVG, pure Rust |
| `pprof` (pprof-rs) | 0.15.0 (May 2025) | 42M | native CPU, pprof out |
| `tikv-jemallocator` | 0.7.0 (May 2026) | 80M | native heap prof (pprof) |
| `dhat` | 0.3.3 (Feb 2024) | 9M | deep native heap (build) |

## Enablement & security

- **Off by default.** Enabled by a CLI flag (e.g. `--diagnostics-port N`) and/or
  a Lisp variable + `M-x` command, mirroring `server-start`.
- **Bind `127.0.0.1` only.** No external exposure, no auth beyond loopback in v1.
- Capture control (`POST /capture`) is the only state-changing surface;
  everything else is read-only.

## Phased roadmap

1. **MVP / tokio pilot** — `neomacs-diagnostics` crate; tokio-edge thread +
   channel bridge; `/metrics` + `/live` SSE; `/report` + `.folded` + inferno
   SVG for Lisp CPU. Immediately useful; proves the tokio pilot end-to-end.
2. **Agent power** — pprof export (Lisp) · `/profile?top/sort` · `/callers` ·
   `/trace/summary` · self-describing `/` · deterministic `elp` mode.
3. **Correlation + diff** — shared-clock cross-layer correlation in `/report` ·
   `/diff` · native `profiler-report` buffer integration.
4. **Native layer + polish** — pprof-rs (`/profile/native.*`), jemalloc-prof,
   documented `samply`/`dhat`/`heaptrack` flow; budgets/alerts (frame >16ms,
   GC >Nms). Optional Firefox-Profiler format.

## Open questions / risks

- **Capture storage for `/diff`** — where do the two named captures live
  (in-memory ring, temp files)? Decide in Phase 3.
- **Latency instrumentation points** — event-loop/input latency needs new
  timestamps at keypress ingest and redisplay completion; confirm the seams in
  the input bridge and frame coordinator.
- **Shared clock** — cross-layer correlation requires the render thread, Lisp
  thread, and GC to stamp events against one monotonic clock; verify a common
  `Instant` origin is threadable across the boundaries.
- **jemalloc as default allocator** — is switching the global allocator to
  jemalloc acceptable project-wide, or gated behind a feature? Affects Phase 4.
