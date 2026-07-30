//! TTY layout tree construction, redisplay callback, and feature provision.
//!
//! Mirrors the TTY child-frame compositing in GNU `src/dispnew.c`
//! (`combine_updates_for_frame`) and the redisplay callback wiring that
//! normally lives in `src/xdisp.c` / `src/dispnew.c`.

use neomacs_display_protocol::SealedFramePresentation;
use neomacs_display_protocol::glyph_matrix::FrameDisplayState;
use neomacs_display_protocol::tty_rif::TtyRif;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedPresentationTicket {
    frame_id: FrameId,
    presentation: neovm_core::window::geometry::PresentationId,
}

impl PreparedPresentationTicket {
    pub fn activate(
        self,
        evaluator: &mut Context,
    ) -> Result<
        Option<neovm_core::window::geometry::PresentationId>,
        neovm_core::window::geometry::PresentationActivateError,
    > {
        evaluator
            .frame_manager_mut()
            .get_mut(self.frame_id)
            .ok_or(
                neovm_core::window::geometry::PresentationActivateError::UnknownPresentation(
                    self.presentation,
                ),
            )?
            .activate_display_presentation(self.presentation)
    }

    pub fn discard(self, evaluator: &mut Context) -> bool {
        evaluator.retire_interaction_presentation(self.presentation.get());
        evaluator
            .frame_manager_mut()
            .get_mut(self.frame_id)
            .is_some_and(|frame| frame.discard_display_presentation(self.presentation))
    }
}

#[must_use = "a prepared display must be submitted, activated, or discarded"]
pub struct PreparedFrameDisplay {
    ticket: PreparedPresentationTicket,
    state: SealedFramePresentation,
}

impl PreparedFrameDisplay {
    pub fn into_submission(self) -> (PreparedPresentationTicket, SealedFramePresentation) {
        (self.ticket, self.state)
    }

    pub fn activate(
        self,
        evaluator: &mut Context,
    ) -> Result<SealedFramePresentation, neovm_core::window::geometry::PresentationActivateError>
    {
        self.ticket.activate(evaluator)?;
        Ok(self.state)
    }

    pub fn discard(self, evaluator: &mut Context) -> FrameDisplayState {
        self.ticket.discard(evaluator);
        self.state.into_state()
    }
}

impl std::ops::Deref for PreparedFrameDisplay {
    type Target = FrameDisplayState;

    fn deref(&self) -> &Self::Target {
        self.state.state()
    }
}

pub(crate) fn current_layout_frame_id(evaluator: &Context) -> Option<FrameId> {
    evaluator
        .frame_manager()
        .selected_frame()
        .map(|frame| frame.id)
}

/// Layout purposes that are allowed to produce renderer-facing frame state.
///
/// Synchronous logical queries use a separate function, so Rust makes it
/// impossible to ask `layout_frame_display_state` for a query and accidentally
/// receive an older cached presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameLayoutPurpose {
    Redisplay,
    Snapshot,
}

impl FrameLayoutPurpose {
    const fn engine_purpose(self) -> neomacs_layout_engine::engine::LayoutPurpose {
        match self {
            Self::Redisplay => neomacs_layout_engine::engine::LayoutPurpose::Redisplay,
            Self::Snapshot => neomacs_layout_engine::engine::LayoutPurpose::Snapshot,
        }
    }

    const fn consumes_pending_input(self) -> bool {
        matches!(self, Self::Redisplay)
    }
}

pub fn layout_frame_display_state(
    evaluator: &mut Context,
    frame_id: FrameId,
    purpose: FrameLayoutPurpose,
) -> Option<PreparedFrameDisplay> {
    LAYOUT_ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        // Smooth scroll (Phase 1, T4): drain a pending trackpad pixel-scroll for
        // this frame and apply it (sub-line vscroll) before re-laying.
        if purpose.consumes_pending_input()
            && let Some(delta) = evaluator.take_pending_pixel_scroll_for_frame(frame_id)
            && let Some(window_id) = evaluator
                .frame_manager()
                .get(frame_id)
                .map(|frame| frame.selected_window)
        {
            // SIGN: trackpad delta_y vs scroll direction is verified on-screen
            // (T5); flip this negation if it scrolls the wrong way.
            let delta_px = (-delta).round() as i32;
            let _ = engine.pixel_scroll_window(evaluator, window_id, delta_px);
        }
        let _ = engine.layout_frame_rust_for_purpose(evaluator, frame_id, purpose.engine_purpose());
        let state = engine.last_frame_display_state.take()?;
        Some(PreparedFrameDisplay {
            ticket: PreparedPresentationTicket {
                frame_id,
                presentation: neovm_core::window::geometry::PresentationId::new(
                    state.presentation_id.get(),
                ),
            },
            state,
        })
    })
}

/// Lay out the frames a snapshot request covers — freshest state, on
/// demand — preserving the canonical parent-relative placement published by
/// layout. Shared by the TTY and GUI frontends (both use the same thread-local
/// `LAYOUT_ENGINE`).
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
        let Some(prepared) =
            layout_frame_display_state(evaluator, node.frame_id, FrameLayoutPurpose::Snapshot)
        else {
            continue;
        };
        states.push(prepared.discard(evaluator));
    }

    // A live frame outside the selected frame's tree (another top-level
    // frame): lay it out directly with its canonical root placement.
    if states.is_empty()
        && let SnapshotTarget::Frame(id) = target
        && let Some(prepared) =
            layout_frame_display_state(evaluator, FrameId(*id), FrameLayoutPurpose::Snapshot)
    {
        states.push(prepared.discard(evaluator));
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

/// Install the synchronous layout-query adapter used by display primitives
/// such as `(window-end WINDOW t)`.
///
/// This targets one window through the canonical row producer without entering
/// the renderer presentation lifecycle. Both GUI and TTY install this adapter;
/// batch mode intentionally does not.
pub fn install_window_layout_query_fn(evaluator: &mut Context) {
    evaluator.window_layout_query_fn = Some(Box::new(|eval, frame_id, window_id| {
        LAYOUT_ENGINE.with(|engine| {
            engine
                .borrow_mut()
                .query_window_end(eval, frame_id, window_id)
        })
    }));
}

// ── TTY layout tree and redisplay ─────────────────────────────────────────

pub fn run_tty_layout_tree(
    evaluator: &mut Context,
) -> Option<(SealedFramePresentation, Vec<SealedFramePresentation>)> {
    let selected = current_layout_frame_id(evaluator)?;
    let root_id = evaluator
        .frame_manager()
        .root_frame_id(selected)
        .unwrap_or(selected);
    let frame_order = evaluator
        .frame_manager()
        .frames_in_reverse_z_order(root_id, true);

    let root_state = layout_frame_display_state(evaluator, root_id, FrameLayoutPurpose::Redisplay)?
        .activate(evaluator)
        .ok()?;

    let mut child_states = Vec::new();
    for frame_id in frame_order {
        if frame_id == root_id {
            continue;
        }
        let Some(prepared) =
            layout_frame_display_state(evaluator, frame_id, FrameLayoutPurpose::Redisplay)
        else {
            continue;
        };
        let Ok(state) = prepared.activate(evaluator) else {
            continue;
        };
        child_states.push(state);
    }

    Some((root_state, child_states))
}

/// Rasterize the display state into a `TtyRif` and write ANSI output to stdout.
pub fn run_tty_rif_redisplay(
    tty_rif: &mut TtyRif,
    root: &SealedFramePresentation,
    children: &[SealedFramePresentation],
) {
    tty_rif.rasterize_presentations(root, children);
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
    install_window_layout_query_fn(evaluator);
}
