# Display Audit 04 — Protocol & Integration (`neomacs-display-protocol`, `neomacs-bin`, `neovm-core` redisplay)

**Date**: 2026-07-02 · Part of the [display stack audit](2026-07-02-display-audit-00-overview.md).
**Scope**: the display data model (`neomacs-display-protocol`), the process/thread integration (`neomacs-bin`), redisplay triggering in `neovm-core`, the TTY interface, and the design-doc inventory.
**Note on crate names**: the Lisp engine crate is **`neovm-core`** (workspace `Cargo.toml:2-17`); `neovm-executor` is a separate member; a `neovm-engine/` directory exists but contains no crate (stale leftovers). The application binary is `neomacs-bin` (binary name `neomacs`). All struct sizes below are **measured** (probe crate against `neomacs-display-protocol`, x86-64, default features), not estimated.

---

## 1. End-to-end integration

```
                          ┌─────────────────────── ONE OS PROCESS ───────────────────────┐

 OS main thread                    "neomacs-evaluator" thread                 "input-bridge" thread
 ┌───────────────────┐             ┌──────────────────────────────────┐      ┌────────────────────┐
 │ winit event loop  │             │ neovm-core Context (LISP)        │      │ display InputEvent │
 │ + wgpu render loop│             │  recursive_edit → command_loop_1 │      │  → keyboard::      │
 │ run_render_loop_  │             │   read_key_seq → read_char       │      │    InputEvent      │
 │ current_thread    │             │    └ redisplay_for_input_wait()  │◄─────┤ convert_display_   │
 │ (main.rs:2110)    │             │       = redisplay_with_force(F)  │      │ event (drops       │
 │                   │             │       [coalesced by              │      │ releases/mods)     │
 │  RenderApp        │             │        RedisplaySignature]       │      └─────────┬──────────┘
 │  .poll_frame()    │             │       └ redisplay_fn(self)       │                │ input_tx
 │   drains frame_rx │             │           = publish_gui_frame    │                ▼
 │   materialize()   │             │             (main.rs:3450)       │        Context.input_rx
 │   per frame       │             │  for each frame in render tree:  │
 │   → FrameGlyphBuf │             │    layout_frame_display_state    │  ← neomacs-layout-engine
 │   set_current_    │             │      → LayoutEngine::            │    (the xdisp.c replacement)
 │   frame           │             │        layout_frame_rust         │
 │   render via wgpu │             │    → FrameDisplayState (GRID)    │
 └─────────▲─────────┘             │    frame_tx.try_send(state) ─────┼──┐
           │ frame_rx              │    render_waker.wake()           │  │ frame_tx
           │ (UNBOUNDED crossbeam) └──────────────────────────────────┘  │ (moved Rust value,
           └───────────────────────────────────────────────────────────◄─┘  NO serialization)

 TTY MODE (no render thread): the evaluator thread runs the SAME layout_frame_display_state,
 then run_tty_rif_redisplay → TtyRif::rasterize(FrameDisplayState) → diff_and_render() → ANSI.
```

One line: **`neovm-core` (Lisp) → `redisplay_fn` closure → `neomacs-layout-engine` (per window) → `FrameDisplayState` grid → crossbeam channel (moved) → render thread `materialize()` → `FrameGlyphBuffer` → `neomacs-renderer-wgpu`.** TTY branches off after `FrameDisplayState` through `TtyRif` on the same thread.

## 2. The protocol carries three display models (plus one dormant)

| Model | Where | Role | Measured size |
|---|---|---|---|
| `FrameDisplayState` | `glyph_matrix.rs:657` | **Layout output**; grid-native; the value that crosses the channel | 1008 B header |
| `GlyphMatrix`/`GlyphRow`/`Glyph` | `glyph_matrix.rs:432/248/113` | grid cells inside `FrameDisplayState`; consumed directly by TTY | `Glyph` **56 B**, `GlyphRow` 152 B |
| `FrameGlyphBuffer`/`FrameGlyph` | `frame_glyphs.rs:770/132` | **render-thread materialization**; pixel-flat; consumed by wgpu | `FrameGlyph` **112 B** |
| `Scene`/`Node`/`NodeKind` | `scene.rs:226/58/9` | **dormant** retained scene graph | — |

Key facts:

- **`FrameDisplayState` is the wire model.** Grid-native, mirroring GNU `dispextern.h`: `window_matrices: Vec<WindowMatrixEntry>`, each holding a `GlyphMatrix` of `GlyphRow`s with `glyphs: [Vec<Glyph>; 3]` (left margin / text / right margin), plus non-grid side vectors (`backgrounds`, `borders`, `cursors`, `images`, `videos`, `xwidgets`, `scroll_bars`, `face_fills`) and `faces: HashMap<u32, Face>` (`Face` = **248 B** measured).
- **`FrameGlyphBuffer` is the render model**, produced on the render thread by `FrameDisplayState::materialize()` (`glyph_matrix.rs:1074`, called at `frame_ingest.rs:273`). `FrameGlyph` (`frame_glyphs.rs:132`) is a **10-variant enum measured at 112 bytes, align 8** — Rust sizes an enum at its largest variant, so **every entry, including a plain ASCII `Char`, occupies 112 B**. The `Char` variant carries `window_id: DisplayWindowId(i64)`, `row_role`, `clip_rect: Option<Rect>`, `slot_id`, `bidi_level: u8`, `char`, `composed: Option<Box<str>>`, `x/y/baseline/width/height/ascent: f32`, `face_id: u32`. Colors/decorations were deliberately denormalized **off** the glyph (resolved from `faces` by `face_id` at draw time, `frame_glyphs.rs:164-173`) — the 112 B is a known tradeoff, but it is unmonitored: **there is not a single `size_of` assertion or budget test in the protocol crate**. A modest 120×40 screen ≈ 4800 glyphs ≈ ~0.5 MB rebuilt per materialize; 200×60 ≈ ~1.1 MB — on top of the ~56 B/glyph grid it was converted from, i.e. the same content is materialized twice in two shapes every frame.
- **Snapshot vs diff — split by backend**: the GUI path is a full-frame snapshot with **no damage support** (struct doc says it outright: *"cleared and rebuilt from scratch each frame … No incremental state management needed"*, `frame_glyphs.rs:766-768`; no previous-frame/damage field exists). The TTY path is diffed: `GlyphRow` carries an FNV-1a `hash` (`glyph_matrix.rs:252,337`) and `row_equal()` (`:383`); `TtyRif` keeps current/desired grids and `diff_and_render()` emits only changed cells (`tty_rif.rs:177-199,599`). **The diff machinery exists in the protocol; the GUI ignores it.**
- `materialize()` also deep-clones `faces` and `cursor_effects_by_window` into the buffer (`glyph_matrix.rs:1092,1140`) — the latter is a `HashMap<_, EffectsConfig>` where `EffectsConfig` = **3,576 B / 149 effect fields measured** (`effect_config.rs:1576`; a 45 KB source file with an 88 KB test file). An enormous cursor-effects configuration surface (galaxy, DNA-helix, candle-flame, lighthouse, …) rides inside the hot per-frame snapshot; it belongs on the command channel.

## 3. Process model — in-process threads, no serialization, no display IPC

- **Three threads in one process** (GUI mode, `neomacs-bin/src/main.rs`): the OS main thread runs winit+wgpu (`run_render_loop_current_thread`, `main.rs:2110`); the **"neomacs-evaluator"** thread (spawned with a large explicit stack, `main.rs:2155-2162`; body `run_gui_evaluator_worker` `:2191`; ends in `evaluator.recursive_edit()` `:2299`) runs the Lisp command loop **and layout**; the **"input-bridge"** thread (`main.rs:2248-2274`) converts display `InputEvent`s into keyboard events for the evaluator.
- **No serialization anywhere on the display path**: `FrameDisplayState` crosses the channel as a moved Rust value (`frame_tx.try_send`, `main.rs:3482`). A grep for `serde|bincode|postcard|rkyv|Serialize` across the three crates finds only `strum` enum-name parsing. The frame channel is **unbounded** (comment at `thread_comm.rs:773`: so `try_send` never drops frames); cmd/input are bounded. Consequence: no backpressure — a stalled renderer accumulates ~MB-scale frames without signaling the producer (mitigated in practice by coalescing, but the materialize cost is still paid per queued frame — runtime report §3).
- **The only client/IPC surface is emacsclient, not display**: `neomacsclient.rs` is an emacsclient clone (options nowait/eval/create-frame/tty/socket-name/server-file; `EMACS_VERSION = "31.0.50"`) over `UnixStream`/`TcpStream` (`:220,249`) for elisp eval / frame creation — the server.el model. There is **no remote-display/multi-client display design** and no protocol versioning (the word "compatibility" in this stack means GNU behavioral fidelity, not a wire version).
- Startup order: pick GUI/TTY → spawn evaluator worker (`spawn_gui_evaluator_worker`, `main.rs:2141`) → evaluator bootstraps buffers, installs a `PrimaryWindowDisplayHost` (`:2221`), spawns the input bridge, installs `redisplay_fn` (`:2286`), publishes an initial frame (`:2289`), enters `recursive_edit()`; meanwhile the main thread runs the render loop.
- Feature flags: protocol `neo-term` (gates `FrameGlyph::Terminal` + `TtyCell`); binary `video`, `jit`, `mimalloc` (default), `neo-term`, `wpe-webkit` (`neomacs-bin/Cargo.toml:11-26`).

## 4. Redisplay triggering in `neovm-core` — coalesced, GNU-style, on the Lisp thread

- Entry: `Context::redisplay()` (`eval.rs:6747`) → `redisplay_with_force(false)` (`:6795`).
- **Not per-command**: there is no redisplay call at the tail of `command_loop_1` (`eval.rs:5922-6328`). Redisplay runs **before `read_char` blocks** for the next key — `redisplay_for_input_wait()` at `keyboard.rs:4030` — exactly GNU's placement. Other triggers: resize (`keyboard.rs:3747`), `sit-for`/`accept-process-output` waits (`wait.rs:830-835`), the `(redisplay)` builtin (`builtins/symbols.rs:1901`). Idle timers are serviced inside the same input-wait loop; there is no separate redisplay timer.
- **Coalescing**: `RedisplaySignature` (buffer/overlay/text-property ticks, point, geometry) early-returns "visible state unchanged" unless forced (`eval.rs:6836-6848`) — a command that changes nothing visible produces no frame. This is the *only* incrementality in the stack today.
- **Blocking**: the `redisplay_fn` closure runs inline as `f(self)` (`eval.rs:6865-6869`); `publish_gui_frame` → per-frame `layout_frame_display_state` (`main.rs:3471-3482`) executes on the evaluator's stack. **Layout blocks Lisp**; only GPU rasterization/present is off-thread.

## 5. TTY interface (`tty_rif.rs`) — the half of GNU redisplay the GUI is missing

`TtyRif` (`tty_rif.rs:177`) is a GNU `term.c`-style backend: two `TtyGrid`s (current/desired) of `TtyCell{ch, attrs, padding, extenders}` (`:60,86`); `rasterize(&FrameDisplayState)` (`:283`) fills desired from the grid; `diff_and_render()` (`:599`) diffs and emits ANSI SGR; `take_output()` (`:690`) returns bytes. Runs on the evaluator thread.

**GUI and TTY share one layout engine** — both call `tty_layout::layout_frame_display_state` → `LayoutEngine::layout_frame_rust` (GUI via `publish_gui_frame`; TTY via `run_tty_layout_tree`). They diverge only at output: GUI `materialize()`→wgpu (full rebuild); TTY `TtyRif`→ANSI (row-hash diff). This matches the pivotal 2026-04-04 design decision (*"shared representation is a character grid … TTY and GUI diverge at the RedisplayInterface"*) — except the GUI side never got its diff. Note the file name: `tty_layout.rs` is the **shared** layout entry point, GUI included.

## 6. Animation ownership — hints from layout, state machines on the render side

- The protocol crate defines the vocabulary only: spring/easing math (`scroll_animation.rs`: `SpringState:334`, `PerLineSpringState:386`, `ScrollEffect:34`), `EffectsConfig` (`effect_config.rs:1576`), `TransitionPolicy` (`transition_policy.rs:11`).
- The layout engine emits **declarative hints per frame**: `transition_hints: Vec<WindowTransitionHint>`, `effect_hints: Vec<WindowEffectHint>`, `cursor_effects_by_window` (`display_output_frame_state.rs:29-33`). It never ticks a state machine.
- The render side owns and advances all animation state (`render_thread/transitions.rs`, `render_thread/frame_windows.rs`, renderer `transitions.rs`). Clean separation; the cost problem is only that the 3.5 KB config rides the per-frame payload (§2).

## 7. `scene.rs` — the abandoned "right" design

`scene.rs:226` is a genuine retained-mode scene graph: `Node` tree, per-node `Transform`/opacity/clip, and **dirty-region damage tracking** (`dirty: Option<Rect>`, `mark_dirty`/`mark_region_dirty`, `:241,318,323`). It is dormant:

- `Scene::build_window_node` stops at `// TODO: Build text nodes from glyph rows` (`scene.rs:504`) — it never emits text.
- The wgpu renderer exposes `render(&Scene)`/`render_to_view`/`render_to_texture` (`renderer/mod.rs:1656,1688,1865`) and the `DisplayBackend` trait's `render(&mut self, scene: &Scene)` (`backend/mod.rs:20`) all take `Scene` — **none are called on the live path** (the live path is `render_pass.rs`, whose methods take `&FrameGlyphBuffer`).

So the crate ships two parallel vocabularies for the same concepts (grid item structs `BorderItem`/`CursorItem`/`ImageItem`/… mirroring `FrameGlyph` variants) plus a third, unfinished retained model. The deletion plan doc corroborates the bloat ("70→81 modules"). Decision needed (roadmap Phase 4): the pragmatic path is row-hash damage on the existing grid — delete `scene.rs` rather than finish it.

## 8. Stale artifacts that actively mislead

- `frame_glyphs.rs:1-5,:767-768` still describe a **C-side matrix walker** ("extracts ALL visible glyphs from Emacs's current_matrix") and `transition_policy.rs:32` references "C→Rust FFI" indices — the C backend no longer exists; the data source is the Rust layout engine + render-thread materialize.
- `docs/rust-display-engine.md` (the 1400-line origin vision) still assumes a C core serializing a `LayoutSnapshot` over FFI (`neomacsterm.c`, `neomacs_build_layout_snapshot()`). Useful as GNU reference; wrong on wiring.
- Sentinel-typed identity: `emacs_frame_id: 0` = "primary/root" recurs (`types.rs:29,260`); `DisplayWindowId` is a bare `i64` from a pointer.
- MEMORY/docs elsewhere referring to a `neovm-engine` crate are stale (see the note at top).

## 9. Design-doc inventory (current vs stale)

**Current / describes what shipped:**
- `docs/plans/2026-06-13-display-pipeline-refactor-handoff.md` — authoritative pipeline shape (`DisplayItemSource → DisplayItem → DisplayRowRenderer → RenderedDisplayRow → installer → renderer`); "SPINE COMPLETE".
- `docs/plans/2026-06-22-pipeline-completion-proof.md` — one shared row center; layout suite 1250/1250.
- `docs/plans/2026-06-21-display-pipeline-deletion-plan.md` — health diagnosis: +2245 lines / 70→81 modules during the refactor; names `display_iterator.rs` (and a since-deleted `bidi_layout.rs`) as dead scaffolding.
- `docs/superpowers/specs/2026-04-04-display-pipeline-refactor-design.md` — the grid-not-pixels decision; matches what shipped.
- `docs/plans/2026-02-04-two-thread-architecture-design.md`, `2026-04-26-gui-main-thread-evaluator-worker-design.md` — the thread model; matches `main.rs`.
- `docs/render-thread-architecture-plan.md` (Draft) — accurately describes current god-objects and the flat ~70-variant `RenderCommand`; its `FrameCompositor`/`FrameRef` proposals are **not** landed.
- `docs/plans/2026-06-08-cursor-architecture-design.md` — names the forcing function (GNU shares one matrix by pointer; neomacs must copy across threads) and concedes the current state is *"the worst of both"* (immediate rebuild + retained caches); proposes `Arc<FrameMatrix>`. Partly stale file refs.

**Stale / superseded:** `docs/rust-display-engine.md` (C-core wiring); `docs/plans/2026-04-11-display-engine-unification*.md` (its `struct It` port landed dormant, slated for deletion); `docs/plans/2026-06-08-display-row-source-unification.md` (its central `DisplayRowSpec` was deleted).

## 10. Summary of protocol/integration debt

1. Full-frame materialization per redisplay — the same content held/rebuilt in two shapes (56 B grid cell → 112 B flat glyph), with faces + 3.5 KB-per-window effects config cloned along; entirely unmonitored (no size/budget tests).
2. Row-hash diff machinery present but GUI-unused.
3. Layout on the Lisp thread (blocking), serial per window.
4. Unbounded frame channel + materialize-per-queued-frame = no backpressure and non-coalesced conversion work.
5. Three-plus display vocabularies (grid, flat, scene, item structs) for one concept space; scene graph abandoned mid-build.
6. Stale C-era comments and doc claims; sentinel ids.
7. No wire versioning (fine in-process; becomes real work if remote display is ever wanted — nothing is reserved for it).
