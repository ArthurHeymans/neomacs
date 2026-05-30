# Render Thread Architecture Modernization Plan

## Date: 2026-05-30

## Status: Draft

---

## 0. Architecture Audit (Self-Review)

### What's Wrong With This Plan

The plan below is v1. After critical review, three issues were found:

**Issue A: TransitionData and EffectData are single-field wrappers (Phase 1c)**

These violate the plan's own principle: _"a sub-struct must have methods, not just fields."_
`TransitionData` wraps a single `Vec<TransitionState>`. The tick logic already lives in
`transitions.rs` as free functions. Wrapping it in a struct adds indirection without
meaningful behavior. Same for `EffectData`: it's just `Vec<RendererEffect>` with no
cohesive behavior beyond what `render_pass.rs` and `lifecycle.rs` already do.

**Fix**: Drop Phase 1c entirely. Merge transitions+effects into a new `FrameCompositor`
struct (see Issue B) — transitions and effects are compositor concerns (they modify
what's drawn), so they belong with glyph atlas, frame buffer, and dirty tracking.

**Issue B: The 9 remaining fields are an orphaned grab-bag**

`current_frame`, `child_frames`, `glyph_atlas`, `frame_dirty`, `visual_cursors`,
`renderer_effects`, and `transitions` are all about **glyph composition and rendering**.
They are the compositor. Leaving them flat on `GuiFrameRenderState` misses the largest
opportunity — every render-side module imports `GuiFrameRenderState` for at least one
of these fields.

**Fix**: Extract `FrameCompositor` — groups all rendering-related state with coherent
behavior (ingest, mark_dirty, tick transitions, update effects). This narrows the
import surface: `frame_ingest.rs` imports only `FrameCompositor`, `transitions.rs`
imports only the transitions slice, `render_pass.rs` imports `FrameCompositor` +
chrome + overlays.

**Issue C: GuiFrameWindowManager's 22 bulk-iterator methods are identical boileplate**

All 22 follow the same pattern: iterate `windows.values_mut()`, filter by
`is_top_level()`, apply a closure. Examples: `for_each_top_level_window_mut`,
`mark_top_level_dirty`, `hide_top_level_popup_menus`, `clear_top_level_tooltips`, etc.
Each is 3-5 lines, and they clutter the manager's API surface.

**Fix**: Add one generic `for_each_top_level_mut(f: impl FnMut(&mut GuiFrameWindowState))`
method. The 22 bulk methods become call sites that pass a closure. Net reduction: ~80 lines.
This is Phase 0 (do it first, it's mechanical and safe).

**Issue D (minor): Single-channel RenderCommand argument is correct but incomplete**

The plan says _"`CreateWindow` must happen before `ResizeWindow`"_ — these are within
the same domain (WindowCommand), so the intra-domain ordering argument holds regardless
of channel topology. Cross-domain ordering doesn't matter (TerminalCreate doesn't need
to precede or follow a ConfigCommand). The real reason to keep a single channel is
**simplicity**: one sender to pass around, one receiver to drain, one drain loop.

**Issue E: `emacs_frame_id: 0` sentinel violates GNU Emacs convention**

GNU Emacs (`frame.c:343`): `frame_next_id = 1` — IDs start at 1. `id == 0` means
"not yet assigned." There is no "primary" concept at the ID level — the selected frame
is tracked separately.

Neomacs currently uses `PRIMARY_PENDING_KEY = 0` as a HashMap key for the primary frame
and `emacs_frame_id: 0` in RenderCommands to mean "route to primary." The producer
(`neomacs-bin/src/main.rs:1018-1020`) translates:

```rust
let emacs_frame_id = if self.primary_frame_id == Some(frame_id) {
    0  // WRONG: 0 means "unassigned" in GNU Emacs, not "primary"
} else {
    frame_id.0
};
```

This creates a fragile magic-number contract. It violates GNU Emacs's convention.
It means `is_primary_frame_id()` must special-case `id == 0` (line 1021). And it
makes the HashMap unsafe — any command accidentally using frame ID 0 silently targets
the primary window.

**Fix**: Replace `u64` frame IDs with typed enums:

```rust
/// Frame reference in render commands.
/// Replaces raw u64 emacs_frame_id — no sentinel values.
/// Matches GNU Emacs convention: 0 is never a valid frame ID.
pub enum FrameRef {
    /// Route to the primary frame (resolved at render-time).
    /// Used by commands that predate per-frame addressing.
    Primary,
    /// Route to a specific frame by its Emacs-assigned ID.
    /// Matches GNU Emacs frame.id (EMACS_UINT, sequential from FRAME_ID_BASE).
    Frame(u64),
}

/// Key for GuiFrameWindowManager.windows HashMap.
#[derive(Hash, Eq, PartialEq)]
pub enum FrameKey {
    /// Primary frame before Emacs assigns a real ID (bootstrap only).
    Pending,
    /// Frame with a real Emacs-assigned ID (primary after adoption, all secondaries).
    Adopted(u64),
}
```

| Before (sentinel) | After (typed) |
|---|---|
| `emacs_frame_id: 0` in commands | `frame: FrameRef::Primary` |
| `emacs_frame_id: frame_id.0` | `frame: FrameRef::Frame(frame_id.0)` |
| `PRIMARY_PENDING_KEY = 0` in HashMap | `FrameKey::Pending` |
| `primary_emacs_frame_id.unwrap_or(0)` | match on `FrameKey::Pending` / `FrameKey::Adopted(id)` |
| `is_primary_frame_id(id)` checks `id == 0` | `matches!(frame, FrameRef::Primary)` |
| Producer translates primary → 0 | Producer sends `FrameRef::Primary` or `FrameRef::Frame(id)` |

This is folded into Phase 0 (FrameKey) and Phase 3 (FrameRef in commands).

### Revised Phase Structure

| Phase | Description | Why First/Next |
|-------|-------------|----------------|
| 0 | `FrameKey` enum + collapse 22 bulk methods into generic + closures | Eliminates sentinel-0 in HashMap, clears clutter |
| 1 | Extract `ChromeState`, `OverlayState`, `FrameCompositor` from `GuiFrameRenderState` | Foundational — everything else builds on this |
| 2 | Add hit-testing APIs, abstract `pointer_events.rs` | Uses Phase 1 sub-structs |
| 3 | `FrameRef` enum in commands + delete 44 `primary_*` delegates | Eliminates sentinel-0 in protocol, cleans API surface |
| 4 | Split `RenderCommand` into domain sub-enums | Last — touches most files, highest risk |

---

## 1. Problem Statement

The render thread has accumulated technical debt around four architectural deficiencies:

### 1.1 God Objects

| Struct | Fields | Methods | File |
|--------|--------|---------|------|
| `RenderApp` | 23-28 | 52 | `state.rs` |
| `GuiFrameRenderState` | 22-23 | 44 | `frame_windows.rs` |
| `GuiFrameWindowManager` | 8 | 62 | `frame_windows.rs` |

`GuiFrameRenderState` absorbs all per-frame visual state: glyph data, chrome overlays,
cursors, transitions, effects, FPS overlay, typing-speed overlay, idle dim, IME preedit,
and floating WebKit views. It has no internal decomposition — every module that touches
any visual state reaches into the same flat struct.

`RenderApp` has 52 methods, of which 44 are thin `self.frame_windows.primary_window().map(...)`
delegates that exist only for backward compatibility with the pre-unification architecture.

### 1.2 Encapsulation Breach in pointer_events.rs

`pointer_events.rs` makes ~90 direct `window_state.render.*` field accesses, bypassing
`GuiFrameRenderState`'s setter methods. The input system is tightly coupled to the render
state's exact field layout. Any change to chrome, popup, tooltip, or overlay fields
requires coordinated changes across pointer_events.rs and frame_windows.rs.

### 1.3 Legacy primary_* Delegate Methods

After the HashMap unification (commit `0b3269149`), primary and secondary frames share
the same `GuiFrameWindowState` type in the same `windows` HashMap. However, 44 `primary_*`
delegate methods still exist on `RenderApp`, creating a parallel API surface:

- Callers must choose between `app.primary_render_state_mut()` (primary-only) and
  `app.frame_windows.get_mut(emacs_frame_id)` (universal).
- Some command handlers still call `self.primary_*()` because the command lacks
  `emacs_frame_id` (legacy commands like `SetWindowTitle` vs `SetFrameWindowTitle`).

### 1.4 70-Variant RenderCommand Enum

`RenderCommand` in `thread_comm.rs` is a flat enum with ~70 variants spanning six
unrelated domains (lifecycle, window management, images, WebKit, terminal, UI overlays,
config). The dispatch chain uses four fallthrough methods:

```
handle_asset_command() → handle_window_command() → handle_terminal_command() → handle_ui_command()
```

Each returns `Result<(), RenderCommand>` — if `Err(cmd)`, it passes to the next handler.
This makes it non-obvious which handler owns which variant without reading all four files.

---

## 2. Research Findings

### 2.1 Component Pattern (Game Programming Patterns — Robert Nystrom)

The component pattern decomposes monolithic objects along domain boundaries. Each component
owns its data and behavior. Key lessons:

- **Don't over-decompose**: Group fields that are always accessed together. A sub-struct
  with no methods is namespace pollution, not architecture.
- **Communication via shared state**: Components can communicate through shared fields on
  the container object. This is simpler than messaging for small numbers of objects.
- **Composition over inheritance**: Use structs containing structs, not trait hierarchies.

Relevance: Our `GuiFrameRenderState` is the monolithic "Bjorn" class. We should decompose
into component structs that have their own behavior (methods), not just data.

### 2.2 Data-Oriented Design in Rust (kyren — RustConf 2018, Starbound architect)

Key lessons:

- Rust rewards data-oriented design with simple ownership semantics.
- OO inheritance hierarchies lead to "poking holes through interfaces" — exactly our
  `pointer_events.rs` problem, where 90 direct field accesses bypass encapsulation.
- The right pattern: group data that changes together, use indices not pointers, keep
  structs public when there's no invariant to protect.
- Don't add abstraction layers that don't carry their weight. If a sub-struct has no
  behavior, it shouldn't exist.

Relevance: Our sub-structs must earn their existence by encapsulating behavior (hit testing,
timeout ticking, show/hide lifecycle), not just grouping fields.

### 2.3 ECS Back and Forth (skypjack)

ECS (Entity Component System) is for thousands of entities with variable component
configurations. Key lessons:

- For a small fixed set of objects, simple composition structs outperform ECS.
- The right granularity is determined by access patterns, not theoretical purity.
- "The main reason to use ECS is code organization, not performance" — for small N,
  composition achieves the same organization without ECS infrastructure.

Relevance: We have ~10 frame windows, not 10,000 game entities. Full ECS (entities as IDs,
components in sparse arrays) is overkill. Composition structs with behavior is the right level.

### 2.4 Practical Rust Architecture: xilem, Bevy

- **xilem** (linebender): Uses a tree of `View` trait objects with message passing. Not
  directly applicable — our render loop is immediate-mode, not retained-mode reactive.
- **Bevy ECS**: Uses sparse-set storage with archetype-based iteration. Excellent for
  games with thousands of entities. Overkill for an editor with ~10 frames.

Relevance: Neither framework's architecture maps directly. Our pattern is closer to a
traditional game engine with a small number of high-level objects (frames/windows) —
composition structs with behavior is the proven pattern.

---

## 3. Design Decisions

### 3.1 What We Will Do

1. **Decompose `GuiFrameRenderState` into behavior-carrying sub-structs** — each sub-struct
   groups fields that are always accessed together AND has its own methods (hit testing,
   timeout ticking, rendering).

2. **Add chrome/overlay hit-testing APIs** — `pointer_events.rs` calls sub-struct methods
   instead of reaching into raw fields. The ~90 direct accesses become ~5 method calls.

3. **Delete 44 `primary_*` delegates** — migrate all callers to frame-addressed access via
   `self.frame_windows.get_mut(emacs_frame_id)`. Some legacy commands need `emacs_frame_id`
  added (protocol change).

4. **Split `RenderCommand` into domain sub-enums** — single channel preserved (ordering
   guaranteed), but the flat 70-variant enum becomes a 2-level structure with 6 sub-enums.

### 3.2 What We Will NOT Do

- **Full ECS**: Overkill for ~10 frames. Composition structs with behavior is sufficient.
- **Multiple channels**: Would break ordering guarantees between domains (e.g.,
  `CreateWindow` must happen before `ResizeWindow`).
- **Trait-object dispatch**: All command types are known at compile time. Virtual dispatch
  adds cost with no benefit.
- **Extract sub-crates**: The render thread is a single compilation unit for good reason
  (single-threaded ownership, no lock overhead). Sub-modules within the crate are sufficient.

### 3.3 Guiding Principles

1. **Behavior earns the decomposition** — a sub-struct must have methods, not just fields.
2. **Access patterns determine boundaries** — group fields accessed together in the same
   hot paths.
3. **Single-threaded ownership preserved** — no Mutex, no Arc, no channels between
   sub-structs within the same frame.
4. **Each commit compiles and passes tests** — incremental, verified slices.
5. **Net line count should decrease** — removing delegates and simplifying dispatch should
  more than offset the new struct definitions.

---

## 4. Implementation Plan

### Phase 0: Collapse GuiFrameWindowManager Bulk Methods (1 commit, ~-80 lines)

`GuiFrameWindowManager` has 22 `for_each_top_level_*`, `mark_top_level_*`, `hide_top_level_*`,
`clear_top_level_*`, `tick_top_level_*` methods. All follow the identical pattern:

```rust
for window_state in self.windows.values_mut() {
    if window_state.is_top_level() {
        window_state.render.some_action();
    }
}
```

Replace with a single generic method:

```rust
pub fn for_each_top_level_mut(&mut self, mut f: impl FnMut(&mut GuiFrameWindowState)) {
    for window_state in self.windows.values_mut() {
        if window_state.is_top_level() {
            f(window_state);
        }
    }
}
```

Each of the 22 bulk methods becomes a call site passing a closure. The logic moves
from the manager to the call site, where context makes the intent clearer:

```rust
// Before (on manager):
pub fn hide_top_level_popup_menus(&mut self) {
    for ws in self.windows.values_mut() {
        if ws.is_top_level() {
            ws.render.popup_menu = None;
        }
    }
}
// Call site:
self.frame_windows.hide_top_level_popup_menus();

// After (generic + closure at call site):
self.frame_windows.for_each_top_level_mut(|ws| {
    ws.render.overlays.hide_popup();
});
```

After this commit, only `for_each_top_level_mut` remains on the manager. All
specialized bulk methods are deleted. For read-only iteration, add a parallel
`for_each_top_level(&self, f)` if needed, or keep existing read-only bulk methods.

### Phase 1: Decompose GuiFrameRenderState (2 commits)

The revised decomposition eliminates the single-field wrappers and groups rendering
state into `FrameCompositor`:

#### 1a. ChromeState and OverlayState (first commit)

These are unchanged from v1 plan — they have the strongest behavioral justification.

```
ChromeState {
    menu_bar: Option<MenuChrome>,
    tool_bar: Option<ToolbarChrome>,
    compact_bar: Option<CompactBar>,
    chrome_interaction: ChromeInteraction,
}
```

Methods: `hit_test()`, `interacting()`, `set_menu_bar()`, `set_tool_bar()`,
`set_compact_bar()`, `clear_interaction()`.

```
OverlayState {
    popup_menu: Option<PopupMenuState>,
    tooltip: Option<TooltipState>,
    visual_bell_start: Option<Instant>,
    fps: FpsOverlayState,
    typing_speed: TypingSpeedState,
    idle_dim: IdleDimState,
    ime_preedit_active: bool,
    ime_preedit_text: String,
}
```

Methods: `popup_hit_test()`, `tooltip_hit_test()`, `show_popup()`, `hide_popup()`,
`show_tooltip()`, `hide_tooltip()`, `trigger_visual_bell()`, `visual_bell_active()`,
`tick_overlays()`, `set_ime_preedit()`.

Rationale: Chrome fields are always accessed together in `pointer_events.rs` for hit
testing and in `ui_commands.rs` for management. Overlay fields share a show/hide/timeout
lifecycle. Both groups have clear behavioral APIs, justifying their decomposition.

#### 1b. FrameCompositor (second commit)

Extracts ALL rendering-related state into one cohesive struct:

```
FrameCompositor {
    glyph_atlas: Option<WgpuGlyphAtlas>,
    current_frame: Option<FrameGlyphBuffer>,
    child_frames: Vec<ChildFrameEntry>,
    frame_dirty: bool,
    visual_cursors: Vec<VisualCursor>,
    renderer_effects: Vec<RendererEffect>,
    transitions: Vec<TransitionState>,
}
```

Methods:
- `mark_dirty(&mut self)` — sets `frame_dirty = true`
- `is_dirty(&self) -> bool`
- `ingest_frame_glyphs(&mut self, buffer: FrameGlyphBuffer)` — stores glyph buffer
- `extend_current_frame_glyphs(&mut self, glyphs: Vec<FrameGlyph>)` — appends overlay glyphs
- `set_glyph_atlas(&mut self, atlas: WgpuGlyphAtlas)` — populates GPU resource

Rationale: These 7 fields are the compositor: the glyph atlas (GPU resource), the frame
buffer (what to draw), child frames (layering), dirty tracking (when to redraw), visual
cursors (cursor rendering), effects (post-processing), and transitions (crossfade/scroll).
Every render-side module that needs rendering state imports `FrameCompositor` — no more
importing `GuiFrameRenderState` just to set `frame_dirty`.

The effect and transition fields are NOT extracted into their own wrapper structs
because they lack cohesive behavior beyond what `render_pass.rs` and `lifecycle.rs`
already do with direct slice access. A single-field wrapper (`EffectData { effects: Vec }`)
violates the "behavior earns decomposition" principle.

**Fields remaining flat on GuiFrameRenderState** (not in any sub-struct):

| Field | Why Flat |
|-------|----------|
| `emacs_frame_id: u64` | Identity — every module needs it, no grouping makes sense |
| `mouse_pos: Option<PhysicalPosition<f64>>` | Input state — single field, no behavioral API |
| `cursor: CursorState` | Cursor blink state — has its own module (`cursor_runtime.rs`) |
| `floating_webkits: Vec<FloatingWebKit>` | Feature-gated field — accessed only in `media.rs` via `pointer_events.rs` for hit testing. Single field, no grouping needed |

**Final GuiFrameRenderState shape:**

```
GuiFrameRenderState {
    emacs_frame_id: u64,
    compositor: FrameCompositor,      // 7 fields
    chrome: ChromeState,              // 4 fields
    overlays: OverlayState,           // 8 fields
    cursor: CursorState,              // 1 field
    mouse_pos: Option<PhysicalPosition<f64>>,  // 1 field
    floating_webkits: Vec<FloatingWebKit>,     // 1 field (feature-gated)
}
```

Down from 22 flat fields to 7 structs/fields. Each import boundary is now narrow:
- `frame_ingest.rs` imports: `GuiFrameRenderState`, `FrameCompositor`
- `pointer_events.rs` imports: `ChromeState`, `OverlayState`, `GuiFrameRenderState`
- `ui_commands.rs` imports: `ChromeState`, `OverlayState`
- `transitions.rs` imports: `TransitionState` (already does)
- `cursor_runtime.rs` imports: `CursorState` (already does)

### Phase 2: Abstract pointer_events.rs (1 commit)

After Phase 1, replace direct field accesses with sub-struct method calls:

Before (90 direct accesses):
```rust
if let Some(ref popup) = window_state.render.popup_menu {
    if popup.rect.contains(pos) { ... }
}
if let Some(ref tooltip) = window_state.render.tooltip {
    if tooltip.rect.contains(pos) { ... }
}
if let Some(ref menu) = window_state.render.menu_bar {
    if menu.rect.contains(pos) { ... }
}
// ... 80 more lines
```

After (~5 method calls):
```rust
if let Some(hit) = window_state.render.overlays.popup_hit_test(pos) {
    return self.handle_popup_click(hit, window_id);
}
if let Some(hit) = window_state.render.chrome.hit_test(pos) {
    return self.handle_chrome_click(hit, window_id);
}
```

### Phase 3: Delete primary_* Delegates (2 commits)

#### 3a. Migrate call sites (first commit)

For each of the ~44 call sites that call `self.primary_*()`:

- **Command handlers with `emacs_frame_id`**: Already have the frame ID. Replace
  `self.primary_render_state_mut()` with
  `self.frame_windows.get_render_state_mut(emacs_frame_id)`.
- **Command handlers without `emacs_frame_id`**: Legacy commands like `SetWindowTitle`
  (no frame ID) vs `SetFrameWindowTitle` (has frame ID). Compute the primary frame ID
  via `self.frame_windows.primary_emacs_frame_id` or
  `self.frame_windows.primary_window_mut()`.
- **Lifecycle code**: Replace `self.primary_window()` with
  `self.frame_windows.primary_window()`.

#### 3b. Delete the methods (second commit)

After all call sites are migrated, delete the 44 `primary_*` methods from `RenderApp`.
Expected: ~200 lines removed.

### Phase 4: Split RenderCommand (2 commits)

#### Why single channel with 2-level enum (not multiple channels)

The plan keeps a single `crossbeam::bounded(64)` channel. The alternative — separate
channels per domain — would be:

```rust
struct CommandBus {
    lifecycle_tx: Sender<LifecycleCommand>,  // priority: immediate
    window_tx: Sender<WindowCommand>,        // priority: 1-2 frames
    ui_tx: Sender<UiCommand>,                // priority: 1-2 frames
    config_tx: Sender<ConfigCommand>,        // priority: relaxed
    asset_tx: Sender<AssetCommand>,          // priority: relaxed
    terminal_tx: Sender<TerminalCommand>,    // priority: relaxed
}
```

Reasons to NOT split channels:

1. **Simplicity**: One sender to pass to producers, one receiver to drain. No
   crossbeam::select! needed. No priority-inversion bugs between channels.
2. **Correctness by construction**: All commands in a frame batch before `Canvas::render()`
   are guaranteed to be processed in order. With multiple channels, a high-priority
   command arriving late would be processed out of order relative to a low-priority
   command that arrived earlier.
3. **Channel capacity is not the bottleneck**: 64 commands at capacity, drained every
   frame. At 60fps, that's 3840 commands/second. The Emacs evaluator produces commands
   at human-interaction rates (tens per second, not thousands).
4. **Cross-domain ordering doesn't matter in practice, but intra-domain ordering does**.
   `CreateWindow` must precede `ResizeWindow` (same domain), but `TerminalCreate` and
   `ConfigCommand` have no ordering dependency. The 2-level enum preserves intra-domain
   ordering by keeping all commands in a single channel while separating the dispatch
   by domain at the type level.

The 2-level enum gives all the benefits of domain separation (type-safe dispatch,
self-documenting which handler owns which command) without the complexity of multiple
channels.

#### 4a. Define sub-enums (first commit)

```rust
// In thread_comm.rs, or a new thread_comm/ module

enum RenderCommand {
    Lifecycle(LifecycleCommand),
    Window(WindowCommand),
    Asset(AssetCommand),
    Terminal(TerminalCommand),
    Ui(UiCommand),
    Config(ConfigCommand),
}

enum LifecycleCommand {
    Shutdown,
    SuspendTty,
    ResumeTty,
}

enum WindowCommand {
    SetWindowTitle { title: String },
    SetFrameWindowTitle { emacs_frame_id: u64, title: String },
    CreateWindow { emacs_frame_id: u64, ... },
    AdoptPrimaryFrame { emacs_frame_id: u64 },
    DestroyWindow { emacs_frame_id: u64 },
    ResizeWindow { emacs_frame_id: u64, ... },
    SetWindowFullscreen { emacs_frame_id: u64, ... },
    // ... 8 more
}

enum AssetCommand {
    // Image commands (5)
    ImageLoadFile { ... },
    ImageLoadData { ... },
    ImageLoadArgb32 { ... },
    ImageLoadRgb24 { ... },
    ImageFree { ... },
    // WebKit commands (14)
    WebKitCreate { ... },
    WebKitLoadUri { ... },
    // ... etc
    // Video commands (4)
    VideoCreate { ... },
    VideoPlay { ... },
    VideoPause { ... },
    VideoDestroy { ... },
}

enum TerminalCommand {
    TerminalCreate { ... },
    TerminalWrite { ... },
    TerminalResize { ... },
    TerminalDestroy { ... },
    TerminalSetFloat { ... },
}

enum UiCommand {
    ShowPopupMenu { ... },
    HidePopupMenu,
    ShowTooltip { ... },
    HideTooltip,
    VisualBell { ... },
    RequestAttention,
}

enum ConfigCommand {
    SetCursorBlink { ... },
    SetCursorAnimation { ... },
    SetAnimationConfig { ... },
    SetScrollIndicators { ... },
    // ... 5 more
}
```

#### 4b. Update dispatch and producers (second commit)

Update the dispatch chain from 4 fallthrough methods to a single top-level match:

```rust
fn dispatch(&mut self, cmd: RenderCommand) {
    match cmd {
        RenderCommand::Lifecycle(c) => self.handle_lifecycle(c),
        RenderCommand::Window(c) => self.handle_window(c),
        RenderCommand::Asset(c) => self.handle_asset(c),
        RenderCommand::Terminal(c) => self.handle_terminal(c),
        RenderCommand::Ui(c) => self.handle_ui(c),
        RenderCommand::Config(c) => self.handle_config(c),
    }
}
```

Update all producers (`neomacs-bin/src/main.rs`, test files) to use the new variants.

---

## 5. Commit Plan

**One phase, one commit.**

| # | Phase | Description | Expected Δ Lines |
|---|-------|-------------|-----------------|
| 0 | 0 | `FrameKey` enum + collapse 22 bulk methods | ~-100 |
| 1 | 1 | Extract `ChromeState`, `OverlayState`, `FrameCompositor` | ~-70 |
| 2 | 2 | Abstract pointer_events with sub-struct APIs | ~-80 |
| 3 | 3 | `FrameRef` enum in commands + delete `primary_*` delegates | ~-250 |
| 4 | 4 | Split `RenderCommand` into domain sub-enums | ~-50 |
|   |   | **Total** | **~-550** |

---

## 6. Risk Assessment

### Low Risk
- Phase 0 (bulk method collapse): Pure refactoring. Each closure replicates the existing
  method body exactly. Compiler guarantees correctness.
- Phase 1 (sub-struct extraction): Pure refactoring, no semantic change. Each field moves
  from `render.field` to `render.chrome.field` or `render.compositor.field`. The compiler
  catches every missed site.
- Phase 2 (pointer_events abstraction): Same behavior, cleaner API. Method bodies become
  sub-struct method calls. No logic change.

### Medium Risk
- Phase 3 (primary_* deletion): Some commands genuinely lack `emacs_frame_id` (legacy
  window commands like `SetWindowTitle`, `SetWindowFullscreen`, `SetWindowDecorated`,
  `SetMouseCursor`, `WarpMouse`). These must be routed through
  `self.frame_windows.primary_window_mut()` directly, which already exists. No protocol
  change needed — the `primary_window()` method on the manager computes the key from
  `primary_emacs_frame_id`.
- Phase 4 (RenderCommand split): Touches every command producer and test (~20+ files).
  Large surface area but mechanical — each variant just gets wrapped in one more enum
  layer. The new variants are `RenderCommand::Window(WindowCommand::CreateWindow { ... })`
  instead of `RenderCommand::CreateWindow { ... }`.

### Mitigations
- Every commit must pass `cargo nextest run -p neomacs-display-runtime` (902 tests).
- Every commit must pass `cargo nextest run --release -p neomacs-tui-tests` (835 tests).
- Release build verified after each phase.
- Each phase is independently reviewable and revertable.

---

## 7. Verification Checklist

After each commit:
- [ ] `cargo nextest run -p neomacs-display-runtime` passes (902 tests, 4 pre-existing TTY failures acceptable)
- [ ] `cargo build --release -p neomacs` succeeds
- [ ] No new compiler warnings

After all phases:
- [ ] `cargo nextest run --release -p neomacs-tui-tests` passes (835 tests)
- [ ] `cargo xtask fresh-build --release` succeeds (full pipeline: build + pdump + .elc)
- [ ] GUI launches on X11 display without errors

---

## 8. Open Questions (Post-Audit)

These are architectural concerns that the plan does NOT address, but should be tracked:

### 8.1 OverlayState May Be Too Broad

OverlayState bundles popup_menu, tooltip, visual_bell, fps, typing_speed, idle_dim, and
ime_preedit into one struct. But the access patterns differ:

- **popup_menu, tooltip**: Pointer-interactive. Need `hit_test()`. Shown/hidden on command.
- **visual_bell, fps, typing_speed, idle_dim**: Non-interactive. Need `tick_timeouts()`.
  Continuous rendering, timeout-based lifecycle.
- **ime_preedit**: Input state, not an overlay. Needs neither hit testing nor timeouts.

If the `tick_overlays()` method becomes a 30-line function that handles unrelated timeout
logic for 5 different overlay types, the abstraction has failed — it's just moved complexity
from the call site into the method body.

**Recommendation**: Implement OverlayState as planned, but watch for the method becoming
a God-function. If it does, split into `InteractiveOverlays` (popup, tooltip) and
`FeedbackOverlays` (bell, fps, typing, idle). ime_preedit should probably stay flat on
GuiFrameRenderState — it's input state, not an overlay.

### 8.2 Phase 0 Needs Read-Only + Predicate Variants

The plan says "only `for_each_top_level_mut` remains" but the 22 methods include:
- Mutable iteration (`for_each_top_level_window_mut`) → `for_each_top_level_mut(f)`
- Read-only iteration (`for_each_top_level_window`) → needs `for_each_top_level(f)`
- Predicate with early return (`any_top_level_window`) → needs `any_top_level(f) -> bool`

Three methods, not one. Still a big win over 22, but the plan should be precise.

### 8.3 Phase 4 Highest Risk-to-Reward Ratio

The RenderCommand split touches 20+ files (all producers in `neomacs-bin/src/main.rs`,
all test files, all handler files) for primarily documentation benefit. The dispatch chain
is already clean — 4 fallthrough methods that work. The 2-level enum adds nesting at
every producer site:

```rust
// Before:
RenderCommand::CreateWindow { emacs_frame_id, ... }
// After:
RenderCommand::Window(WindowCommand::CreateWindow { emacs_frame_id, ... })
```

This is more typing, more nesting, more match arms. The benefit: you can tell which
handler owns a variant without reading all four handler files. Is that worth touching
20+ files?

**Recommendation**: Consider deferring Phase 4 to a separate PR. Phases 0-3 deliver
~80% of the architectural improvement at ~40% of the risk. Phase 4 can be done later
when the codebase is stable.

### 8.4 pending/native Dispatch Pattern Not Addressed

`GuiFrameWindowState` has the `if native.is_some() { use native } else { queue pending }`
pattern repeated ~30 times across its 30 methods. This is a state machine (Pending →
Active → Destroyed) modeled as ad-hoc `Option` checks. The plan doesn't address this.

A cleaner model would be an explicit state enum:
```rust
enum FrameLifecycle {
    Pending { pending_native: PendingNativeState },
    Active { native: GuiFrameNativeWindowState },
    Destroyed,
}
```

But this is a separate refactoring from the decomposition. It should be tracked but
not bundled with this plan.

### 8.5 RenderApp After Deletion

After Phase 3 deletes 44 methods, `RenderApp` has ~8 remaining methods. Are these
cohesive? What's the new field count? The plan should include the expected post-refactor
shape of `RenderApp`:
- Fields: ~23 (same — Phase 3 deletes methods, not fields)
- Methods: ~8 (constructor, cursor sync, test helpers, non-trivial composers)

Is 23 fields still a God-object? Arguably yes. But the remaining fields span genuinely
different domains (GPU, images, terminals, monitors, channels) that can't be grouped
without introducing sub-structs that violate "behavior earns decomposition." The
decomposition of `GuiFrameRenderState` is where the real benefit lies.
