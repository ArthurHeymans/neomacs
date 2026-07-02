//! TTY layout tree construction, redisplay callback, and feature provision.
//!
//! Mirrors the TTY child-frame compositing in GNU `src/dispnew.c`
//! (`combine_updates_for_frame`) and the redisplay callback wiring that
//! normally lives in `src/xdisp.c` / `src/dispnew.c`.

use neomacs_display_protocol::glyph_matrix::FrameDisplayState;
use neomacs_display_protocol::tty_rif::TtyRif;
use neomacs_display_protocol::types::DisplayFrameId;
use neomacs_display_runtime::layout::LayoutEngine;
use neovm_core::emacs_core::eval::Context;
use neovm_core::emacs_core::value::Value;
use neovm_core::window::FrameId;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::StartupOptions;
use super::tty_init;

thread_local! {
    /// Start without font metrics to avoid the ~500ms cosmic-text font
    /// database scan on first access. The GUI path enables cosmic metrics
    /// explicitly; the TTY path leaves it disabled.
    pub static LAYOUT_ENGINE: std::cell::RefCell<LayoutEngine> =
        std::cell::RefCell::new(LayoutEngine::new_without_font_metrics());
}

// ── Layout helpers ────────────────────────────────────────────────────────

pub(crate) fn current_layout_frame_id(evaluator: &Context) -> Option<FrameId> {
    evaluator
        .frame_manager()
        .selected_frame()
        .map(|frame| frame.id)
}

pub fn layout_frame_display_state(
    evaluator: &mut Context,
    frame_id: FrameId,
) -> Option<FrameDisplayState> {
    LAYOUT_ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        engine.layout_frame_rust(evaluator, frame_id);
        engine.last_frame_display_state.take()
    })
}

/// Lay out the frames a snapshot request covers — freshest state, on
/// demand — stamping parent/origin/z-order exactly like `publish_gui_frame`
/// so the result is the composited screen. Shared by the TTY and GUI
/// frontends (both use the same thread-local `LAYOUT_ENGINE`).
pub fn collect_snapshot_states(
    evaluator: &mut Context,
    target: &neovm_core::emacs_core::xdisp::SnapshotTarget,
) -> Result<Vec<FrameDisplayState>, String> {
    use neovm_core::emacs_core::xdisp::SnapshotTarget;

    let selected =
        current_layout_frame_id(evaluator).ok_or_else(|| "no selected frame".to_string())?;
    let tree = evaluator
        .frame_manager()
        .render_frame_tree(selected, true)
        .ok_or_else(|| "no render frame tree for the selected frame".to_string())?;

    let mut states = Vec::new();
    for node in tree.frames_bottom_to_top {
        let keep = match target {
            SnapshotTarget::All => true,
            SnapshotTarget::Selected => node.frame_id == selected,
            SnapshotTarget::Frame(id) => node.frame_id.0 == *id,
        };
        if !keep {
            continue;
        }
        let Some(mut state) = layout_frame_display_state(evaluator, node.frame_id) else {
            continue;
        };
        state.parent_id = DisplayFrameId::new(node.parent_id.map_or(0, |id| id.0));
        state.parent_x = node.origin_in_root_x;
        state.parent_y = node.origin_in_root_y;
        state.z_order = node.z_order;
        states.push(state);
    }

    // A live frame outside the selected frame's tree (another top-level
    // frame): lay it out directly with root stamps.
    if states.is_empty()
        && let SnapshotTarget::Frame(id) = target
        && let Some(state) = layout_frame_display_state(evaluator, FrameId(*id))
    {
        states.push(state);
    }

    if states.is_empty() {
        return Err("frame snapshot: no frame produced display state".to_string());
    }
    Ok(states)
}

/// JSON envelope of a snapshot: `{"frames":[FrameDisplayState...]}` — the
/// array form is uniform for one frame or many, and the wrapper leaves room
/// for future metadata without a schema break.
#[derive(serde::Serialize)]
struct SnapshotDoc<'a> {
    frames: &'a [FrameDisplayState],
}

/// Install the `neomacs--frame-snapshot` hook (`Context::frame_snapshot_fn`).
///
/// Called by both frontends right where they install `redisplay_fn`; batch
/// mode installs nothing, so the subr signals "no display attached" there.
pub fn install_frame_snapshot_fn(evaluator: &mut Context) {
    use neovm_core::emacs_core::xdisp::SnapshotFormat;

    evaluator.frame_snapshot_fn = Some(Box::new(|eval, request| {
        let states = collect_snapshot_states(eval, &request.target)?;
        Ok(match request.format {
            SnapshotFormat::Json => serde_json::to_string(&SnapshotDoc { frames: &states })
                .map_err(|error| format!("frame snapshot JSON serialization failed: {error}"))?,
            SnapshotFormat::Text => states
                .iter()
                .map(|state| state.render_text())
                .collect::<Vec<_>>()
                .join("\n"),
            SnapshotFormat::TextFaces => states
                .iter()
                .map(|state| state.render_text_faces())
                .collect::<Vec<_>>()
                .join("\n"),
        })
    }));
}

fn frame_origin_in_root(evaluator: &Context, frame_id: FrameId) -> (f32, f32) {
    evaluator
        .frame_manager()
        .frame_origin_in_root(frame_id)
        .unwrap_or((0.0, 0.0))
}

// ── TTY layout tree and redisplay ─────────────────────────────────────────

pub fn run_tty_layout_tree(
    evaluator: &mut Context,
) -> Option<(FrameDisplayState, Vec<FrameDisplayState>)> {
    let selected = current_layout_frame_id(evaluator)?;
    let root_id = evaluator
        .frame_manager()
        .root_frame_id(selected)
        .unwrap_or(selected);
    let frame_order = evaluator
        .frame_manager()
        .frames_in_reverse_z_order(root_id, true);

    let mut root_state = layout_frame_display_state(evaluator, root_id)?;
    root_state.parent_id = DisplayFrameId::new(0);
    root_state.parent_x = 0.0;
    root_state.parent_y = 0.0;

    let mut child_states = Vec::new();
    for frame_id in frame_order {
        if frame_id == root_id {
            continue;
        }
        let Some(mut state) = layout_frame_display_state(evaluator, frame_id) else {
            continue;
        };
        let (x, y) = frame_origin_in_root(evaluator, frame_id);
        state.parent_id = root_state.frame_id;
        state.parent_x = x;
        state.parent_y = y;
        child_states.push(state);
    }

    Some((root_state, child_states))
}

/// Rasterize the display state into a `TtyRif` and write ANSI output to stdout.
pub fn run_tty_rif_redisplay(
    tty_rif: &mut TtyRif,
    root: &FrameDisplayState,
    children: &[FrameDisplayState],
) {
    tty_rif.rasterize_frame_tree(root, children);
    tty_rif.diff_and_render();
    let output = tty_rif.take_output();
    tracing::debug!("tty_rif: output {} bytes", output.len());
    if !output.is_empty() {
        use std::io::Write;
        let _ = std::io::stdout().write_all(&output);
        let _ = std::io::stdout().flush();
    }
}

// ── Redisplay callback installation ───────────────────────────────────────

/// Add a symbol to the `features` list in the evaluator, matching GNU's
/// `Fprovide` (`src/fns.c`).
pub fn provide_lisp_feature(evaluator: &mut Context, feature: &str) {
    let features = evaluator
        .obarray()
        .symbol_value("features")
        .copied()
        .unwrap_or(Value::NIL);
    let feature_value = Value::symbol(feature);
    let already_present = neovm_core::emacs_core::value::list_to_vec(&features)
        .is_some_and(|items| items.into_iter().any(|item| item == feature_value));
    if !already_present {
        evaluator.set_variable("features", Value::cons(feature_value, features));
    }
}

/// Install the TTY redisplay callback that drives `TtyRif` rasterization.
///
/// This function wires up:
/// 1. The `tty-child-frames` feature (GNU provides it unconditionally in
///    `syms_of_display`; we provide it here once we know we're TTY).
/// 2. A `TtyRif` with the current terminal dimensions.
/// 3. Disables cosmic-text metrics (TTY uses 1×1 char cells).
/// 4. Sets `evaluator.redisplay_fn` to the layout-tree → rasterize → render
///    pipeline.
#[cfg(test)]
pub fn install_tty_redisplay_callback(evaluator: &mut Context, startup: &StartupOptions) {
    install_tty_redisplay_callback_with_popup_redraw(evaluator, startup, None);
}

pub fn install_tty_redisplay_callback_with_popup_redraw(
    evaluator: &mut Context,
    startup: &StartupOptions,
    force_full_redraw: Option<Arc<AtomicBool>>,
) {
    if !tty_init::should_enable_live_tty_io(startup) {
        return;
    }

    provide_lisp_feature(evaluator, "tty-child-frames");

    let (cols, rows) = tty_init::query_terminal_size_cells().unwrap_or((80, 25));
    let mut tty_rif = TtyRif::new(cols as usize, rows as usize);
    // TTY frames use 1x1 character cell metrics (GNU Emacs
    // frame.c:1184-1185). Drop the layout engine's cosmic-text
    // FontMetricsService so char_advance,
    // status_line_font_metrics, etc. fall back to the
    // char-cell grid.
    LAYOUT_ENGINE.with(|engine| {
        engine.borrow_mut().disable_cosmic_metrics();
    });
    evaluator.redisplay_fn = Some(Box::new(move |eval: &mut Context| {
        eval.setup_thread_locals();
        if let Some((cols, rows)) = tty_init::query_terminal_size_cells() {
            let cols = cols as usize;
            let rows = rows as usize;
            if tty_rif.width() != cols || tty_rif.height() != rows {
                tty_rif.resize(cols, rows);
            }
        }
        if force_full_redraw
            .as_ref()
            .is_some_and(|force| force.swap(false, Ordering::AcqRel))
        {
            tty_rif.force_redraw();
        }
        if let Some((root, children)) = run_tty_layout_tree(eval) {
            run_tty_rif_redisplay(&mut tty_rif, &root, &children);
        }
    }));
    install_frame_snapshot_fn(evaluator);
}
