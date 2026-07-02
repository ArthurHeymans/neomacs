# GUI Frame Snapshot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose 100% of what NeoMacs displays (text, resolved colors, geometry, all visible frames) as plain text via `neomacs--frame-snapshot`, per `docs/plans/2026-07-02-gui-observability-agent-driving-design.md`.

**Architecture:** Serialize the real `FrameDisplayState` (serde JSON + a greppable text rendering) rather than a parallel schema. A new `frame_snapshot_fn` callback on the evaluator (same pattern as `redisplay_fn`, `eval.rs:1871`) is installed by neomacs-bin for both TTY and GUI; it lays out target frames **on demand** via the existing `tty_layout::layout_frame_display_state` (no retention map — supersedes design step 4; on-demand is fresher and needs no eviction). The subr forces `redisplay_with_force(true)` first so marker/point/hscroll prep has run.

**Tech Stack:** serde + serde_json, existing `LayoutEngine`/`FrameDisplayState` pipeline, `defsubr` registration.

## Global Constraints

- `cargo fmt --all` before every commit; commit messages verbose (root cause → mechanism → verification), no issue numbers.
- `cargo nextest run` output ALWAYS redirected to `./tmp/*.log`, never streamed.
- Real-binary tests use `--release`.
- GNU parity: the subr is `neomacs--`-namespaced (GNU release binaries lack GLYPH_DEBUG dump subrs; a GNU-named `dump-glyph-matrix` would be an fboundp divergence).
- Key seams (verified): `redisplay_fn` field `neovm-core/src/emacs_core/eval.rs:1871`, take/call/reinstall at `eval.rs:6867`; `defsubr(name, fn, min, max)` at `eval.rs:14196`, registrations in `builtins/mod.rs` (e.g. `redisplay` at :5879); shared engine `neomacs-bin/src/tty_layout.rs:24`, per-frame layout `tty_layout.rs:37-46`; GUI frame iteration `main.rs:3450-3490` (`render_frame_tree(selected, true).frames_bottom_to_top`, stamps `parent_id`/`parent_x`/`parent_y`/`z_order`); `DisplayFrameId` numeric == core `FrameId.0`.

---

### Task 1: serde derives on the display protocol

**Files:**
- Modify: `neomacs-display-protocol/Cargo.toml` (add `serde.workspace = true`; dev-dep `serde_json = "1"`)
- Modify: `neomacs-display-protocol/src/glyph_matrix.rs`, `frame_glyphs.rs`, `types.rs`, `face.rs`, `cursor.rs`, `effect_config.rs`, `gradient.rs`, `scroll_animation.rs` (derives)
- Test: `neomacs-display-protocol/src/glyph_matrix_test.rs`

**Interfaces:**
- Produces: `FrameDisplayState: Serialize + Deserialize` (and full transitive closure).

- [ ] **Step 1: failing test** — in `glyph_matrix_test.rs`:

```rust
#[test]
fn frame_display_state_serde_round_trip() {
    let state = state_with_text("hello serde"); // existing helper, :243
    let json = serde_json::to_string(&state).expect("serialize");
    let back: FrameDisplayState = serde_json::from_str(&json).expect("deserialize");
    let json2 = serde_json::to_string(&back).expect("re-serialize");
    assert_eq!(json, json2, "serde round-trip must be lossless");
    assert!(json.contains("hello"), "glyph chars must appear in JSON");
}
```

- [ ] **Step 2: verify it fails** — `cargo nextest run -p neomacs-display-protocol serde_round_trip > tmp/t1.log 2>&1`; expect compile error (no Serialize impl).
- [ ] **Step 3: implement** — add `#[derive(serde::Serialize, serde::Deserialize)]` to `FrameDisplayState` and chase compiler errors through the closure: `Glyph`, `GlyphType`, `GlyphArea`, `GlyphRow`, `FringeBitmapInfo`, `GlyphMatrix`, `WindowMatrixEntry`, `FrameChromeRow`, `BackgroundItem`, `FaceFillItem`, `BorderItem`, `CursorItem`, `ImageItem`, `VideoItem`, `XwidgetItem`, `ScrollBarItem`, `TtyMenuBarItem`, `TtyMenuBarState`, `GuiMenuBarState`, `GuiToolBarState`, `GuiCompactBarState`, `FrameTabBarState`, `ScrollRun`, `Color`, `Point`, `Size`, `Rect`, `Transform`, `DisplayFrameId`, `DisplayWindowId`, `DisplaySlotId`, `GlyphRowRole`, `FringeSide`, `FringeBitmapData`, `StipplePattern`, `PhysCursor`, `WindowCursor`, `WindowInfo`, `WindowTransitionKind`, `WindowTransitionHint`, `WindowEffectHint`, `Face` + its enums, `CursorStyle`, `CursorKind`, `CursorSpec`, `CursorBarWidth`, `EffectsConfig` (derive goes **inside the field macro** at `effect_config.rs` top and on each sub-config), `Gradient`, `ColorStop`, scroll/transition enums. Do NOT use `#[serde(skip)]` (100% contract) — every field serializes. `FaceDataFFI` (raw pointers) is NOT in the closure; leave untouched.
- [ ] **Step 4: pass** — same nextest command; expect PASS. Also `cargo nextest run -p neomacs-display-protocol > tmp/t1b.log 2>&1` (no regressions).
- [ ] **Step 5: commit** — `git add -A && cargo fmt --all && git commit` ("feat(display-protocol): serde Serialize/Deserialize across FrameDisplayState closure").

### Task 2: `Face::lisp_name` + `WindowInfo::buffer_name`

**Files:**
- Modify: `neomacs-display-protocol/src/face.rs` (Face struct :143; `impl Default` :225 gets `lisp_name: None`; literals at :441 and `frame_glyphs.rs:975` get `None`)
- Modify: `neomacs-layout-engine/src/neovm_bridge.rs` — the name-threading seam: `resolve_named_face(name: &str)` (:2582, name KNOWN) → add `lisp_name: Option<String>` to `ResolvedFace` (:2360) and set it there; `realize_face` (:3288) keeps `None` for anonymous faces
- Modify: `neomacs-layout-engine/src/display_row_face_state.rs` — carry name on `DisplayRowFace` (:20, set in `from_resolved`) and emit it in `render_face()` (:232, one of the 5 breaking literals)
- Modify: `neomacs-display-runtime/src/render_thread/media.rs:762` (`terminal_cell_face` literal → `None`), `render_thread/frame_state_test.rs:44` (test literal)
- Modify: `neomacs-layout-engine/src/engine.rs:582-585` (WindowFrameMetadata: add `buffer_name` from `evaluator.buffer_manager().get(buf_id)`), `display_frame_output.rs:23,443-461` (thread through to WindowInfo)
- Test: layout-engine engine test asserting both fields.

**Interfaces:**
- Produces: `Face { pub lisp_name: Option<String>, ... }` (LAST field — `#[repr(C)]` prefix layout preserved; `Face::new`/`..Default::default()` sites compile unchanged); `ResolvedFace { pub lisp_name: Option<String>, ... }`; `WindowInfo { pub buffer_name: String, ... }`.
- NOTE: neovm-core is NOT involved in protocol-Face construction (its `face.rs` Face is the lface-style spec type; name lives in registry keys, e.g. `FaceTable.faces: HashMap<Value, Face>`).

- [ ] **Step 1: failing test** — in the layout-engine test module that lays out a real frame (follow `engine_test.rs` conventions):

```rust
#[test]
fn window_info_carries_buffer_name_and_faces_carry_lisp_names() {
    // Arrange exactly like the nearest engine_test.rs test that asserts on
    // engine.last_frame_display_state (~70 precedents in that file): build
    // the bootstrap evaluator, run engine.layout_frame_rust, then:
    let state = engine.last_frame_display_state.take().expect("state");
    let info = state.window_infos.iter().find(|w| !w.is_minibuffer).unwrap();
    assert!(!info.buffer_name.is_empty(), "buffer_name populated");
    assert!(
        state.faces.values().any(|f| f.lisp_name.as_deref() == Some("default")),
        "realized default face carries its Lisp name"
    );
}
```

- [ ] **Step 2: verify fails** (compile error: no such fields). `> tmp/t2.log`
- [ ] **Step 3: implement** — add fields; fix every `Face { .. }` literal the compiler flags (neovm-core/src/face.rs realization knows the face symbol; sites without a name use `None`; `WindowFrameMetadata` gains `buffer_name: String` populated beside `buffer_file_name` at `engine.rs:585`).
- [ ] **Step 4: pass** + full `-p neomacs-layout-engine -p neomacs-display-protocol -p neovm-core` nextest to `tmp/t2b.log`.
- [ ] **Step 5: commit**.

### Task 3: `render_text` / `render_text_faces`

**Files:**
- Create: `neomacs-display-protocol/src/snapshot_text.rs` (+ `mod snapshot_text;` in lib.rs)
- Test: `neomacs-display-protocol/src/snapshot_text_test.rs`

**Interfaces:**
- Produces: `impl FrameDisplayState { pub fn render_text(&self) -> String; pub fn render_text_faces(&self) -> String; }`

**Frozen format** (goldens encode this):

```text
=== frame 1: 80x24 cols 640x384 px ===
[chrome 0]|File Edit Options
-- window 1 "*scratch*" bounds=(0,0 640x368)px text=(8,0 632x352)px start=1 end=12 selected --
   0|;; hello
   1|
[mode-line]|-UUU:----F1  *scratch*
[cursor] window=1 row=0 col=3 charpos=4
```

Rules: enabled rows only; row text = concat areas left_margin+text+right_margin; `GlyphType::Char{ch}` → ch (skip `padding` glyphs), `Composite{text}` → text, `Stretch{width_cols}` → that many spaces, `Image{image_id}` → `[img:{id}]`, `Glyphless{ch}` → ch; trailing spaces trimmed. Text rows numbered by matrix index `%4d|`; rows with `role != Text` labeled `[{role}]|` (kebab-case role name). Frame chrome rows as `[chrome {row_index}]|`. Window header from `WindowMatrixEntry` + matching `WindowInfo` (by window_id): buffer_name quoted, bounds/text bounds as `({x},{y} {w}x{h})px` with `{}` float formatting, `start=/end=`, flags ` selected` / ` minibuffer` when set. `[cursor]` line from `phys_cursor` when present. `render_text_faces` = same plus, after each row line, one `     : run {start_col}-{end_col} {face} fg=#RRGGBB bg=#RRGGBB` line per face run (consecutive glyphs sharing face_id; face = `lisp_name` else `face:{id}`; hex from `Color` accessors in types.rs:39).

- [ ] **Step 1: failing golden test**:

```rust
#[test]
fn render_text_minimal_frame_golden() {
    let mut state = state_with_text(";; hello");
    // give the row a cursor + ensure WindowInfo/buffer_name present (helper-built)
    let text = state.render_text();
    assert!(text.starts_with("=== frame "), "frame header:\n{text}");
    assert!(text.contains("|;; hello"), "row text:\n{text}");
}
#[test]
fn render_text_faces_lists_runs_with_hex_colors() {
    let state = state_with_text(";; hello");
    let out = state.render_text_faces();
    assert!(out.contains("fg=#"), "face runs with colors:\n{out}");
}
```

(plus exact-string golden for a fully hand-built two-row state with mode-line row, stretch glyph, image glyph, wide char + padding — assert the complete expected output string).

- [ ] **Step 2: verify fails** `> tmp/t3.log`
- [ ] **Step 3: implement** per frozen format.
- [ ] **Step 4: pass** + crate suite `> tmp/t3b.log`
- [ ] **Step 5: commit**.

### Task 4: core seam — `frame_snapshot_fn` + subrs

**Files:**
- Modify: `neovm-core/src/emacs_core/eval.rs` (field next to `redisplay_fn:1871`; init sites :4910, :5091, reset :2600)
- Create: snapshot types + subr impls in `neovm-core/src/emacs_core/xdisp.rs` (beside posn-*)
- Modify: `neovm-core/src/emacs_core/builtins/mod.rs` (defsubr registrations near `redisplay` :5879)
- Test: `neovm-core/src/emacs_core/xdisp_test.rs`

**Interfaces:**
- Produces (in `xdisp.rs`):

```rust
pub enum SnapshotTarget { Selected, All, Frame(u64) }
pub enum SnapshotFormat { Text, TextFaces, Json }
pub struct SnapshotRequest { pub target: SnapshotTarget, pub format: SnapshotFormat }
```

- Field: `pub frame_snapshot_fn: Option<Box<dyn FnMut(&mut Context, &SnapshotRequest) -> Result<String, String>>>`
- Subrs: `neomacs--frame-snapshot` (0..2), `neomacs--write-frame-snapshot` (1..3).

**Subr semantics:** FRAME nil→Selected, t→All, frame object→Frame(id) (reuse the frame-decoding helper used by `delete-frame`, `window_cmds/mod.rs:7613` area); FORMAT nil/`text`→Text, `text-faces`→TextFaces, `json`→Json, else `wrong-type-argument`. Flow: parse → `eval.redisplay_with_force(true)` → take/call/reinstall `frame_snapshot_fn` (mirror `eval.rs:6867`) → string Value (or write to PATH, return t). Hook absent → `(error "frame snapshot unavailable (no display)")`.

- [ ] **Step 1: failing tests** in `xdisp_test.rs`:

```rust
#[test]
fn frame_snapshot_errors_without_display_hook() {
    // batch evaluator (no frame_snapshot_fn): must signal, not panic
    assert!(eval.eval_str("(neomacs--frame-snapshot)").is_err());
}
#[test]
fn frame_snapshot_calls_installed_hook_after_redisplay() {
    eval.frame_snapshot_fn = Some(Box::new(|_, req| {
        assert!(matches!(req.target, SnapshotTarget::Selected));
        assert!(matches!(req.format, SnapshotFormat::Text));
        Ok("SNAP".into())
    }));
    let v = eval.eval_str("(neomacs--frame-snapshot)").unwrap();
    assert_eq!(v.as_str().unwrap(), "SNAP");
    let v = eval.eval_str("(neomacs--frame-snapshot t 'json)").unwrap(); // target All + Json
    // hook asserts updated per-case or records requests in a Vec for assertion
}
```

- [ ] **Step 2: fails** `> tmp/t4.log`  → **Step 3: implement** → **Step 4: pass + neovm-core suite slice** `> tmp/t4b.log` → **Step 5: commit**.

### Task 5: bin glue — collect, serialize, install (TTY + GUI)

**Files:**
- Modify: `neomacs-bin/Cargo.toml` (add `serde_json = "1"`), `neomacs-bin/src/tty_layout.rs` (collect + install fns), `neomacs-bin/src/main.rs:2286` area (GUI install), TTY install at `tty_layout.rs:162` site
- Test: `neomacs-bin/src/main_test.rs` (bootstrap-evaluator end-to-end, pattern at :1320)

**Interfaces:**
- Produces in `tty_layout.rs`:

```rust
pub fn install_frame_snapshot_fn(evaluator: &mut Context) {
    evaluator.frame_snapshot_fn = Some(Box::new(|eval, req| {
        let states = collect_snapshot_states(eval, &req.target)?;
        Ok(match req.format {
            SnapshotFormat::Json => {
                #[derive(serde::Serialize)]
                struct Doc<'a> { frames: &'a [FrameDisplayState] }
                serde_json::to_string(&Doc { frames: &states }).map_err(|e| e.to_string())?
            }
            SnapshotFormat::Text =>
                states.iter().map(|s| s.render_text()).collect::<Vec<_>>().join("\n"),
            SnapshotFormat::TextFaces =>
                states.iter().map(|s| s.render_text_faces()).collect::<Vec<_>>().join("\n"),
        })
    }));
}

fn collect_snapshot_states(eval: &mut Context, target: &SnapshotTarget)
    -> Result<Vec<FrameDisplayState>, String>
{
    // Selected/Frame(id): single layout_frame_display_state(eval, id), parent fields zeroed.
    // All: mirror publish_gui_frame (main.rs:3466-3485): render_frame_tree(selected, true)
    //      .frames_bottom_to_top, layout each node, stamp parent_id/parent_x/parent_y/z_order.
    // Frame(id) not live -> Err("no such frame").
}
```

- Install called from BOTH `main.rs` (GUI, beside redisplay_fn :2286) and the TTY setup (beside `tty_layout.rs:162`). Batch mode installs nothing.

- [ ] **Step 1: failing test** (main_test.rs, mirrors :1320 bootstrap pattern):

```rust
#[test]
fn frame_snapshot_subr_end_to_end_json_and_text() {
    let mut eval = create_bootstrap_evaluator_cached_with_features(&["neomacs"]).unwrap();
    let _b = bootstrap_buffers(&mut eval, 960, 640, gui_display());
    /* configure_gnu_startup_state as in :1329 */
    LAYOUT_ENGINE.with(|e| e.borrow_mut().enable_cosmic_metrics());
    install_frame_snapshot_fn(&mut eval);
    let json = eval.eval_str("(neomacs--frame-snapshot t 'json)").unwrap();
    let doc: serde_json::Value = serde_json::from_str(json.as_str().unwrap()).unwrap();
    assert!(!doc["frames"].as_array().unwrap().is_empty());
    let text = eval.eval_str("(neomacs--frame-snapshot)").unwrap();
    assert!(text.as_str().unwrap().contains("=== frame"));
    assert!(text.as_str().unwrap().contains("*scratch*"));
}
```

- [ ] **Step 2: fails** `> tmp/t5.log` → **Step 3: implement** → **Step 4: pass + neomacs-bin suite slice** `> tmp/t5b.log` → **Step 5: commit**.

### Task 6: gui-tests artifacts + fixture + smoke assertions

**Files:**
- Modify: `neomacs-gui-tests/src/lib.rs` (GuiArtifactSet :124-138 → add `frame_snapshot_json`/`frame_snapshot_txt` = `<scenario>.frame-snapshot.json/.txt`; env plumbing beside `NEOMACS_GUI_STATE_JSON`; manifest fields with byte sizes)
- Modify: `neomacs-gui-tests/fixtures/startup-smoke.el` (write snapshots when env vars set):

```elisp
(let ((snap-json (getenv "NEOMACS_GUI_FRAME_SNAPSHOT_JSON"))
      (snap-txt  (getenv "NEOMACS_GUI_FRAME_SNAPSHOT_TXT")))
  (when snap-json (neomacs--write-frame-snapshot snap-json t 'json))
  (when snap-txt  (neomacs--write-frame-snapshot snap-txt t 'text-faces)))
```

- Modify: `neomacs-gui-tests/tests/harness_contract.rs` (artifact-path contract), `tests/real_gui_smoke.rs` (assert snapshot content: `.txt` contains `NeoMacs GUI smoke line 00` and `=== frame`; `.json` parses, `frames[0]` has a window_info with `buffer_name == "*neomacs-gui-smoke*"`).
- [ ] Steps: failing harness test → fail log `tmp/t6.log` → implement → pass (+ real smoke under display if available; release binary) → commit.

### Task 7: drive-loop verification + agent docs + design-doc sync

**Files:** Create `docs/gui-agent-testing.md`; modify `docs/plans/2026-07-02-gui-observability-agent-driving-design.md` (step 4 retention → on-demand layout).

- [ ] Verify live loop (release build): `Xvfb` (or existing GUI test display), `neomacs -Q --eval "(server-start)"`, then `emacsclient --eval '(progn (execute-kbd-macro (kbd "M-x version RET")) (neomacs--frame-snapshot))'` — snapshot must reflect the echo-area/version output. Any server/event-loop bug found → root-cause fix (own commit) or documented blocker.
- [ ] Write `docs/gui-agent-testing.md`: launch recipe, snapshot formats, input injection recipes (`execute-kbd-macro`, `unread-command-events`), elisp introspection cookbook (`posn-at-x-y`, `text-properties-at`, `overlays-in`, `window-tree`), PNG-vs-snapshot roles.
- [ ] Commit.

### Task 8: completeness audit

- [ ] Compare renderer inputs (`frame_ingest.rs` materialization + render passes) against serialized fields; list any element drawn from non-serialized state in the design doc's contract section (expected: none; `FrameGlyphBuffer` is derived from `FrameDisplayState`).
- [ ] Full regression: `cargo xtask fresh-build --release > tmp/fresh.log 2>&1` + broad nextest slice `> tmp/final.log`. Commit doc updates.
