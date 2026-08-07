//! Window and frame management for the editor.
//!
//! Implements the Emacs window tree model:
//! - A **frame** contains a root window (which may be split).
//! - A **window** is either a *leaf* (displays a buffer) or an *internal*
//!   node with children (horizontal or vertical split).
//! - The **selected window** is the one receiving input.
//! - The **minibuffer window** is a special single-line window at the bottom.

use crate::buffer::{
    BufferId, BufferManager, CharLen, CharPos0, EmacsByteLen, EmacsBytePos, LispCharPos1,
    TextPositionAnchor,
};
use crate::emacs_core::value::{HashTableTest, Value};
use crate::gc_trace::GcTrace;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::collections::{HashMap, HashSet};

mod display;
mod frame_params;
pub mod geometry;
mod history;
mod parameters;
mod scroll_bar;
pub mod split;
pub mod window_markers;

pub use split::{CombinationLimit, DeleteResize, ParentSeal, SplitAttachment};

pub use display::{
    WindowBufferDisplayDefaults, WindowFringeDefaults, WindowScrollBarDefaults,
    WindowScrollBarGeometry, resolve_window_scroll_bar_geometry,
};
pub use frame_params::{
    CursorTypeSymbol, FrameFullscreen, FrameParam, FrameParamKey, FrameToolBarPosition,
    FrameZGroup, GNU_FRAME_PARAM_COUNT, GNU_FRAME_PARAMS,
};
pub use scroll_bar::{
    HorizontalScrollBarType, VerticalScrollBarType, is_valid_horizontal_scroll_bar_value,
    is_valid_vertical_scroll_bar_value,
};

// ---------------------------------------------------------------------------
// IDs
// ---------------------------------------------------------------------------

/// Opaque window identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

/// Opaque frame identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameId(pub u64);

/// Root-relative frame placement used by redisplay backends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderFrameNode {
    pub frame_id: FrameId,
    pub parent_id: Option<FrameId>,
    pub origin_in_root_x: f32,
    pub origin_in_root_y: f32,
    pub z_order: i32,
}

/// Bottom-to-top render order for one frame tree.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderFrameTree {
    pub root_id: FrameId,
    pub frames_bottom_to_top: Vec<RenderFrameNode>,
}

/// Keep frame and window numeric domains disjoint while both are represented
/// as Lisp integers.
pub(crate) const FRAME_ID_BASE: u64 = 1 << 32;

/// GNU `DEFAULT_TOOL_BAR_LABEL_SIZE` in `src/dispextern.h`.
pub const DEFAULT_TOOL_BAR_LABEL_SIZE: f32 = 14.0;
/// GNU `DEFAULT_TOOL_BAR_BUTTON_MARGIN` in `src/dispextern.h`.
pub const DEFAULT_TOOL_BAR_BUTTON_MARGIN: f32 = 4.0;
/// GNU `DEFAULT_TOOL_BAR_BUTTON_RELIEF` in `src/dispextern.h`.
pub const DEFAULT_TOOL_BAR_BUTTON_RELIEF: f32 = 1.0;
/// GNU `DEFAULT_TOOL_BAR_IMAGE_HEIGHT` in `src/dispextern.h`.
pub const DEFAULT_TOOL_BAR_IMAGE_HEIGHT: f32 = 24.0;

pub fn default_gui_tool_bar_line_height(font_pixel_size: f32) -> u32 {
    let scale = if font_pixel_size.is_finite() && font_pixel_size > 0.0 {
        (font_pixel_size / DEFAULT_TOOL_BAR_LABEL_SIZE).max(1.0)
    } else {
        1.0
    };
    let image_height = (DEFAULT_TOOL_BAR_IMAGE_HEIGHT * scale).round().max(1.0);
    let margin = (DEFAULT_TOOL_BAR_BUTTON_MARGIN * scale).round().max(0.0);
    let relief = (DEFAULT_TOOL_BAR_BUTTON_RELIEF * scale).round().max(1.0);

    (image_height + 2.0 * margin + 2.0 * relief)
        .round()
        .max(1.0) as u32
}

// ---------------------------------------------------------------------------
// Window geometry
// ---------------------------------------------------------------------------

/// Pixel-based rectangle for window placement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }
}

// ---------------------------------------------------------------------------
// Split direction
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal, // side by side
    Vertical,   // stacked
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitPlacement {
    BeforeTarget,
    AfterTarget,
}

impl SplitPlacement {
    fn is_before_target(self) -> bool {
        matches!(self, Self::BeforeTarget)
    }
}

// ---------------------------------------------------------------------------
// Window display state
// ---------------------------------------------------------------------------

/// Per-window display settings that GNU Emacs stores on `struct window`.
///
/// # Cursor audit follow-through
///
/// neomacs now stores GNU-like cursor state directly on the live window:
///
/// - `cursor`: intended cursor position in the latest redisplay result
/// - `output_cursor`: the nominal output position last committed by redisplay
/// - `phys_cursor`: the last physical cursor geometry emitted on screen
///
/// This mirrors GNU's `struct window` ownership model closely enough for
/// `window-cursor-info` and related stateful cursor queries. The Rust
/// redisplay path now drives this state through an explicit per-window output
/// pass before frame snapshots are published. Rust layout/status-line emission
/// advances `output_cursor` through explicit output-cursor moves, while row
/// snapshots remain published artifacts for renderer handoff.
#[derive(Clone, Debug)]
pub struct WindowDisplayState {
    /// Window-local display table; nil means inherit from the buffer/frame.
    pub display_table: Value,
    /// Window-local cursor type; t means use the buffer-local value.
    pub cursor_type: Value,
    /// Intended cursor position in the latest redisplay result.
    pub cursor: Option<WindowCursorPos>,
    /// Last physical cursor geometry produced by redisplay for this window.
    pub phys_cursor: Option<WindowCursorSnapshot>,
    /// Last nominal output position actually committed by redisplay.
    pub output_cursor: Option<WindowCursorPos>,
    /// Last physical cursor type emitted by redisplay.
    pub phys_cursor_type: WindowCursorKind,
    /// Last accepted redisplay's related query/output facts, committed together.
    redisplay_output: Option<WindowRedisplayOutput>,
    /// Whether the window currently owns a live physical cursor.
    pub phys_cursor_on_p: bool,
    /// Whether the cursor is hidden without invalidating the geometry.
    pub cursor_off_p: bool,
    /// Cursor visibility state committed by the last completed redisplay.
    pub last_cursor_off_p: bool,
    /// Last visual row where redisplay placed the cursor.
    pub last_cursor_vpos: i64,
    /// Raw fringe widths; `-1` means use the frame default.
    pub left_fringe_width: i32,
    pub right_fringe_width: i32,
    pub fringes_outside_margins: bool,
    pub fringes_persistent: bool,
    /// Raw scroll bar sizes; `-1` means use the frame default.
    pub scroll_bar_width: i32,
    pub vertical_scroll_bar_type: Value,
    pub scroll_bar_height: i32,
    pub horizontal_scroll_bar_type: Value,
    pub scroll_bars_persistent: bool,
}

impl Default for WindowDisplayState {
    fn default() -> Self {
        Self {
            display_table: Value::NIL,
            cursor_type: Value::T,
            cursor: None,
            phys_cursor: None,
            output_cursor: None,
            phys_cursor_type: WindowCursorKind::NoCursor,
            redisplay_output: None,
            phys_cursor_on_p: false,
            cursor_off_p: false,
            last_cursor_off_p: false,
            last_cursor_vpos: 0,
            left_fringe_width: -1,
            right_fringe_width: -1,
            fringes_outside_margins: false,
            fringes_persistent: false,
            scroll_bar_width: -1,
            vertical_scroll_bar_type: Value::T,
            scroll_bar_height: -1,
            horizontal_scroll_bar_type: Value::T,
            scroll_bars_persistent: false,
        }
    }
}

impl WindowDisplayState {
    pub const fn redisplay_output(&self) -> Option<&WindowRedisplayOutput> {
        self.redisplay_output.as_ref()
    }

    pub fn clear_cursor_state(&mut self) {
        self.cursor = None;
        self.clear_output_cursor_state();
        self.clear_physical_cursor_state();
    }

    /// Start a new output pass for this window.
    ///
    /// The last committed output cursor remains authoritative until redisplay
    /// emits a new cursor position for this window.
    fn begin_output_pass(&mut self) {
        self.cursor = None;
        self.clear_physical_cursor_state();
    }

    /// Start a new output update for a window that will actively emit rows in
    /// the current redisplay pass.
    fn begin_window_output_update(&mut self) {
        self.begin_output_pass();
        self.clear_output_cursor_state();
    }

    fn clear_output_cursor_state(&mut self) {
        self.output_cursor = None;
    }

    fn clear_physical_cursor_state(&mut self) {
        self.phys_cursor = None;
        self.phys_cursor_type = WindowCursorKind::NoCursor;
        self.phys_cursor_on_p = false;
    }

    fn install_logical_cursor(&mut self, cursor: Option<WindowCursorPos>) {
        self.cursor = cursor;
    }

    /// Move the live output cursor to a new nominal output position.
    ///
    /// This mirrors GNU's `output_cursor_to` style of update more closely
    /// than the older row-start/row-finish helpers: Rust redisplay advances
    /// output by explicit output positions, while row boundaries remain local
    /// to snapshot recording in the layout/output emitter.
    fn output_cursor_to(&mut self, pos: WindowCursorPos) {
        self.output_cursor = Some(pos);
    }

    fn apply_physical_cursor_snapshot(&mut self, cursor: Option<WindowCursorSnapshot>) {
        self.phys_cursor = cursor.clone();
        self.phys_cursor_type = cursor
            .as_ref()
            .map(|c| c.kind)
            .unwrap_or(WindowCursorKind::NoCursor);
        self.phys_cursor_on_p = cursor.is_some();
    }

    fn commit_completed_redisplay(&mut self) {
        self.last_cursor_off_p = self.cursor_off_p;
        if let Some(cursor) = self.phys_cursor.as_ref() {
            self.last_cursor_vpos = cursor.row;
        } else if let Some(cursor) = self.cursor.as_ref() {
            self.last_cursor_vpos = cursor.row;
        }
    }
}

/// Explicit live redisplay/update session for one window.
///
/// This mirrors GNU's per-window output/update ownership model: explicit
/// output-cursor moves, cursor installation, and final redisplay commit all
/// flow through one update object over the live `WindowDisplayState`.
/// Snapshot replay remains a narrow compatibility path for replay/bootstrap
/// cases and is not used by the normal Rust layout pipeline.
pub struct WindowOutputUpdate<'a> {
    display: &'a mut WindowDisplayState,
}

impl<'a> WindowOutputUpdate<'a> {
    fn new(display: &'a mut WindowDisplayState) -> Self {
        Self { display }
    }

    pub fn begin_update(&mut self) {
        self.display.begin_window_output_update();
    }

    pub fn output_cursor_to(&mut self, pos: WindowCursorPos) {
        self.display.output_cursor_to(pos);
    }

    pub fn output_cursor_to_coords(&mut self, row: i64, col: i64, y: i64, x: i64) {
        self.output_cursor_to(WindowCursorPos { x, y, row, col });
    }

    fn replay_output_rows(&mut self, rows: &[DisplayRowSnapshot]) {
        if rows.is_empty() {
            self.display.clear_output_cursor_state();
            return;
        }
        for row in rows {
            self.output_cursor_to_coords(row.row, row.start_col, row.y, row.start_x);
            self.output_cursor_to_coords(row.row, row.end_col, row.y, row.end_x);
        }
    }

    pub fn install_logical_cursor(&mut self, cursor: Option<WindowCursorPos>) {
        self.display.install_logical_cursor(cursor);
    }

    pub fn apply_physical_cursor_snapshot(&mut self, cursor: Option<WindowCursorSnapshot>) {
        self.display.apply_physical_cursor_snapshot(cursor);
    }

    fn fallback_output_cursor_from_snapshot(&mut self, snapshot: &WindowDisplaySnapshot) {
        if self.display.output_cursor.is_none() {
            self.replay_output_rows(&snapshot.rows);
        }
    }

    pub fn finalize_live_update(
        &mut self,
        logical_cursor: Option<WindowCursorPos>,
        phys_cursor: Option<WindowCursorSnapshot>,
    ) {
        self.install_logical_cursor(logical_cursor);
        self.apply_physical_cursor_snapshot(phys_cursor);
        self.commit();
    }

    pub fn finalize_with_output_fallback(
        &mut self,
        logical_cursor: Option<WindowCursorPos>,
        phys_cursor: Option<WindowCursorSnapshot>,
        output_fallback: &WindowDisplaySnapshot,
    ) {
        self.install_logical_cursor(logical_cursor);
        self.apply_physical_cursor_snapshot(phys_cursor);
        self.fallback_output_cursor_from_snapshot(output_fallback);
        self.commit();
    }

    pub fn replay_snapshot(&mut self, snapshot: &WindowDisplaySnapshot) {
        self.begin_update();
        self.install_logical_cursor(snapshot.logical_cursor_pos());
        self.replay_output_rows(&snapshot.rows);
        self.apply_physical_cursor_snapshot(snapshot.phys_cursor.clone());
        self.commit();
    }

    pub fn commit(&mut self) {
        self.display.commit_completed_redisplay();
    }
}

/// Live-window history state that GNU Emacs stores directly on `struct window`.
#[derive(Clone, Debug)]
pub struct WindowHistoryState {
    pub prev_buffers: Value,
    pub next_buffers: Value,
    pub use_time: i64,
}

impl Default for WindowHistoryState {
    fn default() -> Self {
        Self {
            prev_buffers: Value::NIL,
            next_buffers: Value::NIL,
            use_time: 0,
        }
    }
}

pub(crate) type WindowParameters = Vec<(Value, Value)>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowMargins {
    left: usize,
    right: usize,
}

impl WindowMargins {
    pub const ZERO: Self = Self { left: 0, right: 0 };

    pub const fn new(left: usize, right: usize) -> Self {
        Self { left, right }
    }

    pub const fn left(self) -> usize {
        self.left
    }

    pub const fn right(self) -> usize {
        self.right
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowRedisplayState {
    pub id: WindowId,
    pub buffer_id: BufferId,
    pub bounds: (u32, u32, u32, u32),
    pub window_start: LispCharPos1,
    pub window_end: WindowEndState,
    pub point: LispCharPos1,
    pub old_point: LispCharPos1,
    pub hscroll: usize,
    pub vscroll: i32,
    pub preserve_vscroll_p: bool,
}

/// Zero-based glyph-matrix row coordinate.
///
/// This deliberately does not accept or expose buffer positions, display
/// columns, or pixel coordinates. Those domains have separate types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MatrixRow0(usize);

impl MatrixRow0 {
    pub const ZERO: Self = Self(0);

    pub const fn new(row: usize) -> Self {
        Self(row)
    }

    pub const fn get(self) -> usize {
        self.0
    }
}

/// One atomically published GNU-compatible window-end record.
///
/// GNU stores both distances from buffer Z plus the matrix row that produced
/// them. Keeping the tuple together prevents partial char/byte/row updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowEndRecord {
    char_offset_from_z: CharLen,
    byte_offset_from_z: EmacsByteLen,
    matrix_row: MatrixRow0,
}

impl WindowEndRecord {
    pub fn from_anchors(
        buffer_z: TextPositionAnchor,
        end: TextPositionAnchor,
        matrix_row: MatrixRow0,
    ) -> Self {
        Self {
            char_offset_from_z: buffer_z
                .char_pos()
                .saturating_offset_from(end.char_pos().min(buffer_z.char_pos())),
            byte_offset_from_z: buffer_z
                .emacs_byte_pos()
                .saturating_offset_from(end.emacs_byte_pos().min(buffer_z.emacs_byte_pos())),
            matrix_row,
        }
    }

    pub fn from_positions(
        buffer_z_char: LispCharPos1,
        buffer_z_byte: EmacsBytePos,
        end_charpos: LispCharPos1,
        end_bytepos: EmacsBytePos,
        matrix_row: MatrixRow0,
    ) -> Self {
        Self::from_anchors(
            TextPositionAnchor::new(CharPos0::from_lisp(buffer_z_char), buffer_z_byte),
            TextPositionAnchor::new(CharPos0::from_lisp(end_charpos), end_bytepos),
            matrix_row,
        )
    }

    pub const fn char_offset_from_z(self) -> CharLen {
        self.char_offset_from_z
    }

    pub const fn byte_offset_from_z(self) -> EmacsByteLen {
        self.byte_offset_from_z
    }

    pub const fn matrix_row(self) -> MatrixRow0 {
        self.matrix_row
    }

    /// Recover GNU's Lisp-visible end position from the current buffer Z.
    pub fn charpos_from_z(self, buffer_z: LispCharPos1) -> LispCharPos1 {
        let buffer_z = buffer_z.to_one_based_usize();
        LispCharPos1::from_one_based_usize(
            buffer_z
                .saturating_sub(self.char_offset_from_z.get())
                .max(1),
        )
    }

    /// Recover the byte companion of [`Self::charpos_from_z`].
    pub const fn bytepos_from_z(self, buffer_z_byte: EmacsBytePos) -> EmacsBytePos {
        buffer_z_byte.saturating_sub_len(self.byte_offset_from_z)
    }
}

/// One accepted window redisplay's mutually consistent output facts.
///
/// These values all come from the same immutable snapshot and presentation
/// generation. Consumers never have to combine a new end record with an old
/// visible span or cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowRedisplayOutput {
    generation: geometry::PresentationId,
    window_end: WindowEndRecord,
    visible_span: Option<WindowVisibleBufferSpan>,
    logical_cursor: Option<WindowCursorPos>,
    phys_cursor: Option<WindowCursorSnapshot>,
}

impl WindowRedisplayOutput {
    fn from_snapshot(
        generation: geometry::PresentationId,
        snapshot: &WindowDisplaySnapshot,
        window_end: WindowEndRecord,
    ) -> Self {
        Self {
            generation,
            window_end,
            visible_span: snapshot.visible_buffer_span(),
            logical_cursor: snapshot.logical_cursor_pos(),
            phys_cursor: snapshot.phys_cursor.clone(),
        }
    }

    pub const fn generation(&self) -> geometry::PresentationId {
        self.generation
    }

    pub const fn window_end(&self) -> WindowEndRecord {
        self.window_end
    }

    pub const fn visible_span(&self) -> Option<WindowVisibleBufferSpan> {
        self.visible_span
    }

    pub const fn logical_cursor(&self) -> Option<WindowCursorPos> {
        self.logical_cursor
    }

    pub fn phys_cursor(&self) -> Option<&WindowCursorSnapshot> {
        self.phys_cursor.as_ref()
    }
}

/// Presentation lifecycle of a leaf window's last complete end record.
///
/// `Stale` retains GNU's `UPDATE=nil` behavior while making it impossible to
/// mark a partially populated record current.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowEndState {
    #[default]
    Unrecorded,
    Stale(WindowEndRecord),
    Current(WindowEndRecord),
}

impl WindowEndState {
    pub const fn record(self) -> Option<WindowEndRecord> {
        match self {
            Self::Unrecorded => None,
            Self::Stale(record) | Self::Current(record) => Some(record),
        }
    }

    pub const fn is_current(self) -> bool {
        matches!(self, Self::Current(_))
    }

    /// Recover the last recorded end, treating an unrecorded offset as GNU's
    /// zero-initialized distance from Z.
    pub fn charpos_from_z(self, buffer_z: LispCharPos1) -> LispCharPos1 {
        self.record()
            .map_or(buffer_z, |record| record.charpos_from_z(buffer_z))
    }

    /// Recover the byte companion of [`Self::charpos_from_z`].
    pub fn bytepos_from_z(self, buffer_z_byte: EmacsBytePos) -> EmacsBytePos {
        self.record()
            .map_or(buffer_z_byte, |record| record.bytepos_from_z(buffer_z_byte))
    }

    fn invalidate(&mut self) {
        if let Self::Current(record) = *self {
            *self = Self::Stale(record);
        }
    }
}

fn redisplay_f32_bits(value: f32) -> u32 {
    if value == 0.0 {
        0.0f32.to_bits()
    } else {
        value.to_bits()
    }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

/// Marker handles have distinct roles in GNU's live-window invariant. Keeping
/// those roles as separate types prevents accidentally moving (for example)
/// `w->start` when a caller intended to move `w->pointm`.
///
/// Each handle owns both the internal chain ID and the Lisp marker value that
/// roots its `MarkerObj` for precise GC. A numeric ID alone is not ownership:
/// the buffer's intrusive marker chain is deliberately weak and GC unlinks
/// unmarked entries before sweeping them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowStartMarker {
    id: u64,
    gc_root: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPointMarker {
    id: u64,
    gc_root: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowOldPointMarker {
    id: u64,
    gc_root: Value,
}

/// The complete marker set owned by one live leaf window.
///
/// The fields are intentionally private: construction is atomic through the
/// window-marker lifecycle module, so a window cannot represent a partially
/// attached `(start, point, old-point)` marker set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachedWindowPositionMarkers {
    start: WindowStartMarker,
    point: WindowPointMarker,
    old_point: WindowOldPointMarker,
}

/// Marker lifecycle for a leaf window.
///
/// Fresh structural nodes may be detached while their buffer is being chosen;
/// every live frame/window factory must transition them to `Attached` before
/// publishing the window.  One enum replaces three independent `Option`s and
/// makes all partial attachment states unrepresentable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowPositionMarkerState {
    #[default]
    Detached,
    Attached(AttachedWindowPositionMarkers),
}

impl WindowPositionMarkerState {
    pub const fn is_attached(self) -> bool {
        matches!(self, Self::Attached(_))
    }

    fn attached(self) -> Option<AttachedWindowPositionMarkers> {
        match self {
            Self::Detached => None,
            Self::Attached(markers) => Some(markers),
        }
    }

    fn detach(&mut self) -> Option<AttachedWindowPositionMarkers> {
        std::mem::take(self).attached()
    }

    fn trace_roots(self, roots: &mut Vec<Value>) {
        if let Self::Attached(markers) = self {
            markers.trace_roots(roots);
        }
    }
}

impl AttachedWindowPositionMarkers {
    fn new(start: (u64, Value), point: (u64, Value), old_point: (u64, Value)) -> Self {
        Self {
            start: WindowStartMarker {
                id: start.0,
                gc_root: start.1,
            },
            point: WindowPointMarker {
                id: point.0,
                gc_root: point.1,
            },
            old_point: WindowOldPointMarker {
                id: old_point.0,
                gc_root: old_point.1,
            },
        }
    }

    fn trace_roots(self, roots: &mut Vec<Value>) {
        roots.push(self.start.gc_root);
        roots.push(self.point.gc_root);
        roots.push(self.old_point.gc_root);
    }
}

impl WindowStartMarker {
    const fn raw(self) -> u64 {
        self.id
    }
}

impl WindowPointMarker {
    const fn raw(self) -> u64 {
        self.id
    }
}

impl WindowOldPointMarker {
    const fn raw(self) -> u64 {
        self.id
    }
}

/// A window in the window tree.
#[derive(Clone, Debug)]
// Window nodes own their complete leaf/split state and are cloned as snapshots;
// boxing the larger leaf would add indirection throughout the hot layout path.
#[allow(clippy::large_enum_variant)]
pub enum Window {
    /// Leaf window displaying a buffer.
    Leaf {
        id: WindowId,
        buffer_id: BufferId,
        /// Pixel bounds within the frame.
        bounds: Rect,
        /// Cached byte position of the first visible character.
        ///
        /// GNU Emacs stores this as a marker (`w->start`) so that buffer
        /// edits before the start position auto-shift it.  neomacs now
        /// maintains a real marker in `position_markers` alongside this
        /// cached Lisp-visible position.  The cache is refreshed from the marker
        /// by `sync_window_positions_from_markers` after every text edit,
        /// and on every explicit position write.  All read sites continue
        /// to use this typed cache for zero-cost reads.
        window_start: LispCharPos1,
        /// Atomic ownership state for GNU's `start`, `pointm`, and
        /// `old_pointm` markers.
        position_markers: WindowPositionMarkerState,
        /// Last atomically published window-end record and its freshness.
        window_end: WindowEndState,
        /// Cached cursor (point) position in this window.
        ///
        /// GNU Emacs stores this as a marker (`w->pointm`) so that buffer
        /// insertions before the position auto-shift it.  neomacs now
        /// maintains a real marker in `position_markers` alongside this
        /// cached Lisp-visible position.  The cache is refreshed from the marker
        /// after every text edit.  For the selected window, reads should
        /// prefer `buffer.pt` directly (GNU fast path); this cached value
        /// is authoritative for non-selected windows.
        point: LispCharPos1,
        /// Cached previous point value mirrored from GNU `w->old_pointm`.
        old_point: LispCharPos1,
        /// Mirror GNU `w->dedicated`: the dedication flag value.
        /// nil = not dedicated, t = strongly dedicated,
        /// side = side-window dedication (blocks display-buffer reuse but
        /// allows switch-to-buffer / set-window-buffer).
        dedicated: Value,
        /// Lisp-visible per-window parameter alist, newest entries first.
        parameters: WindowParameters,
        /// Live-window history state mirrored from GNU `struct window`.
        history: WindowHistoryState,
        /// Desired height in lines (for fixed windows, 0 = flexible).
        fixed_height: usize,
        /// Desired width in columns (for fixed windows, 0 = flexible).
        fixed_width: usize,
        /// Horizontal scroll offset (columns).
        hscroll: usize,
        /// Lower bound for automatic horizontal scrolling (columns).
        ///
        /// Mirrors GNU `w->min_hscroll` (`src/window.h`). Set equal to the
        /// current `hscroll` by `scroll-left`/`scroll-right` when their
        /// SET-MINIMUM argument is non-nil; the auto-hscroll pass
        /// (`hscroll_window_tree`, STEP 7) never scrolls below it. Reset to
        /// 0 on buffer switch.
        min_hscroll: usize,
        /// Whether automatic horizontal scrolling is currently suspended for
        /// this window.
        ///
        /// Mirrors GNU `w->suspend_auto_hscroll` (`src/window.h`). Set true by
        /// `set-window-hscroll` and `scroll-left`/`scroll-right` so a manual
        /// hscroll sticks instead of being recomputed every redisplay; cleared
        /// by `hscroll_window_tree` STEP 4 once window point explicitly moves,
        /// and reset to false on buffer switch.
        suspend_auto_hscroll: bool,
        /// Raw GNU `w->vscroll` value in pixels: zero or negative.
        ///
        /// Lisp-visible `window-vscroll` reports `-vscroll`, either in pixels
        /// or in canonical line units depending on the call site.
        vscroll: i32,
        /// Mirrors GNU `w->preserve_vscroll_p`.
        preserve_vscroll_p: bool,
        /// Mirrors GNU `w->force_start`: the window start was set explicitly
        /// (window_scroll / set-window-start), so the next redisplay must
        /// honor it and move POINT into the window if it ended up outside,
        /// instead of recomputing the start around point. One-shot: cleared
        /// when redisplay publishes the window's positions.
        force_start: bool,
        /// Window margins in columns.
        margins: WindowMargins,
        /// Window-local display settings mirrored from GNU `struct window`.
        display: WindowDisplayState,
        /// Pending pixel size queued by `set-window-new-pixel`. GNU
        /// stores this as `w->new_pixel`
        /// (`src/window.h:283`). Cleared by `window-resize-apply`
        /// once committed. Window audit Structural 1 in
        /// `drafts/window-system-audit.md` moved this off a
        /// thread-local HashMap onto the window struct so
        /// window-configuration save/restore round-trips it
        /// automatically.
        new_pixel: Option<i64>,
        /// Pending total (line-cell) size queued by
        /// `set-window-new-total`. GNU `w->new_total`
        /// (`src/window.h:284`).
        new_total: Option<i64>,
        /// Pending normal-size fraction queued by
        /// `set-window-new-normal`. GNU `w->new_normal`
        /// (`src/window.h:285`). Stored as a `Value` to mirror
        /// GNU's Lisp_Object slot — `Value::NIL` means "unset".
        new_normal: Value,
        /// Authoritative proportional vertical size
        /// (height fraction of parent). GNU `w->normal_lines`
        /// (`src/window.h:128`). Initialized to 1.0 on the root
        /// and updated by `window-resize-apply` from
        /// `new_normal`. `(window-normal-size w nil)` returns
        /// this value. Window audit Critical 7 in
        /// `drafts/window-system-audit.md`.
        normal_lines: Value,
        /// Authoritative proportional horizontal size
        /// (width fraction of parent). GNU `w->normal_cols`
        /// (`src/window.h:129`).
        normal_cols: Value,
        /// Character-line top edge, stored separately from `bounds` (pixels).
        /// Mirrors GNU `w->top_line` (`src/window.h`). GNU maintains this in
        /// parallel with `pixel_top`; in batch a menu bar occupies 1 *line* but
        /// 0 *pixels*, so `top_line` can be nonzero while `bounds.y` is 0.
        /// Set by the resize passes (root gets `FRAME_TOP_MARGIN`).
        top_line: i64,
        /// Character-column left edge; GNU `w->left_col`. See `top_line`.
        left_col: i64,
    },

    /// Internal node: contains children split in a direction.
    Internal {
        id: WindowId,
        direction: SplitDirection,
        children: Vec<Window>,
        bounds: Rect,
        /// Character-line top edge; GNU `w->top_line`. See `Leaf::top_line`.
        top_line: i64,
        /// Character-column left edge; GNU `w->left_col`. See `Leaf::top_line`.
        left_col: i64,
        /// Lisp-visible per-window parameter alist, newest entries first.
        parameters: WindowParameters,
        /// Combination limit — prevents recombination when non-nil.
        /// Mirrors GNU Emacs `w->combination_limit`.
        combination_limit: bool,
        /// Pending pixel size — see `Leaf::new_pixel`. GNU keeps
        /// the same `new_pixel` slot on every `struct window`,
        /// regardless of leaf/internal split state.
        new_pixel: Option<i64>,
        /// Pending total size — see `Leaf::new_total`.
        new_total: Option<i64>,
        /// Pending normal-size fraction — see `Leaf::new_normal`.
        new_normal: Value,
        /// Persistent normal-size fraction — see
        /// `Leaf::normal_lines`.
        normal_lines: Value,
        /// Persistent normal-size fraction — see
        /// `Leaf::normal_cols`.
        normal_cols: Value,
    },
}

impl Window {
    /// Create a new leaf window.
    pub fn new_leaf(id: WindowId, buffer_id: BufferId, bounds: Rect) -> Self {
        Window::Leaf {
            id,
            buffer_id,
            bounds,
            window_start: LispCharPos1::ONE,
            position_markers: WindowPositionMarkerState::Detached,
            window_end: WindowEndState::Unrecorded,
            point: LispCharPos1::ONE,
            old_point: LispCharPos1::ONE,
            dedicated: Value::NIL,
            parameters: Vec::new(),
            history: WindowHistoryState::default(),
            fixed_height: 0,
            fixed_width: 0,
            hscroll: 0,
            min_hscroll: 0,
            suspend_auto_hscroll: false,
            vscroll: 0,
            preserve_vscroll_p: false,
            force_start: false,
            margins: WindowMargins::ZERO,
            display: WindowDisplayState::default(),
            new_pixel: None,
            new_total: None,
            new_normal: Value::NIL,
            // GNU `make_window` initializes `normal_lines` and
            // `normal_cols` to 1.0 (`src/window.c:4603-4604`).
            normal_lines: Value::make_float(1.0),
            normal_cols: Value::make_float(1.0),
            // GNU `make_window` leaves top_line/left_col zero; the resize passes
            // assign them (root window gets `FRAME_TOP_MARGIN`).
            top_line: 0,
            left_col: 0,
        }
    }

    /// Character-line top edge. GNU `w->top_line` (`WINDOW_TOP_EDGE_LINE`).
    pub fn top_line(&self) -> i64 {
        match self {
            Window::Leaf { top_line, .. } | Window::Internal { top_line, .. } => *top_line,
        }
    }

    /// Set the character-line top edge. GNU assigns `w->top_line` in the resize
    /// passes / config restore.
    pub fn set_top_line(&mut self, value: i64) {
        match self {
            Window::Leaf { top_line, .. } | Window::Internal { top_line, .. } => *top_line = value,
        }
    }

    /// Character-column left edge. GNU `w->left_col` (`WINDOW_LEFT_EDGE_COL`).
    pub fn left_col(&self) -> i64 {
        match self {
            Window::Leaf { left_col, .. } | Window::Internal { left_col, .. } => *left_col,
        }
    }

    /// Set the character-column left edge. GNU `w->left_col`.
    pub fn set_left_col(&mut self, value: i64) {
        match self {
            Window::Leaf { left_col, .. } | Window::Internal { left_col, .. } => *left_col = value,
        }
    }

    /// Read the pending `new_pixel` slot. GNU `w->new_pixel`.
    pub fn new_pixel(&self) -> Option<i64> {
        match self {
            Window::Leaf { new_pixel, .. } | Window::Internal { new_pixel, .. } => *new_pixel,
        }
    }

    /// Write the pending `new_pixel` slot. GNU `wset_new_pixel`.
    pub fn set_new_pixel(&mut self, value: Option<i64>) {
        match self {
            Window::Leaf { new_pixel, .. } | Window::Internal { new_pixel, .. } => {
                *new_pixel = value;
            }
        }
    }

    /// Read the pending `new_total` slot. GNU `w->new_total`.
    pub fn new_total(&self) -> Option<i64> {
        match self {
            Window::Leaf { new_total, .. } | Window::Internal { new_total, .. } => *new_total,
        }
    }

    /// Write the pending `new_total` slot. GNU `wset_new_total`.
    pub fn set_new_total(&mut self, value: Option<i64>) {
        match self {
            Window::Leaf { new_total, .. } | Window::Internal { new_total, .. } => {
                *new_total = value;
            }
        }
    }

    /// Read the pending `new_normal` Lisp slot. GNU `w->new_normal`.
    pub fn new_normal(&self) -> Value {
        match self {
            Window::Leaf { new_normal, .. } | Window::Internal { new_normal, .. } => *new_normal,
        }
    }

    /// Write the pending `new_normal` Lisp slot.
    pub fn set_new_normal(&mut self, value: Value) {
        match self {
            Window::Leaf { new_normal, .. } | Window::Internal { new_normal, .. } => {
                *new_normal = value;
            }
        }
    }

    /// Read the persistent `normal_lines` Lisp slot. GNU
    /// `w->normal_lines`.
    pub fn normal_lines(&self) -> Value {
        match self {
            Window::Leaf { normal_lines, .. } | Window::Internal { normal_lines, .. } => {
                *normal_lines
            }
        }
    }

    /// Write the persistent `normal_lines` Lisp slot. GNU
    /// `wset_normal_lines`.
    pub fn set_normal_lines(&mut self, value: Value) {
        match self {
            Window::Leaf { normal_lines, .. } | Window::Internal { normal_lines, .. } => {
                *normal_lines = value;
            }
        }
    }

    /// Read the persistent `normal_cols` Lisp slot. GNU
    /// `w->normal_cols`.
    pub fn normal_cols(&self) -> Value {
        match self {
            Window::Leaf { normal_cols, .. } | Window::Internal { normal_cols, .. } => *normal_cols,
        }
    }

    /// Write the persistent `normal_cols` Lisp slot. GNU
    /// `wset_normal_cols`.
    pub fn set_normal_cols(&mut self, value: Value) {
        match self {
            Window::Leaf { normal_cols, .. } | Window::Internal { normal_cols, .. } => {
                *normal_cols = value;
            }
        }
    }

    /// Record this leaf as width-fixed at COLS columns.  A value of 0 clears
    /// the fixed-width constraint.
    pub fn set_fixed_width_cols(&mut self, cols: usize) {
        if let Window::Leaf { fixed_width, .. } = self {
            *fixed_width = cols;
        }
    }

    /// Record this leaf as height-fixed at LINES rows.  A value of 0 clears
    /// the fixed-height constraint.
    pub fn set_fixed_height_lines(&mut self, lines: usize) {
        if let Window::Leaf { fixed_height, .. } = self {
            *fixed_height = lines;
        }
    }

    pub fn fixed_width_cols(&self) -> usize {
        match self {
            Window::Leaf { fixed_width, .. } => *fixed_width,
            Window::Internal { .. } => 0,
        }
    }

    pub fn fixed_height_lines(&self) -> usize {
        match self {
            Window::Leaf { fixed_height, .. } => *fixed_height,
            Window::Internal { .. } => 0,
        }
    }

    /// Set the window's point from a buffer position.
    /// GNU Emacs xdisp.c:20616 syncs w->pointm from buffer PT before redisplay.
    pub fn set_point(&mut self, pos: LispCharPos1) {
        if let Window::Leaf { point, .. } = self {
            *point = pos.max(LispCharPos1::ONE);
        }
    }

    /// Horizontal scroll offset in columns (`w->hscroll`). 0 for internal
    /// nodes.
    pub fn hscroll(&self) -> usize {
        match self {
            Window::Leaf { hscroll, .. } => *hscroll,
            Window::Internal { .. } => 0,
        }
    }

    /// Lower bound for automatic horizontal scrolling (`w->min_hscroll`).
    pub fn min_hscroll(&self) -> usize {
        match self {
            Window::Leaf { min_hscroll, .. } => *min_hscroll,
            Window::Internal { .. } => 0,
        }
    }

    /// Set the lower bound for automatic horizontal scrolling
    /// (`w->min_hscroll`). No-op for internal nodes.
    pub fn set_min_hscroll(&mut self, value: usize) {
        if let Window::Leaf { min_hscroll, .. } = self {
            *min_hscroll = value;
        }
    }

    /// Whether automatic horizontal scrolling is currently suspended
    /// (`w->suspend_auto_hscroll`).
    pub fn suspend_auto_hscroll(&self) -> bool {
        match self {
            Window::Leaf {
                suspend_auto_hscroll,
                ..
            } => *suspend_auto_hscroll,
            Window::Internal { .. } => false,
        }
    }

    /// Set the auto-hscroll suspend flag (`w->suspend_auto_hscroll`). No-op
    /// for internal nodes.
    pub fn set_suspend_auto_hscroll(&mut self, value: bool) {
        if let Window::Leaf {
            suspend_auto_hscroll,
            ..
        } = self
        {
            *suspend_auto_hscroll = value;
        }
    }

    pub fn redisplay_state(&self) -> Option<WindowRedisplayState> {
        match self {
            Window::Leaf {
                id,
                buffer_id,
                bounds,
                window_start,
                window_end,
                point,
                old_point,
                hscroll,
                vscroll,
                preserve_vscroll_p,
                ..
            } => Some(WindowRedisplayState {
                id: *id,
                buffer_id: *buffer_id,
                bounds: (
                    redisplay_f32_bits(bounds.x),
                    redisplay_f32_bits(bounds.y),
                    redisplay_f32_bits(bounds.width),
                    redisplay_f32_bits(bounds.height),
                ),
                window_start: *window_start,
                window_end: *window_end,
                point: *point,
                old_point: *old_point,
                hscroll: *hscroll,
                vscroll: *vscroll,
                preserve_vscroll_p: *preserve_vscroll_p,
            }),
            Window::Internal { .. } => None,
        }
    }

    /// Window ID.
    pub fn id(&self) -> WindowId {
        match self {
            Window::Leaf { id, .. } | Window::Internal { id, .. } => *id,
        }
    }

    /// Pixel bounds.
    pub fn bounds(&self) -> &Rect {
        match self {
            Window::Leaf { bounds, .. } | Window::Internal { bounds, .. } => bounds,
        }
    }

    /// Mutable reference to bounds.
    pub fn bounds_mut(&mut self) -> &mut Rect {
        match self {
            Window::Leaf { bounds, .. } | Window::Internal { bounds, .. } => bounds,
        }
    }

    /// Set bounds.
    pub fn set_bounds(&mut self, new_bounds: Rect) {
        match self {
            Window::Leaf { bounds, .. } | Window::Internal { bounds, .. } => {
                *bounds = new_bounds;
            }
        }
    }

    /// Whether this is a leaf window.
    pub fn is_leaf(&self) -> bool {
        matches!(self, Window::Leaf { .. })
    }

    /// Return this leaf window's display state.
    pub fn display(&self) -> Option<&WindowDisplayState> {
        match self {
            Window::Leaf { display, .. } => Some(display),
            Window::Internal { .. } => None,
        }
    }

    pub fn redisplay_output(&self) -> Option<&WindowRedisplayOutput> {
        self.display()?.redisplay_output()
    }

    /// Accept every query-visible fact produced by one redisplay generation.
    ///
    /// Synchronous layout queries deliberately do not use this path: they may
    /// refresh `window-end`, but cannot masquerade as a presented redisplay.
    fn accept_redisplay_output(&mut self, output: WindowRedisplayOutput) {
        if let Self::Leaf {
            window_end,
            display,
            ..
        } = self
        {
            *window_end = WindowEndState::Current(output.window_end());
            display.redisplay_output = Some(output);
        }
    }

    /// Return a mutable reference to this leaf window's display state.
    pub fn display_mut(&mut self) -> Option<&mut WindowDisplayState> {
        match self {
            Window::Leaf { display, .. } => Some(display),
            Window::Internal { .. } => None,
        }
    }

    /// Return this window's Lisp-visible parameter alist.
    pub fn parameters(&self) -> &WindowParameters {
        match self {
            Window::Leaf { parameters, .. } | Window::Internal { parameters, .. } => parameters,
        }
    }

    /// Return a mutable reference to this window's Lisp-visible parameter alist.
    pub fn parameters_mut(&mut self) -> &mut WindowParameters {
        match self {
            Window::Leaf { parameters, .. } | Window::Internal { parameters, .. } => parameters,
        }
    }

    /// Return this live window's history state.
    pub fn history(&self) -> Option<&WindowHistoryState> {
        match self {
            Window::Leaf { history, .. } => Some(history),
            Window::Internal { .. } => None,
        }
    }

    /// Return a mutable reference to this live window's history state.
    pub fn history_mut(&mut self) -> Option<&mut WindowHistoryState> {
        match self {
            Window::Leaf { history, .. } => Some(history),
            Window::Internal { .. } => None,
        }
    }

    /// Get the combination limit for an internal window.
    pub fn combination_limit(&self) -> Option<bool> {
        match self {
            Window::Internal {
                combination_limit, ..
            } => Some(*combination_limit),
            Window::Leaf { .. } => None,
        }
    }

    /// Set the combination limit for an internal window.
    pub fn set_combination_limit(&mut self, limit: bool) {
        if let Window::Internal {
            combination_limit, ..
        } = self
        {
            *combination_limit = limit;
        }
    }

    /// Buffer displayed in this window (leaf only).
    pub fn buffer_id(&self) -> Option<BufferId> {
        match self {
            Window::Leaf { buffer_id, .. } => Some(*buffer_id),
            Window::Internal { .. } => None,
        }
    }

    /// Set the buffer displayed in this window (leaf only).
    pub fn set_buffer(&mut self, new_id: BufferId) {
        if let Window::Leaf {
            buffer_id,
            window_start,
            position_markers,
            window_end,
            point,
            min_hscroll,
            suspend_auto_hscroll,
            ..
        } = self
        {
            *buffer_id = new_id;
            *window_start = LispCharPos1::ONE;
            *position_markers = WindowPositionMarkerState::Detached;
            *window_end = WindowEndState::Unrecorded;
            *point = LispCharPos1::ONE;
            // GNU resets the auto-hscroll lower bound and the suspend flag on
            // every buffer switch (`unshow_buffer`, src/window.c:4368). The
            // `hscroll` reset itself is handled by the caller's normal
            // window-start/point reinitialization path.
            *min_hscroll = 0;
            *suspend_auto_hscroll = false;
        }
    }

    /// Stored Lisp-visible `window-end` for this leaf window.
    pub fn window_end_charpos(&self, buffer_z: LispCharPos1) -> Option<LispCharPos1> {
        match self {
            Window::Leaf { window_end, .. } => Some(window_end.charpos_from_z(buffer_z)),
            Window::Internal { .. } => None,
        }
    }

    /// Stored byte-position `window-end` for this leaf window.
    pub fn window_end_bytepos(&self, buffer_z_byte: EmacsBytePos) -> Option<EmacsBytePos> {
        match self {
            Window::Leaf { window_end, .. } => Some(window_end.bytepos_from_z(buffer_z_byte)),
            Window::Internal { .. } => None,
        }
    }

    /// Typed presentation lifecycle for this leaf window.
    pub fn window_end_state(&self) -> Option<WindowEndState> {
        match self {
            Window::Leaf { window_end, .. } => Some(*window_end),
            Window::Internal { .. } => None,
        }
    }

    /// Whether the stored window-end came from a completed redisplay.
    pub fn window_end_valid(&self) -> Option<bool> {
        self.window_end_state().map(WindowEndState::is_current)
    }

    /// Mark the last complete window-end record stale without discarding it.
    ///
    /// GNU keeps the offsets available for `window-end` with `UPDATE=nil`
    /// after invalidation, while `UPDATE=t` must recompute.
    pub fn invalidate_window_end(&mut self) {
        if let Window::Leaf { window_end, .. } = self {
            window_end.invalidate();
        }
    }

    /// Publish the last redisplay's window-end state for this leaf window.
    pub fn set_window_end_from_positions(
        &mut self,
        buffer_z_char: LispCharPos1,
        buffer_z_byte: EmacsBytePos,
        end_charpos: LispCharPos1,
        end_bytepos: EmacsBytePos,
        vpos: usize,
    ) {
        self.set_window_end_record(WindowEndRecord::from_positions(
            buffer_z_char,
            buffer_z_byte,
            end_charpos,
            end_bytepos,
            MatrixRow0::new(vpos),
        ));
    }

    /// Publish a complete char/byte/row record without reopening its tuple.
    pub fn set_window_end_record(&mut self, record: WindowEndRecord) {
        if let Window::Leaf { window_end, .. } = self {
            *window_end = WindowEndState::Current(record);
        }
    }

    /// Replace a displayed buffer id in all leaf windows under this node.
    ///
    /// This is used when a buffer is killed; any window still attached to the
    /// dead buffer is moved back to a replacement buffer (typically `*scratch*`).
    pub fn replace_buffer_id(&mut self, old_id: BufferId, new_id: BufferId) {
        match self {
            Window::Leaf { buffer_id, .. } => {
                if *buffer_id == old_id {
                    self.set_buffer(new_id);
                }
            }
            Window::Internal { children, .. } => {
                for child in children {
                    child.replace_buffer_id(old_id, new_id);
                }
            }
        }
    }

    /// Find a leaf window by ID in this subtree.
    pub fn find(&self, target: WindowId) -> Option<&Window> {
        if self.id() == target {
            return Some(self);
        }
        if let Window::Internal { children, .. } = self {
            for child in children {
                if let Some(w) = child.find(target) {
                    return Some(w);
                }
            }
        }
        None
    }

    /// Find a mutable leaf window by ID in this subtree.
    pub fn find_mut(&mut self, target: WindowId) -> Option<&mut Window> {
        if self.id() == target {
            return Some(self);
        }
        if let Window::Internal { children, .. } = self {
            for child in children {
                if let Some(w) = child.find_mut(target) {
                    return Some(w);
                }
            }
        }
        None
    }

    /// Collect all leaf window IDs.
    pub fn leaf_ids(&self) -> Vec<WindowId> {
        let mut result = Vec::new();
        self.collect_leaves(&mut result);
        result
    }

    fn collect_leaves(&self, out: &mut Vec<WindowId>) {
        match self {
            Window::Leaf { id, .. } => out.push(*id),
            Window::Internal { children, .. } => {
                for child in children {
                    child.collect_leaves(out);
                }
            }
        }
    }

    /// Find the window at pixel coordinates.
    pub fn window_at(&self, px: f32, py: f32) -> Option<WindowId> {
        match self {
            Window::Leaf { id, bounds, .. } => {
                if bounds.contains(px, py) {
                    Some(*id)
                } else {
                    None
                }
            }
            Window::Internal {
                children, bounds, ..
            } => {
                if !bounds.contains(px, py) {
                    return None;
                }
                for child in children {
                    if let Some(id) = child.window_at(px, py) {
                        return Some(id);
                    }
                }
                None
            }
        }
    }

    /// Count leaf windows in this subtree.
    pub fn leaf_count(&self) -> usize {
        match self {
            Window::Leaf { .. } => 1,
            Window::Internal { children, .. } => children.iter().map(|c| c.leaf_count()).sum(),
        }
    }

    /// Invalidate redisplay-derived window-end state for this subtree.
    pub fn invalidate_display_state(&mut self) {
        match self {
            Window::Leaf {
                window_end,
                display,
                ..
            } => {
                window_end.invalidate();
                display.clear_physical_cursor_state();
            }
            Window::Internal { children, .. } => {
                for child in children {
                    child.invalidate_display_state();
                }
            }
        }
    }

    /// Drop only horizontal scroll derived from the previous geometry.
    ///
    /// GNU distinguishes auto-hscroll state from an explicit
    /// `set-window-hscroll` with `suspend_auto_hscroll`. Geometry changes
    /// invalidate the former, while the latter remains user-owned.
    fn invalidate_automatic_hscroll_for_geometry_change(&mut self) {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum HorizontalScrollOwnership {
            Automatic,
            Explicit,
        }

        match self {
            Window::Leaf {
                hscroll,
                min_hscroll,
                suspend_auto_hscroll,
                ..
            } => {
                let ownership = if *suspend_auto_hscroll {
                    HorizontalScrollOwnership::Explicit
                } else {
                    HorizontalScrollOwnership::Automatic
                };
                match ownership {
                    HorizontalScrollOwnership::Automatic => *hscroll = *min_hscroll,
                    HorizontalScrollOwnership::Explicit => {}
                }
            }
            Window::Internal { children, .. } => {
                for child in children {
                    child.invalidate_automatic_hscroll_for_geometry_change();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Last Display Snapshot
// ---------------------------------------------------------------------------

/// Authoritative display geometry for a single visible buffer position.
///
/// These records are published by redisplay after layout so editor-side
/// queries like `posn-at-point` can answer from the actual rendered result.
/// A position can describe either a source glyph or a visible insertion
/// boundary such as end-of-buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayPointSnapshot {
    /// 1-based visible buffer position.
    pub buffer_pos: LispCharPos1,
    /// X relative to the text area's left edge, in pixels.
    pub x: i64,
    /// Y relative to the window's top edge, in pixels.
    pub y: i64,
    /// Rendered glyph or insertion-slot width in pixels.
    pub width: i64,
    /// Rendered glyph or insertion-slot height in pixels.
    pub height: i64,
    /// Visual row number in the window (0-based).
    pub row: i64,
    /// Visual column start for this position.
    pub col: i64,
}

/// Body-local row facts emitted directly by redisplay for semantic queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentedBodyRowSnapshot {
    pub output_row: i64,
    pub body_row: i64,
    pub body_y: i64,
}

/// Per-row metrics from the last redisplay of a window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisplayRowSnapshot {
    /// Visual row number in the window (0-based).
    pub row: i64,
    /// Y relative to the window's top edge, in pixels.
    pub y: i64,
    /// Row height in pixels.
    pub height: i64,
    /// X position where redisplay started emitting this row, relative to the
    /// text area's left edge.
    pub start_x: i64,
    /// Visual column where redisplay started emitting this row.
    pub start_col: i64,
    /// X position where redisplay finished emitting this row, relative to the
    /// text area's left edge.
    pub end_x: i64,
    /// Visual column where redisplay finished emitting this row.
    pub end_col: i64,
    /// First buffer position represented on this row, if any.
    pub start_buffer_pos: Option<LispCharPos1>,
    /// Last visible/source position associated with this row, if any.
    pub end_buffer_pos: Option<LispCharPos1>,
}

/// Last authoritative physical cursor kind for a window.
///
/// Mirrors GNU `enum text_cursor_kinds` for resolved physical cursor states.
/// GNU's `DEFAULT_CURSOR = -2` is intentionally absent here because
/// `phys_cursor_type` stores the cursor kind after redisplay resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(i8)]
pub enum WindowCursorKind {
    NoCursor = -1,
    FilledBox = 0,
    HollowBox = 1,
    Bar = 2,
    Hbar = 3,
}

impl WindowCursorKind {
    pub fn from_gnu_code(code: i8) -> Option<Self> {
        Self::try_from(code).ok()
    }

    pub fn gnu_code(self) -> i8 {
        self.into()
    }
}

/// Cursor position within a window's text area.
///
/// Mirrors GNU's lightweight `struct cursor_pos`; physical cursor size and
/// style live separately on `WindowCursorSnapshot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowCursorPos {
    /// X relative to the text area's left edge, in pixels.
    pub x: i64,
    /// Y relative to the text area's top edge, in pixels.
    pub y: i64,
    /// Visual row within the window's text area.
    pub row: i64,
    /// Visual column within that row.
    pub col: i64,
}

impl WindowCursorPos {
    pub fn from_snapshot(snapshot: &WindowCursorSnapshot) -> Self {
        Self {
            x: snapshot.x,
            y: snapshot.y,
            row: snapshot.row,
            col: snapshot.col,
        }
    }
}

/// Last authoritative physical cursor geometry for a window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowCursorSnapshot {
    /// Physical cursor kind that redisplay emitted for this window.
    pub kind: WindowCursorKind,
    /// X relative to the text area's left edge, in pixels.
    pub x: i64,
    /// Y relative to the text area's top edge, in pixels.
    pub y: i64,
    /// Cursor width in pixels.
    pub width: i64,
    /// Cursor height in pixels.
    pub height: i64,
    /// Pixels above the baseline.
    pub ascent: i64,
    /// Visual row within the window's text area.
    pub row: i64,
    /// Visual column within that row.
    pub col: i64,
}

/// Last authoritative redisplay geometry for a live leaf window.
pub use neomacs_display_protocol::frame_glyphs::PresentedWindowRegions;

/// Last authoritative redisplay geometry for a live leaf window.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowDisplaySnapshot {
    /// Window identifier this snapshot belongs to.
    pub window_id: WindowId,
    /// Character-grid origin stored independently from pixel geometry.
    pub cell_origin: geometry::CellOrigin,
    /// Immutable regions produced by the same completed redisplay.
    pub regions: PresentedWindowRegions,
    /// Whether redisplay materialized the window body and chrome in this snapshot.
    pub regions_materialized: bool,
    /// Body-local row mapping published by the producer, independent of chrome
    /// row counts retained for compatibility.
    pub body_rows: Vec<PresentedBodyRowSnapshot>,
    /// Text-area offset from the window's left edge, in pixels.
    pub text_area_left_offset: i64,
    /// Last redisplay mode-line height in pixels.
    pub mode_line_height: i64,
    /// Last redisplay header-line height in pixels.
    pub header_line_height: i64,
    /// Last redisplay tab-line height in pixels.
    pub tab_line_height: i64,
    /// Intended cursor position in the redisplay result, even when no physical
    /// cursor was emitted.
    pub logical_cursor: Option<WindowCursorPos>,
    /// Last redisplay physical cursor geometry for this window, if the cursor
    /// was shown.
    pub phys_cursor: Option<WindowCursorSnapshot>,
    /// Visible source-position geometry, sorted by `buffer_pos`.
    pub points: Vec<DisplayPointSnapshot>,
    /// Visible row metrics, sorted by `row`.
    pub rows: Vec<DisplayRowSnapshot>,
    /// The displayed buffer's `modified_tick` when this snapshot was produced.
    /// The snapshot is a redisplay cache; a display primitive that reads it
    /// (e.g. `vertical-motion` with a column target) must treat it as valid
    /// only while the buffer is unchanged since that redisplay — GNU always
    /// recomputes from current buffer state. `None` disables the staleness
    /// gate (used by test fixtures that install a synthetic snapshot).
    pub buffer_modiff: Option<i64>,
    /// Exact end record produced by the same row walk as this snapshot.
    pub window_end_record: Option<WindowEndRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowVisibleBufferSpan {
    start: LispCharPos1,
    end: LispCharPos1,
}

impl WindowVisibleBufferSpan {
    pub const fn new(start: LispCharPos1, end: LispCharPos1) -> Self {
        Self { start, end }
    }

    pub const fn start(self) -> LispCharPos1 {
        self.start
    }

    pub const fn end(self) -> LispCharPos1 {
        self.end
    }

    /// Convert the inclusive redisplay span into GNU's Lisp-visible
    /// `window-end` position.
    ///
    /// Published row ends normally identify the last displayed character, so
    /// `window-end` is the following insertion position. Redisplay also
    /// publishes Z itself for a visible EOB insertion slot; that boundary is
    /// already a Lisp position and must not be advanced past `point-max`.
    pub fn lisp_window_end(self, buffer_z: LispCharPos1) -> LispCharPos1 {
        if self.end >= buffer_z {
            buffer_z
        } else {
            LispCharPos1::new(self.end.as_i64().saturating_add(1))
        }
    }
}

impl WindowDisplaySnapshot {
    /// Height of header/tab chrome above the window's text body.
    pub fn top_chrome_height(&self) -> i64 {
        self.header_line_height
            .max(0)
            .saturating_add(self.tab_line_height.max(0))
    }

    /// Number of rendered header/tab rows above the window's text body.
    pub fn top_chrome_rows(&self) -> i64 {
        i64::from(self.header_line_height > 0) + i64::from(self.tab_line_height > 0)
    }

    /// Convert a redisplay Y relative to the window top into GNU's text-area
    /// coordinate space used by text `posn` values.
    pub fn text_area_relative_y(&self, window_relative_y: i64) -> i64 {
        window_relative_y.saturating_sub(self.top_chrome_height())
    }

    /// Convert a redisplay row relative to the window top into GNU's text-area
    /// row used by text `posn` values.
    pub fn text_area_relative_row(&self, window_relative_row: i64) -> i64 {
        window_relative_row.saturating_sub(self.top_chrome_rows())
    }

    /// Resolve the published body-row entry for an output row.
    ///
    /// `body_rows` is sorted by `output_row` and deduplicated at
    /// construction (window_output.rs seals every snapshot through the same
    /// sort+dedup), so a binary search is exact. Hit-test compilation calls
    /// this once per glyph point — a linear scan here was O(points x rows)
    /// per window per frame.
    pub fn body_row_for_output_row(&self, output_row: i64) -> Option<&PresentedBodyRowSnapshot> {
        self.body_rows
            .binary_search_by_key(&output_row, |row| row.output_row)
            .ok()
            .map(|idx| &self.body_rows[idx])
    }

    /// Resolve the body-local coordinates published for an output row.
    ///
    /// New redisplay producers publish this mapping directly.  The chrome
    /// subtraction is retained only for snapshots from producers that do not
    /// yet materialize body rows (notably the synchronous TTY path).
    pub fn text_body_position(&self, output_row: i64, window_y: i64) -> (i64, i64) {
        self.body_row_for_output_row(output_row).map_or_else(
            || {
                (
                    self.text_area_relative_row(output_row),
                    self.text_area_relative_y(window_y),
                )
            },
            |row| (row.body_row, row.body_y),
        )
    }

    pub fn logical_cursor_pos(&self) -> Option<WindowCursorPos> {
        self.logical_cursor.or_else(|| {
            self.phys_cursor
                .as_ref()
                .map(WindowCursorPos::from_snapshot)
        })
    }

    pub fn visible_buffer_span(&self) -> Option<WindowVisibleBufferSpan> {
        let start = self
            .rows
            .iter()
            .find_map(|row| row.start_buffer_pos)
            .or_else(|| self.points.first().map(|point| point.buffer_pos))?;
        let end = self
            .rows
            .iter()
            .rev()
            .find_map(|row| row.end_buffer_pos)
            .or_else(|| self.points.last().map(|point| point.buffer_pos))?;
        Some(WindowVisibleBufferSpan::new(start, end))
    }

    fn row_for_buffer_pos(&self, pos: LispCharPos1) -> Option<&DisplayRowSnapshot> {
        self.rows.iter().find(|row| {
            let Some(start) = row.start_buffer_pos else {
                return false;
            };
            let Some(end) = row.end_buffer_pos else {
                return false;
            };
            start <= pos && pos <= end
        })
    }

    /// Return the visible point for POS, or the nearest visible neighbor when
    /// POS itself is hidden by redisplay within the visible span.
    ///
    /// Off-window positions return `None`, matching GNU Emacs `posn-at-point`
    /// and `pos-visible-in-window-p` semantics.
    pub fn point_for_buffer_pos(&self, pos: LispCharPos1) -> Option<&DisplayPointSnapshot> {
        if self.points.is_empty() {
            return None;
        }
        let visible_span = self.visible_buffer_span()?;
        if pos < visible_span.start() || pos > visible_span.end() {
            return None;
        }
        let idx = self.points.partition_point(|point| point.buffer_pos < pos);
        if self
            .points
            .get(idx)
            .is_some_and(|point| point.buffer_pos == pos)
        {
            self.points.get(idx)
        } else {
            let row = self.row_for_buffer_pos(pos)?;
            let next_on_row = self
                .points
                .iter()
                .find(|point| point.row == row.row && point.buffer_pos > pos);
            let prev_on_row = self
                .points
                .iter()
                .rev()
                .find(|point| point.row == row.row && point.buffer_pos < pos);
            match (prev_on_row, next_on_row) {
                // GNU `posn-at-point` may report neighboring positions when
                // the requested buffer position is hidden by redisplay
                // within the same visible row, but it returns nil when the
                // position is not visible at all.
                (Some(_), Some(next)) => Some(next),
                _ => None,
            }
        }
    }

    /// Return the visible point nearest to window-relative coordinates.
    ///
    /// `x` is relative to the text area's left edge. `y` is relative to the
    /// window's top edge, matching GNU Emacs `posn-at-x-y` conventions.
    pub fn point_at_coords(&self, x: i64, y: i64) -> Option<DisplayPointSnapshot> {
        let row = self
            .rows
            .iter()
            .find(|row| y >= row.y && y < row.y.saturating_add(row.height.max(1)))?;
        let mut row_points: Vec<_> = self
            .points
            .iter()
            .filter(|point| point.row == row.row)
            .collect();
        row_points.sort_by_key(|point| (point.x, point.col, point.buffer_pos));
        let mut row_points = row_points.into_iter();
        let Some(mut last) = row_points.next() else {
            return row.start_buffer_pos.map(|buffer_pos| DisplayPointSnapshot {
                buffer_pos,
                x: row.start_x,
                y: row.y,
                width: 0,
                height: row.height.max(1),
                row: row.row,
                col: row.start_col,
            });
        };
        if x <= last.x {
            return Some(last.clone());
        }
        for point in row_points {
            let right = last.x.saturating_add(last.width.max(1));
            if x < right {
                return Some(last.clone());
            }
            if x < point.x {
                return Some(last.clone());
            }
            last = point;
        }
        Some(last.clone())
    }

    /// Row metrics for visual row ROW.
    pub fn row_metrics(&self, row: i64) -> Option<&DisplayRowSnapshot> {
        self.rows.iter().find(|metrics| metrics.row == row)
    }
}

impl Default for WindowDisplaySnapshot {
    fn default() -> Self {
        Self {
            window_id: WindowId(0),
            cell_origin: geometry::CellOrigin::default(),
            regions: PresentedWindowRegions::default(),
            regions_materialized: false,
            body_rows: Vec::new(),
            text_area_left_offset: 0,
            mode_line_height: 0,
            header_line_height: 0,
            tab_line_height: 0,
            logical_cursor: None,
            phys_cursor: None,
            points: Vec::new(),
            rows: Vec::new(),
            buffer_modiff: None,
            window_end_record: None,
        }
    }
}

/// Redisplay-owned runtime state used to decide which GNU window hooks fire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowHookSnapshot {
    /// Buffer currently shown in the window.
    pub buffer_id: BufferId,
    /// Last known live bounds of the window.
    pub bounds: Rect,
}

/// Per-frame redisplay record for GNU window change hook ownership.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameWindowHookRecord {
    /// Last known live windows on the frame.
    pub windows: HashMap<WindowId, WindowHookSnapshot>,
    /// Selected window the last time window change hooks were recorded.
    pub selected_window: Option<WindowId>,
    /// Whether this frame was the selected frame at last record time.
    pub was_selected_frame: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingGuiResize {
    pub width_cols: i64,
    pub total_lines: i64,
    pub host_request_sent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuiFrameGeometryHints {
    pub base_width: u32,
    pub base_height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub width_inc: u32,
    pub height_inc: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct PreparedDisplayPresentation {
    geometry: geometry::PresentationGeometry,
    snapshots: Vec<WindowDisplaySnapshot>,
}

#[derive(Default)]
struct FramePresentationState {
    prepared: HashMap<geometry::PresentationId, PreparedDisplayPresentation>,
    active: Option<geometry::PresentationGeometry>,
    last_identity: Option<geometry::PresentationId>,
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FrameDisplayIdentity {
    #[default]
    None,
    Wayland(String),
    X11(String),
}

impl FrameDisplayIdentity {
    pub fn wayland(display: impl Into<String>) -> Self {
        Self::Wayland(display.into())
    }

    pub fn x11(display: impl Into<String>) -> Self {
        Self::X11(display.into())
    }

    pub fn native_display(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Wayland(display) | Self::X11(display) => Some(display),
        }
    }

    pub fn x_display(&self) -> Option<&str> {
        match self {
            Self::X11(display) => Some(display),
            Self::None | Self::Wayland(_) => None,
        }
    }
}

/// A frame (top-level window/screen).
pub struct Frame {
    pub id: FrameId,
    /// GNU `struct frame.name`: a Lisp string used for resources and default
    /// title fallback.
    pub name: Value,
    /// GNU `struct frame.explicit_name`: whether the frame name came from an
    /// explicit Lisp-side parameter rather than an auto-generated `F<num>`
    /// fallback.
    pub explicit_name: bool,
    /// GNU `struct frame.icon_name`: explicit icon name, or nil.
    pub icon_name: Value,
    /// GNU `struct frame.focus_frame`: frame receiving this frame's keystrokes,
    /// or nil when focus is not redirected.
    pub focus_frame: Value,
    /// GNU `struct frame.parent_frame`: parent frame for child frames, or nil.
    pub parent_frame: Value,
    /// Terminal owner id for GNU `frame-terminal` / terminal lifecycle.
    pub terminal_id: u64,
    /// GNU `FRAME_INITIAL_P`: bootstrap placeholder frame that exists before
    /// a real terminal or window-system frame is installed.
    pub initial: bool,
    /// Root of the window tree.
    pub root_window: Window,
    /// The selected (active) window.
    pub selected_window: WindowId,
    /// The previously-selected window. GNU stores this as
    /// `frame->old_selected_window` and returns it from
    /// `frame-old-selected-window` (`src/frame.c`). Window audit
    /// Critical 8 in `drafts/window-system-audit.md` flagged the
    /// builtin as a stub returning nil because this field did not
    /// exist; the builtin now reads it.
    ///
    /// Initialized to `None` (nil) on a fresh frame to match GNU
    /// `make_frame_without_minibuffer`, then set to whichever
    /// window was previously selected on every `select-window`,
    /// `set-frame-selected-window`, and `set-window-configuration`
    /// transition.
    pub old_selected_window: Option<WindowId>,
    /// Minibuffer window (always a leaf).
    pub minibuffer_window: Option<WindowId>,
    /// Storage for the minibuffer leaf, which is not part of the split tree.
    pub minibuffer_leaf: Option<Window>,
    /// Frame pixel dimensions.
    pub width: u32,
    pub height: u32,
    /// Physical device pixels per logical Emacs pixel for this native frame.
    /// TTY and X11 frames use 1.0; Wayland can publish fractional values.
    pub device_scale_factor: f64,
    /// Pixel/cell position of the frame on its display, or relative to its
    /// parent for child frames.
    pub left_pos: i64,
    pub top_pos: i64,
    /// Internal window-system kind, mirroring GNU Emacs frame state rather
    /// than the mutable Lisp-visible frame parameter alist.
    pub window_system: Option<Value>,
    /// Native display connection plus the X display inherited by child
    /// processes. These differ for a Wayland frame running alongside
    /// Xwayland, just as GNU's PGTK backend distinguishes its GDK display from
    /// the `DISPLAY` exported to subprocesses.
    display_identity: FrameDisplayIdentity,
    /// Frame parameters.
    ///
    /// Window audit Medium 12 in
    /// `drafts/window-system-audit.md`: GNU's
    /// `Fset_frame_parameter` calls into the per-toolkit
    /// backend (`x_set_*`, `pgtk_set_*`, etc.) for each parameter
    /// class (position, size, fonts, fullscreen, scroll bars).
    /// neomacs writes to this HashMap unconditionally, so a
    /// `(modify-frame-parameters f '((width . 100)))` call
    /// updates the parameter alist but does not always reach the
    /// active display backend. Wiring the dispatch is tracked as
    /// audit Phase 6.
    pub parameters: HashMap<Value, Value>,
    /// Whether the frame is visible.
    pub visible: bool,
    /// Whether the menu / tab / tool bars are actually displayed and therefore
    /// occupy rows of the window text area (mirrors GNU realizing
    /// `FRAME_MENU_BAR_LINES` into `FRAME_TOP_MARGIN` only on a shown frame).
    /// Set on interactively displayed frames; left false for non-displayed
    /// frames (e.g. `--batch`), so window-edge coordinates match GNU there.
    pub displays_chrome: bool,
    /// GNU `struct frame.title`: explicit title override, or nil.
    pub title: Value,
    /// Menu bar height in pixels.
    pub menu_bar_height: u32,
    /// Tool bar height in pixels.
    pub tool_bar_height: u32,
    /// Compact bar height in pixels.
    pub compact_bar_height: u32,
    /// Tab bar height in pixels.
    pub tab_bar_height: u32,
    /// Default font size in pixels.
    pub font_pixel_size: f32,
    /// Default character width.
    pub char_width: f32,
    /// Default character height.
    pub char_height: f32,
    /// One-shot guard used when a live default-font change updates the frame's
    /// character metrics before GNU would commit the follow-up width/height
    /// window-system resize.
    pub defer_next_gui_parameter_resize: bool,
    /// Logical GUI resize requested via frame parameters but not yet committed
    /// to the live host window.
    pub pending_gui_resize: Option<PendingGuiResize>,
    /// Authoritative last-redisplay geometry keyed by live leaf window.
    ///
    /// Window audit Medium 10 / Medium 11 in
    /// `drafts/window-system-audit.md`: GNU keeps `change_stamp`,
    /// `use_time`, `sequence_number`, `old_pixel_width`,
    /// `old_pixel_height`, `old_body_pixel_width`,
    /// `old_body_pixel_height`, and `old_buffer` directly on
    /// `struct window`. neomacs centralizes the redisplay-time
    /// geometry inside `presentation_state` and the change-detection
    /// state inside `window_hook_record`. The fields below are the
    /// neomacs-side equivalents — adding the GNU names verbatim is
    /// tracked as future work in the audit's Phase 4 plan.
    presentation_state: FramePresentationState,
    /// Latest completed layout output used for incremental redisplay and GNU
    /// output bookkeeping. This cache is not renderer-active geometry.
    redisplay_cache: HashMap<WindowId, WindowDisplaySnapshot>,
    /// Last recorded redisplay state for GNU window change hooks.
    pub(crate) window_hook_record: FrameWindowHookRecord,
    /// GNU `frame-window-state-change` flag.
    pub(crate) window_state_change: bool,
    /// Real frame-local Lisp face hash table, mirroring GNU `frame->face_hash_table`.
    pub face_hash_table: Value,
    /// Per-frame realized Lisp faces, mirroring GNU's `frame->face_hash_table`
    /// runtime surface for renderer-facing consumers.
    /// GNU `struct frame.z_order`: stacking order among sibling child frames.
    pub z_order: i32,
    /// Whether a child frame suppresses the TTY decoration border.
    pub undecorated: bool,
    /// Non-focusable frame hint.  The TTY path stores it for Lisp-visible
    /// parameter parity; input focus policy is handled elsewhere.
    pub no_accept_focus: bool,
    /// Unsplittable frame hint.
    pub no_split: bool,
    /// GNU `struct frame.buffer_list`: buffers most-recently shown in this
    /// frame, in most-recently-shown-first order.  Updated by
    /// `bury-buffer-internal` and cleaned up on buffer kill.
    pub buffer_list: Vec<BufferId>,
    /// GNU `struct frame.buried_buffer_list`: buffers buried in this frame,
    /// in most-recently-buried-first order.  Updated by
    /// `bury-buffer-internal` and cleaned up on buffer kill.
    pub buried_buffer_list: Vec<BufferId>,
}

impl Frame {
    pub fn new(
        id: FrameId,
        name: Value,
        terminal_id: u64,
        width: u32,
        height: u32,
        mut root_window: Window,
        minibuffer_window_id: WindowId,
    ) -> Self {
        // The minibuffer window id is allocated from the shared window sequence
        // counter by the caller (right after the root), so it gets #2 like GNU.
        let minibuffer_window = minibuffer_window_id;
        let minibuffer_buffer_id = root_window.buffer_id().unwrap_or(BufferId(0));
        let minibuffer_height = 16.0_f32.min(height as f32);
        let root_bounds = Rect::new(
            root_window.bounds().x,
            root_window.bounds().y,
            width as f32,
            (height as f32 - minibuffer_height).max(0.0),
        );
        resize_window_subtree(&mut root_window, root_bounds);
        let mut minibuffer_leaf = Window::new_leaf(
            minibuffer_window,
            minibuffer_buffer_id,
            Rect::new(
                root_bounds.x,
                root_bounds.y + root_bounds.height,
                width as f32,
                minibuffer_height,
            ),
        );
        if let Window::Leaf {
            window_start,
            point,
            ..
        } = &mut minibuffer_leaf
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
        let selected = root_window
            .leaf_ids()
            .first()
            .copied()
            .unwrap_or(WindowId(0));
        Self {
            id,
            name,
            explicit_name: false,
            icon_name: Value::NIL,
            focus_frame: Value::NIL,
            parent_frame: Value::NIL,
            terminal_id,
            initial: false,
            root_window,
            selected_window: selected,
            // GNU `make_frame_without_minibuffer` leaves
            // `old_selected_window` as Qnil. The first
            // `select-window` records the outgoing selection.
            old_selected_window: None,
            minibuffer_window: Some(minibuffer_window),
            minibuffer_leaf: Some(minibuffer_leaf),
            width,
            height,
            left_pos: 0,
            top_pos: 0,
            window_system: None,
            display_identity: FrameDisplayIdentity::default(),
            // GNU terminal frames expose the terminal's default colors as
            // string sentinel values, not concrete RGB colors.  `term.c`
            // initializes FRAME_FOREGROUND_PIXEL / FRAME_BACKGROUND_PIXEL to
            // FACE_TTY_DEFAULT_FG_COLOR / FACE_TTY_DEFAULT_BG_COLOR, and
            // `xfaces.c::tty_color_name` exposes those as "unspecified-fg" /
            // "unspecified-bg".  GUI frame creation overwrites these with
            // concrete black/white defaults after it installs a window system.
            parameters: {
                let mut params = HashMap::new();
                params.insert(
                    FrameParam::ForegroundColor.symbol(),
                    Value::string("unspecified-fg"),
                );
                params.insert(
                    FrameParam::BackgroundColor.symbol(),
                    Value::string("unspecified-bg"),
                );
                params.insert(FrameParam::CursorColor.symbol(), Value::string("white"));
                // GNU terminal frames expose a numeric tab-bar-lines frame
                // parameter even when the tab bar is disabled. Lisp window
                // deletion code compares it with `>`, so nil is not compatible.
                params.insert(FrameParam::TabBarLines.symbol(), Value::fixnum(0));
                params.insert(Value::symbol("minibuffer"), Value::T);
                params
            },
            visible: true,
            // Set true only once an interactive frontend displays this frame.
            displays_chrome: false,
            title: Value::NIL,
            menu_bar_height: 0,
            tool_bar_height: 0,
            compact_bar_height: 0,
            tab_bar_height: 0,
            font_pixel_size: 16.0,
            char_width: 8.0,
            char_height: 16.0,
            device_scale_factor: 1.0,
            defer_next_gui_parameter_resize: false,
            pending_gui_resize: None,
            presentation_state: FramePresentationState::default(),
            redisplay_cache: HashMap::new(),
            window_hook_record: FrameWindowHookRecord::default(),
            window_state_change: false,
            face_hash_table: Value::hash_table(HashTableTest::Eq),
            z_order: 0,
            undecorated: false,
            no_accept_focus: false,
            no_split: false,
            buffer_list: Vec::new(),
            buried_buffer_list: Vec::new(),
        }
    }

    pub fn name_value(&self) -> Value {
        self.name
    }

    pub fn title_value(&self) -> Value {
        self.title
    }

    pub fn explicit_name_value(&self) -> Value {
        Value::bool_val(self.explicit_name)
    }

    pub fn icon_name_value(&self) -> Value {
        self.icon_name
    }

    pub fn focus_frame_value(&self) -> Value {
        self.focus_frame
    }

    pub fn name_runtime_string_owned(&self) -> String {
        self.name
            .as_lisp_string()
            .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
            .unwrap_or_default()
    }

    pub fn title_runtime_string_owned(&self) -> Option<String> {
        self.title
            .as_lisp_string()
            .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
    }

    pub fn host_title_runtime_string_owned(&self) -> String {
        self.title_runtime_string_owned()
            .filter(|title| !title.is_empty())
            .or_else(|| {
                let name = self.name_runtime_string_owned();
                (!name.is_empty()).then_some(name)
            })
            .unwrap_or_else(|| "Neomacs".to_string())
    }

    pub fn host_title_lisp_string(&self) -> crate::heap_types::LispString {
        self.title
            .as_lisp_string()
            .filter(|ls| !ls.as_bytes().is_empty())
            .or_else(|| {
                self.name
                    .as_lisp_string()
                    .filter(|ls| !ls.as_bytes().is_empty())
            })
            .cloned()
            .unwrap_or_else(|| crate::heap_types::LispString::from_utf8("Neomacs"))
    }

    pub fn generated_name_runtime_string(&self) -> String {
        let ordinal = if self.id.0 >= FRAME_ID_BASE {
            self.id.0 - FRAME_ID_BASE + 1
        } else {
            self.id.0
        };
        format!("F{ordinal}")
    }

    pub fn generated_name_value(&self) -> Value {
        Value::string(self.generated_name_runtime_string())
    }

    pub fn set_name_value(&mut self, name: Value) {
        self.explicit_name = true;
        self.name = name;
    }

    pub fn set_generated_name_value(&mut self, name: Value) {
        self.explicit_name = false;
        self.name = name;
    }

    pub fn set_name_parameter_value(&mut self, name: Value) {
        if name.is_nil() {
            self.set_generated_name_value(self.generated_name_value());
        } else {
            self.set_name_value(name);
        }
    }

    pub fn set_title_value(&mut self, title: Value) {
        self.title = title;
    }

    pub fn clear_title(&mut self) {
        self.title = Value::NIL;
    }

    /// Recalculate minibuffer bounds after an operation that changes only the
    /// window tree.
    ///
    /// This must not call `sync_window_area_bounds`: GNU's `window-resize-apply`
    /// has already computed child sizes, and resyncing the whole frame would
    /// redistribute the tree and lose those sizes.
    pub fn recalculate_minibuffer_bounds(&mut self) {
        self.reposition_minibuffer_below_root();
    }

    /// Get the selected window.
    pub fn selected_window(&self) -> Option<&Window> {
        self.find_window(self.selected_window)
    }

    /// Get a mutable reference to the selected window.
    pub fn selected_window_mut(&mut self) -> Option<&mut Window> {
        self.find_window_mut(self.selected_window)
    }

    /// Replace all leaf window buffer bindings for `old_id` with `new_id`.
    pub fn replace_buffer_bindings(&mut self, old_id: BufferId, new_id: BufferId) {
        self.root_window.replace_buffer_id(old_id, new_id);
        if let Some(minibuffer_leaf) = self.minibuffer_leaf.as_mut() {
            minibuffer_leaf.replace_buffer_id(old_id, new_id);
        }
    }

    /// Return the effective window-system symbol for this frame.
    pub fn effective_window_system(&self) -> Option<Value> {
        self.window_system
            .filter(|value| value.is_truthy())
            .or_else(|| {
                self.parameter("window-system")
                    .filter(|value| value.is_truthy())
            })
    }

    /// Update the frame's internal window-system kind and keep the Lisp-visible
    /// frame parameter in sync.
    pub fn set_window_system(&mut self, window_system: Option<Value>) {
        self.window_system = window_system;
        match window_system {
            Some(value) => {
                self.set_parameter(Value::symbol("window-system"), value);
            }
            None => {
                self.remove_parameter(Value::symbol("window-system"));
            }
        }
    }

    pub fn frame_parameter_int(&self, key: &str) -> Option<i64> {
        self.parameter(key).and_then(|v| v.as_int())
    }

    pub fn known_frame_parameter_int(&self, key: FrameParam) -> Option<i64> {
        self.known_parameter(key).and_then(|v| v.as_int())
    }

    fn nonnegative_frame_parameter_int(&self, key: FrameParam) -> Option<i64> {
        self.known_frame_parameter_int(key)
            .map(|value| value.max(0))
    }

    pub fn child_frame_border_width_raw(&self) -> Option<i64> {
        self.nonnegative_frame_parameter_int(FrameParam::ChildFrameBorderWidth)
    }

    pub fn internal_border_width(&self) -> i64 {
        if self.effective_window_system().is_none() {
            return 0;
        }
        if self.parent_frame.as_frame_id().is_some()
            && let Some(width) = self.child_frame_border_width_raw()
        {
            return width;
        }
        self.nonnegative_frame_parameter_int(FrameParam::InternalBorderWidth)
            .unwrap_or(0)
    }

    pub fn frame_child_frame_border_width(&self) -> i64 {
        if self.effective_window_system().is_none() {
            return 0;
        }
        self.child_frame_border_width_raw()
            .unwrap_or_else(|| self.internal_border_width())
    }

    pub fn install_gnu_gui_default_parameters(&mut self) {
        // GNU GUI ports seed these through gui_default_parameter before the
        // frame's Lisp face defaults are realized.
        self.set_known_parameter(FrameParam::ForegroundColor, Value::string("black"));
        self.set_known_parameter(FrameParam::BackgroundColor, Value::string("white"));
        self.set_known_parameter(FrameParam::MouseColor, Value::string("black"));
        self.set_known_parameter(FrameParam::CursorColor, Value::string("black"));
        self.set_known_parameter(FrameParam::BorderColor, Value::string("black"));
        self.set_known_parameter(FrameParam::CursorType, Value::symbol("box"));

        // Chrome parameters that GNU's xfns.c seeds through gui_default_parameter
        // for every GUI frame. Without these, `(frame-parameter f 'vertical-scroll-bars)`
        // and friends report nil even though the layout already reserves and draws
        // the corresponding chrome — the frame parameter lies about what is on screen.
        //
        // Each value equals the fallback the layout (window/display.rs) already
        // applies for the missing parameter, so seeding them changes reporting only,
        // never geometry: GTK builds default vertical scroll bars to the right,
        // horizontal off, the standard 8px fringes, and a zero internal/outer border.
        // scroll-bar-width is intentionally left unset: its fallback tracks the live
        // char width, so a fixed seed would go stale when the frame font changes
        // (GNU re-resolves it on font change; our fallback already follows the font).
        self.set_known_parameter(FrameParam::VerticalScrollBars, Value::symbol("right"));
        self.set_known_parameter(FrameParam::HorizontalScrollBars, Value::NIL);
        self.set_known_parameter(FrameParam::LeftFringe, Value::fixnum(8));
        self.set_known_parameter(FrameParam::RightFringe, Value::fixnum(8));
        self.set_known_parameter(FrameParam::InternalBorderWidth, Value::fixnum(0));
        self.set_known_parameter(FrameParam::BorderWidth, Value::fixnum(0));
    }

    pub fn parameter(&self, key: &str) -> Option<Value> {
        self.parameters.get(&Value::symbol(key)).copied()
    }

    pub fn set_display_identity(&mut self, identity: FrameDisplayIdentity) {
        if let Some(display) = identity.native_display() {
            self.set_parameter(Value::symbol("display"), Value::string(display));
        } else {
            self.remove_parameter(Value::symbol("display"));
        }
        self.display_identity = identity;
    }

    pub fn display_identity(&self) -> &FrameDisplayIdentity {
        &self.display_identity
    }

    pub fn known_parameter(&self, key: FrameParam) -> Option<Value> {
        self.parameters.get(&key.symbol()).copied()
    }

    pub fn parameter_key(&self, key: FrameParamKey) -> Option<Value> {
        self.parameters.get(&key.symbol()).copied()
    }

    pub fn set_parameter(&mut self, key: Value, value: Value) -> Option<Value> {
        self.parameters.insert(key, value)
    }

    pub fn set_known_parameter(&mut self, key: FrameParam, value: Value) -> Option<Value> {
        self.set_parameter(key.symbol(), value)
    }

    pub fn set_parameter_key(&mut self, key: FrameParamKey, value: Value) -> Option<Value> {
        self.set_parameter(key.symbol(), value)
    }

    pub fn remove_parameter(&mut self, key: Value) -> Option<Value> {
        self.parameters.remove(&key)
    }

    pub fn remove_known_parameter(&mut self, key: FrameParam) -> Option<Value> {
        self.remove_parameter(key.symbol())
    }

    pub fn remove_parameter_key(&mut self, key: FrameParamKey) -> Option<Value> {
        self.remove_parameter(key.symbol())
    }

    pub fn face_hash_table(&self) -> Value {
        self.face_hash_table
    }

    pub fn defer_next_gui_parameter_resize(&mut self) {
        self.defer_next_gui_parameter_resize = true;
    }

    pub fn should_defer_gui_parameter_resize(&self) -> bool {
        self.defer_next_gui_parameter_resize || self.pending_gui_resize.is_some()
    }

    pub fn queue_pending_gui_resize(
        &mut self,
        width_cols: i64,
        total_lines: i64,
        host_request_sent: bool,
    ) {
        self.defer_next_gui_parameter_resize = false;
        self.pending_gui_resize = Some(PendingGuiResize {
            width_cols,
            total_lines,
            host_request_sent,
        });
    }

    pub fn take_pending_gui_resize(&mut self) -> Option<PendingGuiResize> {
        self.defer_next_gui_parameter_resize = false;
        self.pending_gui_resize.take()
    }

    pub fn clear_pending_gui_resize(&mut self) {
        self.defer_next_gui_parameter_resize = false;
        self.pending_gui_resize = None;
    }

    pub fn gui_geometry_hints(&self) -> GuiFrameGeometryHints {
        let width_inc = self.char_width.max(1.0).round() as u32;
        let height_inc = self.char_height.max(1.0).round() as u32;
        let base_width = width_inc.saturating_add(self.horizontal_non_text_width().max(0) as u32);
        let base_height = height_inc.saturating_add(
            self.menu_bar_height
                .saturating_add(self.tool_bar_height)
                .saturating_add(self.compact_bar_height)
                .saturating_add(self.tab_bar_height),
        );
        GuiFrameGeometryHints {
            base_width,
            base_height,
            min_width: base_width,
            min_height: base_height,
            width_inc,
            height_inc,
        }
    }

    fn chrome_top_height(&self) -> f32 {
        self.menu_bar_height
            .saturating_add(self.tool_bar_height)
            .saturating_add(self.compact_bar_height)
            .saturating_add(self.tab_bar_height) as f32
    }

    pub fn child_frame_viewport_origin(&self) -> (f32, f32) {
        (0.0, self.chrome_top_height().min(self.height as f32))
    }

    fn default_left_fringe_width(&self) -> i64 {
        self.known_parameter(FrameParam::LeftFringe)
            .and_then(|v| v.as_int())
            .unwrap_or(8)
            .max(0)
    }

    fn default_right_fringe_width(&self) -> i64 {
        self.known_parameter(FrameParam::RightFringe)
            .and_then(|v| v.as_int())
            .unwrap_or(8)
            .max(0)
    }

    fn default_vertical_scroll_bar_side(&self) -> Option<&'static str> {
        let raw = self
            .known_parameter(FrameParam::VerticalScrollBars)
            .unwrap_or_else(|| {
                if self.effective_window_system().is_some() {
                    Value::symbol("right")
                } else {
                    Value::NIL
                }
            });
        match VerticalScrollBarType::from_symbol_value(&raw) {
            Some(side) => Some(side.name()),
            _ if raw.is_nil() => None,
            _ if raw.is_truthy() => Some("right"),
            _ => None,
        }
    }

    fn default_vertical_scroll_bar_width(&self) -> i64 {
        self.known_parameter(FrameParam::ScrollBarWidth)
            .and_then(|v| v.as_int())
            .filter(|value| *value > 0)
            .unwrap_or_else(|| self.char_width.max(1.0).round() as i64)
    }

    pub(crate) fn horizontal_non_text_width(&self) -> i64 {
        if self.effective_window_system().is_none() {
            return 0;
        }

        let left_fringe = self.default_left_fringe_width();
        let right_fringe = self.default_right_fringe_width();
        let scroll_bar_width = if self.default_vertical_scroll_bar_side().is_some() {
            self.default_vertical_scroll_bar_width()
        } else {
            0
        };

        left_fringe
            .saturating_add(right_fringe)
            .saturating_add(scroll_bar_width)
    }

    fn window_text_area_bounds_with_chrome(&self, reserve_chrome: bool) -> Rect {
        let frame_w = self.width as f32;
        let frame_h = self.height as f32;
        // The menu / tab / tool bars reduce the window text area only when they
        // are actually displayed.  GNU realizes `FRAME_MENU_BAR_LINES` (and the
        // tab/tool bars) into the frame's top margin only on a frame that is
        // being shown; a non-displayed frame (e.g. `--batch`, where the oracle
        // checks `window-edges`) keeps `frame-total-lines == frame-text-lines`
        // and the root window at line 0 even though `menu-bar-lines` is 1.
        // `displays_chrome` mirrors that: it is set on interactively displayed
        // frames and left false otherwise, so window-edge coordinates match GNU
        // in batch while interactive frames place windows below the chrome.
        let chrome_top = if reserve_chrome {
            self.chrome_top_height().min(frame_h)
        } else {
            0.0
        };
        let border = self.internal_border_width().max(0) as f32;
        let horizontal_border = border.min(frame_w / 2.0);
        let available_height = (frame_h - chrome_top).max(0.0);
        let vertical_border = border.min(available_height / 2.0);
        let content_height = (available_height - 2.0 * vertical_border).max(0.0);
        let minibuffer_height = self
            .minibuffer_leaf
            .as_ref()
            .map(|mini| mini.bounds().height.max(0.0))
            .unwrap_or(0.0)
            .min(content_height);
        let root_height = (content_height - minibuffer_height).max(0.0);
        Rect::new(
            horizontal_border,
            chrome_top + vertical_border,
            (frame_w - 2.0 * horizontal_border).max(0.0),
            root_height,
        )
    }

    fn window_text_area_bounds(&self) -> Rect {
        self.window_text_area_bounds_with_chrome(self.displays_chrome)
    }

    pub fn sync_window_area_bounds(&mut self) {
        let root_bounds = self.window_text_area_bounds();
        resize_window_subtree(&mut self.root_window, root_bounds);
        sync_window_character_edges_from_bounds(
            &mut self.root_window,
            self.char_width,
            self.char_height,
        );

        self.reposition_minibuffer_below_root();
    }

    /// Reconcile a restored window tree with the frame's current geometry.
    ///
    /// GNU `Fset_window_configuration` restores the saved window fields and
    /// then calls `adjust_frame_size` (`window.c`).  That final pass realizes
    /// the current `FRAME_TOP_MARGIN` even when the saved initial batch-frame
    /// tree still spans the pre-menu-bar area.  Keep this transition separate
    /// from ordinary batch-frame initialization: the latter intentionally
    /// retains its initial `(0, 0)` root until a reconciliation point occurs.
    pub fn reconcile_restored_window_configuration_geometry(&mut self) {
        let root_bounds = self.window_text_area_bounds_with_chrome(true);
        resize_window_subtree(&mut self.root_window, root_bounds);
        self.root_window.set_left_col(0);
        self.root_window.set_top_line(self.frame_top_margin());
        sync_window_character_edges_from_bounds(
            &mut self.root_window,
            self.char_width,
            self.char_height,
        );
        self.reposition_minibuffer_below_root();
    }

    pub fn reposition_minibuffer_below_root(&mut self) {
        let root_bounds = *self.root_window.bounds();
        // GNU `resize_frame_windows` / `Fwindow_resize_apply_total` place the
        // minibuffer's character-line edge directly below the root:
        // `m->top_line = r->top_line + r->total_lines` (window.c:5127/5026).
        // Because the top margin adds a line to the root's `top_line` but
        // removes one from its `total_lines` (the menu-bar row has 0 pixel
        // height in batch), that sum collapses to the root's *pixel* bottom --
        // the minibuffer sits below the margin and so carries no offset, its
        // character-line top equalling its pixel row.
        let root_left_col = self.root_window.left_col();
        let char_h = self.char_height.max(1.0);
        let mini_top_line = ((root_bounds.y + root_bounds.height) / char_h).round() as i64;
        if let Some(mini) = self.minibuffer_leaf.as_mut() {
            let mini_h = mini
                .bounds()
                .height
                .max(0.0)
                .min((self.height as f32 - (root_bounds.y + root_bounds.height)).max(0.0));
            mini.set_bounds(Rect::new(
                root_bounds.x,
                root_bounds.y + root_bounds.height,
                root_bounds.width,
                mini_h,
            ));
            mini.set_top_line(mini_top_line);
            mini.set_left_col(root_left_col);
            mini.invalidate_display_state();
        }

        self.root_window.invalidate_display_state();
        self.redisplay_cache.clear();
    }

    pub fn sync_tab_bar_height_from_parameters(&mut self) {
        let lines = self
            .known_frame_parameter_int(FrameParam::TabBarLines)
            .unwrap_or(0)
            .max(0) as u32;
        let char_height = self.char_height.max(1.0).round() as u32;
        self.tab_bar_height = lines.saturating_mul(char_height);
        self.sync_window_area_bounds();
    }

    /// Recompute `menu_bar_height` from the `menu-bar-lines` frame parameter.
    ///
    /// Mirrors GNU `frame.c` (`x_set_menu_bar_lines` / TTY frame init at
    /// frame.c:1307-1309): `FRAME_MENU_BAR_LINES (f) = NILP (Vmenu_bar_mode) ? 0 : 1`.
    /// On TTY the menu bar takes one character row, identical to GNU's
    /// behaviour, so the resulting pixel height is `lines * char_height`
    /// where `char_height` is 1 for TTY frames.
    ///
    /// `chrome_top_height()` already adds `menu_bar_height` into the
    /// reserved top region used by `window_text_area_bounds()`, so calling
    /// `sync_window_area_bounds()` here is enough to push the root window
    /// (and its mode line / minibuffer) down to make room.
    pub fn sync_menu_bar_height_from_parameters(&mut self) {
        let lines = self
            .known_frame_parameter_int(FrameParam::MenuBarLines)
            .unwrap_or(0)
            .max(0) as u32;
        let char_height = self.char_height.max(1.0).round() as u32;
        self.menu_bar_height = lines.saturating_mul(char_height);
        self.sync_window_area_bounds();
    }

    /// GNU `FRAME_TOP_MARGIN(f)` (`frame.h:1132`) = `FRAME_MENU_BAR_LINES` +
    /// `FRAME_TAB_BAR_LINES` (+ tool-bar top lines) in CHARACTER lines. Unlike
    /// `chrome_top_height` (pixels, gated on `displays_chrome`), this row count
    /// applies even in batch, so a window's `top_line` sits below the menu/tab
    /// bar rows while its pixel top may be 0 (the bars have no pixel height
    /// without a display).
    pub fn frame_top_margin(&self) -> i64 {
        let menu = self
            .known_frame_parameter_int(FrameParam::MenuBarLines)
            .unwrap_or(0)
            .max(0);
        let tab = self
            .known_frame_parameter_int(FrameParam::TabBarLines)
            .unwrap_or(0)
            .max(0);
        menu + tab
    }

    /// Recompute `tool_bar_height` from the `tool-bar-lines` frame parameter.
    ///
    /// GNU stores `tool-bar-lines` as a row count and separately tracks the
    /// pixel height needed by toolbar images plus button margin/relief.  For
    /// GUI frames Neomacs follows that pixel model, scaled to the frame font
    /// pixels because our renderer works in physical frame pixels.
    pub fn sync_tool_bar_height_from_parameters(&mut self) {
        let lines = self
            .known_frame_parameter_int(FrameParam::ToolBarLines)
            .unwrap_or(0)
            .max(0) as u32;
        let line_height = if self.effective_window_system().is_some() {
            default_gui_tool_bar_line_height(self.font_pixel_size)
        } else {
            self.char_height.max(1.0).round() as u32
        };
        self.tool_bar_height = lines.saturating_mul(line_height);
        self.sync_window_area_bounds();
    }

    pub fn sync_compact_bar_height_from_parameters(&mut self) {
        let lines = self
            .frame_parameter_int("compact-bar-lines")
            .unwrap_or(0)
            .max(0) as u32;
        let line_height = if self.effective_window_system().is_some() {
            default_gui_tool_bar_line_height(self.font_pixel_size)
        } else {
            self.char_height.max(1.0).round() as u32
        };
        self.compact_bar_height = lines.saturating_mul(line_height);
        self.sync_window_area_bounds();
    }

    /// Select a window by ID.
    pub fn select_window(&mut self, id: WindowId) -> bool {
        if self.find_window(id).is_some() {
            // GNU `Fselect_window` does NOT touch
            // `frame->old_selected_window`. That field is only
            // updated by `window_change_record`, which runs from
            // `run_window_change_functions` at redisplay time
            // (`src/window.c:3954-3990`). neomacs's analog lives
            // in `builtins/hooks.rs::frame_window_hook_record_from_live_state`
            // — it stores the new "old" inside `window_hook_record`
            // and propagates it back to `Frame::old_selected_window`
            // there. Window audit Critical 8 in
            // `drafts/window-system-audit.md`.
            self.selected_window = id;
            true
        } else {
            false
        }
    }

    /// Find a window by ID.
    pub fn find_window(&self, id: WindowId) -> Option<&Window> {
        if let Some(window) = self.root_window.find(id) {
            return Some(window);
        }
        self.minibuffer_leaf.as_ref().and_then(|window| {
            if window.id() == id {
                Some(window)
            } else {
                None
            }
        })
    }

    /// Find a mutable window by ID.
    pub fn find_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        if let Some(window) = self.root_window.find_mut(id) {
            return Some(window);
        }
        self.minibuffer_leaf.as_mut().and_then(|window| {
            if window.id() == id {
                Some(window)
            } else {
                None
            }
        })
    }

    /// All leaf window IDs.
    pub fn window_list(&self) -> Vec<WindowId> {
        self.root_window.leaf_ids()
    }

    fn live_window_ids_with_minibuffer(&self) -> Vec<WindowId> {
        let mut ids = self.window_list();
        if let Some(minibuffer_leaf) = self.minibuffer_leaf.as_ref() {
            ids.push(minibuffer_leaf.id());
        }
        ids
    }

    /// Number of visible windows (leaves).
    pub fn window_count(&self) -> usize {
        self.root_window.leaf_count()
    }

    /// Find which window is at pixel coordinates.
    pub fn window_at(&self, px: f32, py: f32) -> Option<WindowId> {
        self.root_window.window_at(px, py)
    }

    /// Columns (based on default char width).
    pub fn columns(&self) -> u32 {
        (self.width as f32 / self.char_width) as u32
    }

    /// Lines (based on default char height).
    pub fn lines(&self) -> u32 {
        (self.height as f32 / self.char_height) as u32
    }

    /// Test fixture for installing a completed redisplay and replaying its live
    /// output state. Production redisplay uses the presentation transaction.
    #[cfg(test)]
    pub(crate) fn commit_redisplay_cache_for_test(
        &mut self,
        snapshots: Vec<WindowDisplaySnapshot>,
    ) {
        let generation = self
            .presentation_state
            .last_identity
            .unwrap_or_else(|| geometry::PresentationId::new(1));
        self.commit_completed_window_output(generation, &snapshots);
        self.replace_redisplay_cache_for_test(snapshots);
    }

    /// Test fixture for the synchronous prepare/activate lifecycle.
    #[cfg(test)]
    pub(crate) fn prepare_and_activate_display_presentation_for_test(
        &mut self,
        presentation: geometry::PresentationId,
        snapshots: Vec<WindowDisplaySnapshot>,
    ) -> Result<(), geometry::PresentationPrepareError> {
        self.prepare_display_presentation(presentation, snapshots)?;
        self.activate_display_presentation(presentation)
            .expect("a successfully prepared presentation must activate");
        Ok(())
    }

    /// Prepare an immutable display publication without making it visible to
    /// evaluator geometry queries. The renderer owns the later activation.
    pub fn prepare_display_presentation(
        &mut self,
        presentation: geometry::PresentationId,
        snapshots: Vec<WindowDisplaySnapshot>,
    ) -> Result<(), geometry::PresentationPrepareError> {
        let snapshots: Vec<_> = snapshots
            .into_iter()
            .filter(|snapshot| self.find_window(snapshot.window_id).is_some())
            .collect();
        let candidate = geometry::PresentationGeometry::new_with_frame_placement(
            self.id,
            presentation,
            self.parent_frame.as_frame_id().map(FrameId),
            self.left_pos,
            self.top_pos,
            self.width,
            self.height,
            self.z_order,
            snapshots.clone(),
        )
        .map_err(geometry::PresentationPrepareError::InvalidGeometry)?;
        let prepared = PreparedDisplayPresentation {
            geometry: candidate,
            snapshots,
        };
        if self
            .presentation_state
            .last_identity
            .is_some_and(|last| presentation <= last)
        {
            if self.presentation_state.active.as_ref() == Some(&prepared.geometry)
                || self.presentation_state.prepared.get(&presentation) == Some(&prepared)
            {
                return Ok(());
            }
            return Err(geometry::PresentationPrepareError::ReusedPresentation(
                presentation,
            ));
        }
        self.commit_completed_window_output(presentation, &prepared.snapshots);
        self.redisplay_cache = prepared
            .snapshots
            .iter()
            .cloned()
            .map(|snapshot| (snapshot.window_id, snapshot))
            .collect();
        self.presentation_state
            .prepared
            .insert(presentation, prepared);
        self.presentation_state.last_identity = Some(presentation);
        Ok(())
    }

    /// Activate a renderer-confirmed presentation and return the identity it
    /// replaced. Re-activating the current presentation is idempotent.
    pub fn activate_display_presentation(
        &mut self,
        presentation: geometry::PresentationId,
    ) -> Result<Option<geometry::PresentationId>, geometry::PresentationActivateError> {
        if self
            .presentation_state
            .active
            .as_ref()
            .is_some_and(|active| active.presentation() == presentation)
        {
            return Ok(None);
        }
        let prepared = self
            .presentation_state
            .prepared
            .remove(&presentation)
            .ok_or(geometry::PresentationActivateError::UnknownPresentation(
                presentation,
            ))?;
        let replaced = self
            .presentation_state
            .active
            .replace(prepared.geometry)
            .map(|geometry| geometry.presentation());
        Ok(replaced)
    }

    /// Discard a presentation that never became renderer-visible.
    pub fn discard_display_presentation(&mut self, presentation: geometry::PresentationId) -> bool {
        self.presentation_state
            .prepared
            .remove(&presentation)
            .is_some()
    }

    pub fn retire_display_presentation(&mut self, presentation: geometry::PresentationId) -> bool {
        if self
            .presentation_state
            .active
            .as_ref()
            .is_some_and(|active| active.presentation() == presentation)
        {
            self.presentation_state.active = None;
            true
        } else {
            false
        }
    }

    pub fn is_display_presentation_prepared(&self, presentation: geometry::PresentationId) -> bool {
        self.presentation_state.prepared.contains_key(&presentation)
    }

    pub fn has_prepared_display_presentations(&self) -> bool {
        !self.presentation_state.prepared.is_empty()
    }

    pub const fn active_presentation(&self) -> Option<geometry::PresentationId> {
        match &self.presentation_state.active {
            Some(geometry) => Some(geometry.presentation()),
            _ => None,
        }
    }

    /// Geometry for the presentation currently used by renderer drawing and
    /// hit testing. Prepared geometry is deliberately inaccessible here.
    pub const fn active_presentation_geometry(&self) -> Option<&geometry::PresentationGeometry> {
        self.presentation_state.active.as_ref()
    }

    /// Resolve an exact visual anchor only from renderer-active geometry.
    pub fn resolve_active_visual_anchor(
        &self,
        query: geometry::VisualAnchorQuery,
    ) -> Result<geometry::VisualAnchorGeometry, geometry::GeometryQueryError> {
        let geometry = self
            .active_presentation_geometry()
            .ok_or(geometry::GeometryQueryError::NotYetActive { frame: self.id })?;
        geometry.resolve(query)
    }

    /// Begin a GNU-shaped output pass for all live windows on this frame.
    pub fn begin_display_output_pass(&mut self) {
        let live_window_ids = self.live_window_ids_with_minibuffer();
        for wid in &live_window_ids {
            if let Some(window) = self.find_window_mut(*wid)
                && let Some(display) = window.display_mut()
            {
                display.begin_output_pass();
            }
        }
    }

    pub fn window_output_update(&mut self, window_id: WindowId) -> Option<WindowOutputUpdate<'_>> {
        let display = self.find_window_mut(window_id)?.display_mut()?;
        Some(WindowOutputUpdate::new(display))
    }

    /// Replay a completed window snapshot through the live output lifecycle.
    pub fn replay_window_output_snapshot(&mut self, snapshot: &WindowDisplaySnapshot) {
        if let Some(mut update) = self.window_output_update(snapshot.window_id) {
            update.replay_snapshot(snapshot);
        }
    }

    /// Commit one accepted redisplay attempt into GNU-shaped live per-window
    /// output state. Speculative layout never calls this; all window cursors
    /// and output progress become visible together at the presentation prepare
    /// boundary.
    fn commit_completed_window_output(
        &mut self,
        generation: geometry::PresentationId,
        snapshots: &[WindowDisplaySnapshot],
    ) {
        self.begin_display_output_pass();
        for snapshot in snapshots {
            self.replay_window_output_snapshot(snapshot);
            let Some(window_end) = snapshot.window_end_record else {
                continue;
            };
            if let Some(window) = self.find_window_mut(snapshot.window_id) {
                window.accept_redisplay_output(WindowRedisplayOutput::from_snapshot(
                    generation, snapshot, window_end,
                ));
            }
        }
    }

    /// Test fixture for replacing only the latest redisplay cache, without
    /// mutating live cursor/output state.
    #[cfg(test)]
    pub(crate) fn replace_redisplay_cache_for_test(
        &mut self,
        snapshots: Vec<WindowDisplaySnapshot>,
    ) {
        self.redisplay_cache = snapshots
            .into_iter()
            .filter(|snapshot| self.find_window(snapshot.window_id).is_some())
            .map(|snapshot| (snapshot.window_id, snapshot))
            .collect();
    }

    /// Latest completed redisplay output for WINDOW-ID, independent of which
    /// presentation is renderer-active.
    pub fn redisplay_snapshot(&self, id: WindowId) -> Option<&WindowDisplaySnapshot> {
        self.redisplay_cache.get(&id)
    }

    pub(crate) fn remove_redisplay_snapshot(&mut self, id: WindowId) {
        self.redisplay_cache.remove(&id);
    }

    /// Resize the frame and window tree to new pixel dimensions.
    pub fn resize_pixelwise(&mut self, width: u32, height: u32) {
        let horizontal_geometry_changed = self.width != width;
        self.clear_pending_gui_resize();
        self.width = width;
        self.height = height;
        self.sync_window_area_bounds();
        if horizontal_geometry_changed {
            self.root_window
                .invalidate_automatic_hscroll_for_geometry_change();
            if let Some(minibuffer) = self.minibuffer_leaf.as_mut() {
                minibuffer.invalidate_automatic_hscroll_for_geometry_change();
            }
        }

        let char_width = self.char_width.max(1.0).round();
        let char_height = self.char_height.max(1.0).round();
        let text_width = (i64::from(width) - self.horizontal_non_text_width()).max(1) as f32;
        let cols = (text_width / char_width).floor().max(1.0) as i64;
        self.set_parameter(Value::symbol("width"), Value::fixnum(cols));

        if self.effective_window_system().is_none() && self.parent_frame.as_frame_id().is_none() {
            // Top-level terminal frame. Mirror GNU frame.c line accounting:
            // FRAME_TOTAL_LINES is the whole terminal, and the 'height'
            // parameter is FRAME_LINES = FRAME_TOTAL_LINES - FRAME_MENU_BAR_LINES
            // - FRAME_TAB_BAR_LINES (the minibuffer is INCLUDED in FRAME_LINES).
            // Derive the menu/tab reservation from the LINE parameters
            // (`frame_top_margin`) rather than the transient `menu_bar_height`
            // pixel cache, which may still be 0 when a resize event lands before
            // the next redisplay syncs it. Only realized (displayed) chrome
            // reduces FRAME_LINES; a non-displayed frame (--batch) keeps
            // FRAME_TOTAL_LINES == FRAME_LINES, matching GNU's batch geometry.
            let total_terminal_lines = (self.height as f32 / char_height).floor().max(1.0) as i64;
            let top_margin = if self.displays_chrome {
                self.frame_top_margin()
            } else {
                0
            };
            let frame_lines = (total_terminal_lines - top_margin).max(1);
            let minibuffer_lines = i64::from(self.minibuffer_leaf.is_some());
            let text_lines = (frame_lines - minibuffer_lines).max(1);
            self.set_parameter(Value::symbol("height"), Value::fixnum(frame_lines));
            self.set_parameter(
                Value::symbol("neovm--frame-total-lines"),
                Value::fixnum(total_terminal_lines),
            );
            self.set_parameter(
                Value::symbol("neovm--frame-text-lines"),
                Value::fixnum(text_lines),
            );
        } else {
            let root_height = self.root_window.bounds().height;
            let text_lines = (root_height / char_height).floor().max(1.0) as i64;
            let total_lines = text_lines.saturating_add(1);
            self.set_parameter(Value::symbol("height"), Value::fixnum(total_lines));
            self.set_parameter(
                Value::symbol("neovm--frame-text-lines"),
                Value::fixnum(text_lines),
            );
        }
    }

    /// Refresh fixed-size constraints from buffers currently displayed in the
    /// window tree.
    ///
    /// GNU's `window-size-fixed-p` reads the buffer-local `window-size-fixed`
    /// of a live window's buffer. Display backends only report physical frame
    /// sizes, so Neomacs materializes that dynamic Lisp state onto the window
    /// tree immediately before low-level frame resize layout.
    pub fn sync_window_size_fixed_from_buffers(&mut self, buffers: &BufferManager) {
        let char_width = self.char_width.max(1.0);
        let char_height = self.char_height.max(1.0);

        fn fixed_axes(buffers: &BufferManager, buffer_id: BufferId) -> (bool, bool) {
            let value = buffers
                .get(buffer_id)
                .and_then(|buffer| buffer.buffer_local_value("window-size-fixed"))
                .unwrap_or(Value::NIL);
            if value.is_nil() {
                return (false, false);
            }
            if value == Value::T {
                return (true, true);
            }
            match value.as_symbol_name() {
                Some("width") => (true, false),
                Some("height") => (false, true),
                _ => (true, true),
            }
        }

        fn sync_window(
            window: &mut Window,
            buffers: &BufferManager,
            char_width: f32,
            char_height: f32,
        ) {
            match window {
                Window::Leaf {
                    buffer_id, bounds, ..
                } => {
                    let (fixed_width, fixed_height) = fixed_axes(buffers, *buffer_id);
                    let fixed_cols = if fixed_width {
                        (bounds.width / char_width).round().max(1.0) as usize
                    } else {
                        0
                    };
                    let fixed_lines = if fixed_height {
                        (bounds.height / char_height).round().max(1.0) as usize
                    } else {
                        0
                    };
                    window.set_fixed_width_cols(fixed_cols);
                    window.set_fixed_height_lines(fixed_lines);
                }
                Window::Internal { children, .. } => {
                    for child in children {
                        sync_window(child, buffers, char_width, char_height);
                    }
                }
            }
        }

        sync_window(&mut self.root_window, buffers, char_width, char_height);
        if let Some(minibuffer) = self.minibuffer_leaf.as_mut() {
            sync_window(minibuffer, buffers, char_width, char_height);
        }
    }

    /// Apply a live physical frame resize while honoring Lisp-visible window
    /// constraints carried by displayed buffers.
    pub fn resize_pixelwise_with_buffer_constraints(
        &mut self,
        buffers: &BufferManager,
        width: u32,
        height: u32,
    ) {
        self.sync_window_size_fixed_from_buffers(buffers);
        self.resize_pixelwise(width, height);
    }

    /// Grow the minibuffer window by `delta_rows` character-cell rows.
    ///
    /// Mirrors GNU `grow_mini_window` at `src/window.c:5896-5930`.
    /// The minibuffer height is clamped to the range [1 row,
    /// `max-mini-window-height` fraction of frame inner height].
    /// After adjusting the minibuffer bounds,
    /// `sync_window_area_bounds` propagates the change to the root
    /// window tree (the root shrinks by the same delta).
    pub fn grow_mini_window(&mut self, delta_rows: i32) {
        // The default `max-mini-window-height` is the float 0.25 (a fraction of
        // the frame's inner height). Resolve it to a line count so it matches
        // the lines-only contract of `*_with_max_lines`.
        let char_h = self.char_height.max(1.0);
        let frame_inner_rows = ((self.height as f32 - self.chrome_top_height()) / char_h).max(1.0);
        self.grow_mini_window_with_max_lines(delta_rows, 0.25 * frame_inner_rows);
    }

    /// Grow the minibuffer window using GNU's `max-mini-window-height`
    /// semantics resolved by the caller.
    ///
    /// `max_lines` is either an absolute line count or a frame-height
    /// fraction already converted into lines.
    pub fn grow_mini_window_with_max_lines(&mut self, delta_rows: i32, max_lines: f32) {
        // Snapshot scalar values before taking mutable borrow of minibuffer_leaf.
        let char_h = self.char_height.max(1.0);
        let unit = char_h;
        let frame_inner_h = (self.height as f32) - self.chrome_top_height();
        // `max_lines` is a resolved LINE COUNT (the caller resolves GNU
        // `max-mini-window-height` to lines: Float -> frac*frame_rows,
        // Fixnum -> the integer). GNU caps the mini-window at `lines * unit`
        // pixels (xdisp.c:13330 resize_mini_window, FIXNUMP branch), clipped to
        // [unit, frame_inner_h]. NOTE: this used to branch on `max_lines <= 1.0`
        // to re-apply the fraction, which wrongly treated an integer cap of 1
        // line (e.g. vertico-posframe's `(setq-local max-mini-window-height 1)`)
        // as 100% of the frame -> the minibuffer grew to the whole frame and
        // crushed the main window.
        let requested_max_h = unit * max_lines;
        let max_h = requested_max_h.min(frame_inner_h).max(unit);

        let Some(mini) = self.minibuffer_leaf.as_mut() else {
            return;
        };
        let current_h = mini.bounds().height;
        let new_h = (current_h + delta_rows as f32 * unit).clamp(unit, max_h);
        if (new_h - current_h).abs() < 0.5 {
            return;
        }
        let mut bounds = *mini.bounds();
        bounds.height = new_h;
        mini.set_bounds(bounds);
        self.sync_window_area_bounds();
    }

    /// Shrink the minibuffer window to its minimum height (1 row).
    ///
    /// Mirrors GNU `shrink_mini_window` at `src/window.c:5938-5960`.
    /// The freed space is returned to the root window via
    /// `sync_window_area_bounds`.
    pub fn shrink_mini_window(&mut self) {
        let Some(mini) = self.minibuffer_leaf.as_mut() else {
            return;
        };
        let unit = self.char_height.max(1.0);
        let mut bounds = *mini.bounds();
        if (bounds.height - unit).abs() < 0.5 {
            return;
        }
        bounds.height = unit;
        mini.set_bounds(bounds);
        self.sync_window_area_bounds();
    }
}

// ---------------------------------------------------------------------------
// FrameManager
// ---------------------------------------------------------------------------

/// Manages all frames and tracks the selected frame.
pub struct FrameManager {
    frames: HashMap<FrameId, Frame>,
    /// Called on each newly built Frame before it is inserted. The Lisp
    /// layer injects xfaces::init_frame_lisp_faces here (GNU calls
    /// init_frame_faces from the frame.c creation paths, not make_frame);
    /// display-side FrameManagers leave it unset.
    frame_init_hook: Option<fn(&mut Frame)>,
    selected: Option<FrameId>,
    next_frame_id: u64,
    next_window_id: u64,
    old_selected_window: Option<WindowId>,
    deleted_windows: HashSet<WindowId>,
    deleted_window_parameters: HashMap<WindowId, WindowParameters>,
    window_select_count: i64,
}

impl FrameManager {
    pub fn new() -> Self {
        Self {
            frames: HashMap::new(),
            frame_init_hook: None,
            selected: None,
            next_frame_id: FRAME_ID_BASE,
            next_window_id: 1,
            old_selected_window: None,
            deleted_windows: HashSet::new(),
            deleted_window_parameters: HashMap::new(),
            window_select_count: 0,
        }
    }

    /// Install the hook run on every newly created frame.
    pub fn set_frame_init_hook(&mut self, hook: fn(&mut Frame)) {
        self.frame_init_hook = Some(hook);
    }

    /// Allocate a new window ID.
    pub fn next_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        self.deleted_windows.remove(&id);
        self.deleted_window_parameters.remove(&id);
        id
    }

    /// Create a new frame with a single window displaying `buffer_id`.
    pub fn create_frame(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        buffer_id: BufferId,
    ) -> FrameId {
        self.create_frame_value(Value::string(name), width, height, buffer_id)
    }

    pub fn create_frame_value(
        &mut self,
        name: Value,
        width: u32,
        height: u32,
        buffer_id: BufferId,
    ) -> FrameId {
        self.create_frame_value_on_terminal(name, 0, width, height, buffer_id)
    }

    pub fn create_frame_on_terminal(
        &mut self,
        name: &str,
        terminal_id: u64,
        width: u32,
        height: u32,
        buffer_id: BufferId,
    ) -> FrameId {
        self.create_frame_value_on_terminal(
            Value::string(name),
            terminal_id,
            width,
            height,
            buffer_id,
        )
    }

    pub fn create_frame_value_on_terminal(
        &mut self,
        name: Value,
        terminal_id: u64,
        width: u32,
        height: u32,
        buffer_id: BufferId,
    ) -> FrameId {
        let frame_id = FrameId(self.next_frame_id);
        self.next_frame_id += 1;

        let window_id = self.next_window_id();
        let bounds = Rect::new(0.0, 0.0, width as f32, height as f32);
        let root = Window::new_leaf(window_id, buffer_id, bounds);

        // GNU `make_frame' creates the root window first, then the minibuffer
        // window, both drawing from the same global window sequence counter
        // (frame.c:1228/1232) -> root gets #1, minibuffer #2. Allocate the
        // minibuffer id from the counter here (immediately after the root) so
        // neomacs's window numbering matches GNU exactly.
        let minibuffer_window_id = self.next_window_id();

        let mut frame = Frame::new(
            frame_id,
            name,
            terminal_id,
            width,
            height,
            root,
            minibuffer_window_id,
        );
        if let Some(init) = self.frame_init_hook {
            init(&mut frame);
        }
        let selected_wid = frame.selected_window;
        self.frames.insert(frame_id, frame);
        self.note_window_selected(selected_wid);

        if self.selected.is_none() {
            self.selected = Some(frame_id);
            self.old_selected_window = Some(selected_wid);
        }

        frame_id
    }

    /// Get a frame by ID.
    pub fn get(&self, id: FrameId) -> Option<&Frame> {
        self.frames.get(&id)
    }

    /// Get a mutable frame by ID.
    pub fn get_mut(&mut self, id: FrameId) -> Option<&mut Frame> {
        self.frames.get_mut(&id)
    }

    /// Force the frame's minibuffer window back to exactly one line, content
    /// independent.
    ///
    /// This is neomacs's analogue of GNU's unconditional
    /// `resize_mini_window (XWINDOW (minibuf_window), 0)` at the outermost
    /// minibuffer unwind (`minibuf.c:1188-1190`). Unlike the layout-engine
    /// auto-resize heuristic (which only *reacts* to measured content and is
    /// the steady-state mechanism), this asserts the end state imperatively at
    /// teardown so the mini-window cannot stay grown after the active
    /// minibuffer is gone.
    ///
    /// It reuses the existing minibuffer split / shrink machinery
    /// (`reposition_minibuffer_below_root` via `sync_window_area_bounds`) to
    /// set the mini-window height to one line and return the freed space to the
    /// root. The next redisplay re-measures the inactive mini-window from its
    /// displayed echo buffer's *content* (see the layout engine's
    /// `echo_content_rows`), so it correctly stays one line for an empty echo
    /// or "Quit", and re-grows for a genuine multi-line message — without any
    /// stale-matrix override that would suppress such a message.
    pub fn force_resize_mini_window_to_one_line(&mut self, id: FrameId) {
        let Some(frame) = self.frames.get_mut(&id) else {
            return;
        };
        let unit = frame.char_height.max(1.0);
        if let Some(mini) = frame.minibuffer_leaf.as_mut() {
            let mut bounds = *mini.bounds();
            bounds.height = unit;
            mini.set_bounds(bounds);
            // Reset any stale vscroll left on the mini-window by
            // vertico-posframe (which scrolls the real minibuffer out of view
            // while its child frame shows the candidates). A nonzero vscroll on
            // a one-line mini-window drives `visible_max_rows` to 0 (the
            // posframe-hiding guard in the layout engine), which silently drops
            // the echo row — so after a C-g abort the "Quit"/message text is
            // laid out but produces no glyphs. GNU's `resize_mini_window`
            // resets the window's scroll on unwind; mirror that here, matching
            // the split-reset path elsewhere in this file.
            if let Window::Leaf {
                vscroll,
                preserve_vscroll_p,
                ..
            } = mini
            {
                *vscroll = 0;
                *preserve_vscroll_p = false;
            }
        }
        // Hand the freed rows back to the root window and reposition the
        // mini-window below it, exactly like `shrink_mini_window`.
        frame.sync_window_area_bounds();
    }

    pub fn frames_mut(&mut self) -> impl Iterator<Item = &mut Frame> {
        self.frames.values_mut()
    }

    /// Get the selected frame.
    pub fn selected_frame(&self) -> Option<&Frame> {
        self.selected.and_then(|id| self.frames.get(&id))
    }

    /// Get a mutable reference to the selected frame.
    pub fn selected_frame_mut(&mut self) -> Option<&mut Frame> {
        self.selected.and_then(|id| self.frames.get_mut(&id))
    }

    /// Select a frame.
    pub fn select_frame(&mut self, id: FrameId) -> bool {
        if self.frames.contains_key(&id) {
            let previous = self.selected;
            self.selected = Some(id);
            if let Some(previous) = previous {
                let previous_value = Value::make_frame(previous.0);
                let redirected_value = Value::make_frame(id.0);
                for frame in self.frames.values_mut() {
                    if frame.focus_frame == previous_value {
                        frame.focus_frame = redirected_value;
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Temporarily make `window_id` the selected window (and its frame the
    /// selected frame) for the duration of mode/tab/header-line evaluation.
    ///
    /// Mirrors GNU `display_mode_lines` (`src/xdisp.c`), which assigns
    /// `selected_window = w` and `XFRAME (new_frame)->selected_window` before
    /// walking the format and restores them via `unwind_protect`.  Without
    /// this, `:eval` forms that read `(selected-window)` — e.g.
    /// `tab-line-tabs-window-buffers`, the default `tab-line-tabs-function`
    /// (`lisp/tab-line.el`) — see the globally selected window instead of the
    /// window being redisplayed, so every window's tab line shows the
    /// selected window's buffer.  This is a lightweight assignment (no focus
    /// redirection, no hooks), matching GNU's note that full `select_window`
    /// is only needed if the format mutates point.  Pass the returned token to
    /// [`Self::restore_selected_window_for_mode_line`].
    #[must_use]
    pub fn select_window_for_mode_line(
        &mut self,
        window_id: WindowId,
    ) -> (Option<FrameId>, Option<(FrameId, WindowId)>) {
        let prev_selected_frame = self.selected;
        let frame_id = self.find_window_frame_id(window_id);
        let prev_frame_window = frame_id.and_then(|fid| {
            let frame = self.frames.get_mut(&fid)?;
            let prev = frame.selected_window;
            frame.selected_window = window_id;
            Some((fid, prev))
        });
        if let Some(fid) = frame_id {
            self.selected = Some(fid);
        }
        (prev_selected_frame, prev_frame_window)
    }

    /// Undo [`Self::select_window_for_mode_line`].
    pub fn restore_selected_window_for_mode_line(
        &mut self,
        saved: (Option<FrameId>, Option<(FrameId, WindowId)>),
    ) {
        let (prev_selected_frame, prev_frame_window) = saved;
        if let Some((fid, prev)) = prev_frame_window
            && let Some(frame) = self.frames.get_mut(&fid)
        {
            frame.selected_window = prev;
        }
        self.selected = prev_selected_frame;
    }

    /// Delete a frame.
    pub fn delete_frame(&mut self, id: FrameId) -> bool {
        if let Some(frame) = self.frames.remove(&id) {
            for wid in frame.window_list() {
                self.deleted_windows.insert(wid);
                if let Some(window) = frame.find_window(wid) {
                    self.deleted_window_parameters
                        .insert(wid, window.parameters().clone());
                }
            }
            if let Some(minibuffer_leaf) = frame.minibuffer_leaf.as_ref() {
                let minibuffer_wid = minibuffer_leaf.id();
                self.deleted_windows.insert(minibuffer_wid);
                self.deleted_window_parameters
                    .insert(minibuffer_wid, minibuffer_leaf.parameters().clone());
            }
            if self.selected == Some(id) {
                self.selected = self.frames.keys().next().copied();
            }
            true
        } else {
            false
        }
    }

    /// List all frame IDs.
    pub fn frame_list(&self) -> Vec<FrameId> {
        self.frames.keys().copied().collect()
    }

    /// Return FRAME's parent frame id, when FRAME is a child frame.
    pub fn frame_parent_id(&self, id: FrameId) -> Option<FrameId> {
        self.frames
            .get(&id)
            .and_then(|frame| frame.parent_frame.as_frame_id())
            .map(FrameId)
            .filter(|parent| self.frames.contains_key(parent))
    }

    /// Return the root frame reached by following parent-frame links.
    pub fn root_frame_id(&self, id: FrameId) -> Option<FrameId> {
        if !self.frames.contains_key(&id) {
            return None;
        }
        let mut current = id;
        let mut seen = HashSet::new();
        while seen.insert(current) {
            let Some(parent) = self.frame_parent_id(current) else {
                return Some(current);
            };
            current = parent;
        }
        Some(current)
    }

    /// Return true when ANCESTOR is in DESCENDANT's parent-frame chain.
    pub fn frame_ancestor_p(&self, ancestor: FrameId, descendant: FrameId) -> bool {
        let mut current = descendant;
        let mut seen = HashSet::new();
        while seen.insert(current) {
            let Some(parent) = self.frame_parent_id(current) else {
                return false;
            };
            if parent == ancestor {
                return true;
            }
            current = parent;
        }
        false
    }

    pub fn max_child_z_order(&self, parent: FrameId) -> i32 {
        self.frames
            .values()
            .filter(|frame| frame.parent_frame.as_frame_id() == Some(parent.0))
            .map(|frame| frame.z_order)
            .max()
            .unwrap_or(0)
    }

    pub fn set_child_z_order_above_siblings(&mut self, child: FrameId, parent: FrameId) {
        let z_order = 1 + self.max_child_z_order(parent);
        if let Some(frame) = self.frames.get_mut(&child) {
            frame.z_order = z_order;
        }
    }

    pub fn raise_or_lower_child_frame(&mut self, id: FrameId, raise: bool) {
        let Some(parent) = self.frame_parent_id(id) else {
            return;
        };
        let mut siblings: Vec<FrameId> = self
            .frames
            .iter()
            .filter_map(|(frame_id, frame)| {
                (frame.parent_frame.as_frame_id() == Some(parent.0)).then_some(*frame_id)
            })
            .collect();
        siblings.sort_by(|a, b| self.frame_z_order_cmp(*a, *b));

        let mut next_z = 0;
        for sibling in siblings {
            if sibling == id {
                continue;
            }
            if let Some(frame) = self.frames.get_mut(&sibling) {
                frame.z_order = next_z;
            }
            next_z += 1;
        }

        if let Some(frame) = self.frames.get_mut(&id) {
            frame.z_order = if raise { next_z } else { 0 };
        }
    }

    fn frame_ancestors_visible_p(&self, id: FrameId) -> bool {
        let mut current = Some(id);
        let mut seen = HashSet::new();
        while let Some(frame_id) = current {
            if !seen.insert(frame_id) {
                return false;
            }
            let Some(frame) = self.frames.get(&frame_id) else {
                return false;
            };
            if !frame.visible {
                return false;
            }
            current = self.frame_parent_id(frame_id);
        }
        true
    }

    fn frame_z_order_cmp(&self, a: FrameId, b: FrameId) -> std::cmp::Ordering {
        if a == b {
            return std::cmp::Ordering::Equal;
        }
        if self.frame_ancestor_p(a, b) {
            return std::cmp::Ordering::Less;
        }
        if self.frame_ancestor_p(b, a) {
            return std::cmp::Ordering::Greater;
        }

        let a_z = self.frames.get(&a).map(|frame| frame.z_order).unwrap_or(0);
        let b_z = self.frames.get(&b).map(|frame| frame.z_order).unwrap_or(0);
        a_z.cmp(&b_z).then_with(|| a.0.cmp(&b.0))
    }

    /// Return frames with the same root as FRAME, sorted bottom-to-top.
    ///
    /// The root frame is first.  Children and descendants follow according to
    /// GNU's TTY z-order rules, where ancestors sort below descendants.
    pub fn frames_in_reverse_z_order(&self, frame: FrameId, visible_only: bool) -> Vec<FrameId> {
        let Some(root) = self.root_frame_id(frame) else {
            return Vec::new();
        };
        let mut frames: Vec<FrameId> = self
            .frames
            .keys()
            .copied()
            .filter(|frame_id| self.root_frame_id(*frame_id) == Some(root))
            .filter(|frame_id| !visible_only || self.frame_ancestors_visible_p(*frame_id))
            .collect();
        frames.sort_by(|a, b| self.frame_z_order_cmp(*a, *b));
        frames
    }

    pub fn frame_origin_in_root(&self, id: FrameId) -> Option<(f32, f32)> {
        if !self.frames.contains_key(&id) {
            return None;
        }

        let mut x = 0_i64;
        let mut y = 0_i64;
        let mut viewport_x = 0.0_f32;
        let mut viewport_y = 0.0_f32;
        let mut current = Some(id);
        let mut seen = HashSet::new();
        while let Some(frame_id) = current {
            if !seen.insert(frame_id) {
                return None;
            }
            let frame = self.frames.get(&frame_id)?;
            x += frame.left_pos;
            y += frame.top_pos;
            current = self.frame_parent_id(frame_id);
            if let Some(parent_id) = current {
                let parent = self.frames.get(&parent_id)?;
                let (dx, dy) = parent.child_frame_viewport_origin();
                viewport_x += dx;
                viewport_y += dy;
            }
        }
        Some((x as f32 + viewport_x, y as f32 + viewport_y))
    }

    /// Resolve immutable parent-relative child placement through only accepted
    /// frame presentations. Parent chrome and root desktop offsets never enter
    /// this composition.
    pub fn place_active_frame(
        &self,
        frame: FrameId,
        presentation: geometry::PresentationId,
    ) -> Result<neomacs_display_protocol::PlacedFrame, neomacs_display_protocol::PlaceChildError>
    {
        let scene = neomacs_display_protocol::PresentedFrameScene::from_placements(
            self.frames.values().filter_map(|frame| {
                frame
                    .active_presentation_geometry()
                    .map(|geometry| geometry.frame_placement())
            }),
        )?;
        scene.place(neomacs_display_protocol::PlaceChildQuery::new(
            neomacs_display_protocol::DisplayFrameId::new(frame.0),
            neomacs_display_protocol::PresentationId::new(presentation.get()),
        ))
    }

    pub fn render_frame_tree(
        &self,
        selected_or_root: FrameId,
        visible_only: bool,
    ) -> Option<RenderFrameTree> {
        let root_id = self.root_frame_id(selected_or_root)?;
        let frames_bottom_to_top = self
            .frames_in_reverse_z_order(root_id, visible_only)
            .into_iter()
            .filter_map(|frame_id| {
                let frame = self.frames.get(&frame_id)?;
                let (origin_in_root_x, origin_in_root_y) = self.frame_origin_in_root(frame_id)?;
                Some(RenderFrameNode {
                    frame_id,
                    parent_id: self.frame_parent_id(frame_id),
                    origin_in_root_x,
                    origin_in_root_y,
                    z_order: frame.z_order,
                })
            })
            .collect();

        Some(RenderFrameTree {
            root_id,
            frames_bottom_to_top,
        })
    }

    /// Split a window horizontally or vertically.
    /// Returns the new window's ID, or None if the window wasn't found.
    ///
    /// `size` controls how space is divided:
    /// - `None` or `Some(0)`: split 50/50
    /// - `Some(n)` where n > 0: the **new** window gets `n` units (lines or
    ///   columns), the old window gets the remainder.
    /// - `Some(n)` where n < 0: the **old** window gets `|n|` units, the new
    ///   window gets the remainder.
    ///
    /// `placement` controls whether the new window is inserted before or after
    /// the split target in the parent child list. This mirrors GNU
    /// `split-window-internal` side ordering: `above`/`left` insert before the
    /// target, while `below`/`right` insert after it.
    pub fn split_window(
        &mut self,
        frame_id: FrameId,
        window_id: WindowId,
        direction: SplitDirection,
        new_buffer_id: BufferId,
        size: Option<i64>,
        placement: SplitPlacement,
    ) -> Option<WindowId> {
        self.split_window_with_combination_limit(
            frame_id,
            window_id,
            direction,
            new_buffer_id,
            size,
            placement,
            CombinationLimit::TreeDecides,
        )
    }

    /// Split a window, honoring the caller's `window-combination-limit`.
    ///
    /// [`Self::split_window`] is the same operation with the variable's
    /// default (`nil`) policy. Only the Lisp entry point
    /// (`split-window-internal`) has a dynamic binding to pass here; GNU reads
    /// it as `Vwindow_combination_limit` in `Fsplit_window_internal`
    /// (`src/window.c:5426`).
    #[allow(clippy::too_many_arguments)] // mirrors GNU's split parameter set
    pub fn split_window_with_combination_limit(
        &mut self,
        frame_id: FrameId,
        window_id: WindowId,
        direction: SplitDirection,
        new_buffer_id: BufferId,
        size: Option<i64>,
        placement: SplitPlacement,
        combination_limit: CombinationLimit,
    ) -> Option<WindowId> {
        let internal_id = self.alloc_window_id();
        let new_id = self.alloc_window_id();
        let frame = self.frames.get_mut(&frame_id)?;

        // GNU decides "interpose a new parent" vs "splice into the existing
        // combination" *before* touching the tree, from the dynamic variable
        // plus the target's position in it (`src/window.c:5423-5431`).
        let parent = parent_combination_of(&frame.root_window, window_id)?;
        let attachment = SplitAttachment::decide(combination_limit, parent, direction);

        split_window_in_tree(
            &mut frame.root_window,
            window_id,
            direction,
            internal_id,
            new_id,
            new_buffer_id,
            size,
            placement,
            attachment,
        )?;

        // GNU's split path leaves frame chrome/top-margin realization to
        // `window--pixel-to-total` / `window-resize-apply-total`.  Resyncing the
        // whole frame area here would recompute child character edges from the
        // root's menu-bar top margin and incorrectly pull side-window leaves down
        // by one line in batch.
        frame.recalculate_minibuffer_bounds();
        Some(new_id)
    }

    /// Delete a window from a frame. Cannot delete the last window.
    ///
    /// The freed space is spread over the remaining siblings. Callers coming
    /// from Lisp must use [`Self::delete_window_with_resize`] with
    /// [`DeleteResize::ApplyStaged`] instead, so that the sizes `window.el`
    /// already staged are honored.
    pub fn delete_window(&mut self, frame_id: FrameId, window_id: WindowId) -> bool {
        self.delete_window_with_resize(frame_id, window_id, DeleteResize::Redistribute)
    }

    /// Delete a window, reclaiming its space per `resize`.
    ///
    /// Mirrors GNU `Fdelete_window_internal`, which unlinks the window and then
    /// commits the staged sizes with `window_resize_apply`.
    pub fn delete_window_with_resize(
        &mut self,
        frame_id: FrameId,
        window_id: WindowId,
        resize: DeleteResize,
    ) -> bool {
        let Some(frame) = self.frames.get_mut(&frame_id) else {
            return false;
        };
        if frame.root_window.leaf_count() <= 1 {
            return false; // Can't delete last window
        }

        let deleted_parameters = frame
            .find_window(window_id)
            .map(|window| window.parameters().clone());
        let removed = delete_window_in_tree(&mut frame.root_window, window_id, resize);
        if removed {
            self.deleted_windows.insert(window_id);
            self.deleted_window_parameters
                .insert(window_id, deleted_parameters.unwrap_or_default());
            frame.recalculate_minibuffer_bounds();
        }

        if removed && frame.selected_window == window_id {
            // Select the first remaining leaf. We do NOT touch
            // `old_selected_window` here — that field is recorded
            // by `window_change_record` (GNU
            // `src/window.c:3954-3990`) at redisplay time, not
            // immediately on deletion.
            if let Some(first) = frame.root_window.leaf_ids().first() {
                frame.selected_window = *first;
            }
        }

        removed
    }

    /// Replace ROOT with its descendant WINDOW, deleting the other windows in
    /// ROOT's subtree.
    ///
    /// This mirrors GNU Emacs's `delete-other-windows-internal` /
    /// `replace_window`: WINDOW keeps its identity and window-local state but
    /// inherits ROOT's geometry and normal sizes.  Keeping this as one tree
    /// transition avoids the geometry drift caused by deleting sibling leaves
    /// one at a time.
    pub fn keep_only_window_in_subtree(
        &mut self,
        frame_id: FrameId,
        window_id: WindowId,
        root_id: WindowId,
    ) -> bool {
        let Some(frame) = self.frames.get_mut(&frame_id) else {
            return false;
        };
        let Some(root) = frame.root_window.find(root_id) else {
            return false;
        };
        let Some(mut replacement) = root.find(window_id).cloned() else {
            return false;
        };

        if window_id == root_id {
            return true;
        }

        let root_bounds = *root.bounds();
        let root_top_line = root.top_line();
        let root_left_col = root.left_col();
        let root_normal_lines = root.normal_lines();
        let root_normal_cols = root.normal_cols();

        let mut kept_ids = HashSet::new();
        collect_window_ids(&replacement, &mut kept_ids);
        let mut removed_windows = Vec::new();
        collect_window_metadata(root, &mut removed_windows);
        removed_windows.retain(|(id, _)| !kept_ids.contains(id));

        resize_window_subtree(&mut replacement, root_bounds);
        sync_window_character_edges_from_bounds_at(
            &mut replacement,
            root_left_col,
            root_top_line,
            frame.char_width,
            frame.char_height,
        );
        replacement.set_normal_lines(root_normal_lines);
        replacement.set_normal_cols(root_normal_cols);
        if let Window::Leaf {
            vscroll,
            preserve_vscroll_p,
            ..
        } = &mut replacement
        {
            *vscroll = 0;
            *preserve_vscroll_p = false;
        }
        replacement.invalidate_display_state();

        let Some(root) = frame.root_window.find_mut(root_id) else {
            return false;
        };
        *root = replacement;

        if let Some(kept_subtree) = frame.root_window.find(window_id)
            && kept_subtree.find(frame.selected_window).is_none()
            && let Some(first) = kept_subtree.leaf_ids().first()
        {
            frame.selected_window = *first;
        }
        frame.recalculate_minibuffer_bounds();

        for (id, parameters) in removed_windows {
            self.deleted_windows.insert(id);
            self.deleted_window_parameters.insert(id, parameters);
        }

        true
    }

    fn alloc_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        self.deleted_windows.remove(&id);
        self.deleted_window_parameters.remove(&id);
        id
    }

    /// Replace dead-buffer bindings in every live frame.
    pub fn replace_buffer_in_windows(&mut self, old_id: BufferId, new_id: BufferId) {
        for frame in self.frames.values_mut() {
            frame.replace_buffer_bindings(old_id, new_id);
        }
    }

    /// Return the frame containing a live window ID, if any.
    pub fn find_window_frame_id(&self, window_id: WindowId) -> Option<FrameId> {
        self.frames.iter().find_map(|(frame_id, frame)| {
            frame.find_window(window_id)?.is_leaf().then_some(*frame_id)
        })
    }

    /// Return true if WINDOW_ID is the minibuffer window of any live frame.
    /// (Structural replacement for the old `id >= MINIBUFFER_WINDOW_ID_BASE`
    /// magic-range check.)
    pub fn is_minibuffer_window_id(&self, window_id: WindowId) -> bool {
        self.frames
            .values()
            .any(|frame| frame.minibuffer_window == Some(window_id))
    }

    /// Return the frame containing a valid window ID, if any.
    ///
    /// Valid windows include live leaf windows, internal windows, and the
    /// minibuffer window of a live frame.
    pub fn find_valid_window_frame_id(&self, window_id: WindowId) -> Option<FrameId> {
        self.frames
            .iter()
            .find_map(|(frame_id, frame)| frame.find_window(window_id).map(|_| *frame_id))
    }

    /// Return true when WINDOW-ID designates a live window in any frame.
    pub fn is_live_window_id(&self, window_id: WindowId) -> bool {
        self.find_window_frame_id(window_id).is_some()
    }

    /// Return true when WINDOW-ID designates a valid live or internal window.
    pub fn is_valid_window_id(&self, window_id: WindowId) -> bool {
        self.find_valid_window_frame_id(window_id).is_some()
    }

    /// Return true when WINDOW-ID designates a live or stale window object.
    pub fn is_window_object_id(&self, window_id: WindowId) -> bool {
        self.is_valid_window_id(window_id) || self.deleted_windows.contains(&window_id)
    }

    /// Look up a window by id across every live frame, returning a
    /// shared reference. Mirrors GNU's `decode_window` plus tree
    /// walk.
    pub fn lookup_window(&self, window_id: WindowId) -> Option<&Window> {
        for frame in self.frames.values() {
            if let Some(w) = frame.find_window(window_id) {
                return Some(w);
            }
        }
        None
    }

    /// Look up a window by id across every live frame, returning a
    /// mutable reference.
    pub fn lookup_window_mut(&mut self, window_id: WindowId) -> Option<&mut Window> {
        for frame in self.frames.values_mut() {
            if let Some(w) = frame.find_window_mut(window_id) {
                return Some(w);
            }
        }
        None
    }

    /// Read `w->new_pixel`. Mirrors GNU
    /// `Fwindow_new_pixel` (`src/window.c`). Returns `None` if the
    /// window doesn't exist or has no pending pixel size.
    pub fn window_new_pixel(&self, window_id: WindowId) -> Option<i64> {
        self.lookup_window(window_id).and_then(Window::new_pixel)
    }

    /// Read `w->new_total`. Mirrors GNU `Fwindow_new_total`.
    pub fn window_new_total(&self, window_id: WindowId) -> Option<i64> {
        self.lookup_window(window_id).and_then(Window::new_total)
    }

    /// Read `w->new_normal`. Mirrors GNU `Fwindow_new_normal`.
    pub fn window_new_normal(&self, window_id: WindowId) -> Value {
        self.lookup_window(window_id)
            .map(Window::new_normal)
            .unwrap_or(Value::NIL)
    }

    /// Write `w->new_pixel`. When `add` is true, accumulates onto
    /// the existing slot (mirroring GNU
    /// `Fset_window_new_pixel` ADD argument).
    pub fn set_window_new_pixel(&mut self, window_id: WindowId, size: i64, add: bool) -> i64 {
        if let Some(window) = self.lookup_window_mut(window_id) {
            let stored = if add {
                window.new_pixel().unwrap_or(0) + size
            } else {
                size
            };
            window.set_new_pixel(Some(stored));
            stored
        } else {
            size
        }
    }

    /// Write `w->new_total`. ADD semantics match GNU
    /// `Fset_window_new_total`.
    pub fn set_window_new_total(&mut self, window_id: WindowId, size: i64, add: bool) -> i64 {
        if let Some(window) = self.lookup_window_mut(window_id) {
            let stored = if add {
                window.new_total().unwrap_or(0) + size
            } else {
                size
            };
            window.set_new_total(Some(stored));
            stored
        } else {
            size
        }
    }

    /// Write `w->new_normal`. Mirrors GNU `Fset_window_new_normal`.
    pub fn set_window_new_normal(&mut self, window_id: WindowId, value: Value) {
        if let Some(window) = self.lookup_window_mut(window_id) {
            window.set_new_normal(value);
        }
    }
}

impl Default for FrameManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tree manipulation helpers
// ---------------------------------------------------------------------------

/// The combination direction of `target`'s parent within `tree`.
///
/// Returns `Some(None)` when `target` *is* `tree`'s root (GNU's
/// `NILP (o->parent)`), `Some(Some(dir))` when it is a child of a combination
/// running along `dir`, and `None` when `target` is not in this tree at all.
///
/// The doubled `Option` is deliberate: "absent" and "has no parent" are
/// different answers, and collapsing them would silently turn a lookup miss
/// into a root split.
fn parent_combination_of(tree: &Window, target: WindowId) -> Option<split::ParentCombination> {
    if tree.id() == target {
        return Some(None);
    }
    let Window::Internal {
        children,
        direction,
        ..
    } = tree
    else {
        return None;
    };
    for child in children {
        if child.id() == target {
            return Some(Some(*direction));
        }
        if let Some(found) = parent_combination_of(child, target) {
            return Some(found);
        }
    }
    None
}

/// Attach a new window next to `target`, per `attachment`.
///
/// `attachment` carries GNU's already-made `combination_limit` decision (see
/// [`crate::window::split`]); this function only *executes* it, and must not
/// re-derive it from the tree.
///
/// `size` semantics (lines for vertical, columns for horizontal — 1 unit = 1.0
/// pixel in the abstract coordinate system):
/// - `None` / `Some(0)`: 50/50 split.
/// - `Some(n)` (n > 0): the new window gets `n` units.
/// - `Some(n)` (n < 0): the target window keeps `|n|` units.
#[allow(clippy::too_many_arguments)] // recursive split carries the explicit IDs and requested geometry
fn split_window_in_tree(
    tree: &mut Window,
    target: WindowId,
    direction: SplitDirection,
    internal_id: WindowId,
    new_id: WindowId,
    new_buffer_id: BufferId,
    size: Option<i64>,
    placement: SplitPlacement,
    attachment: SplitAttachment,
) -> Option<()> {
    fn split_sizes(total: f32, requested_new_size: Option<i64>) -> (f32, f32) {
        let total_px = total.round().max(0.0) as i64;
        let new_size_px = match requested_new_size {
            Some(n) if n > 0 => n.clamp(1, total_px.saturating_sub(1)),
            Some(n) if n < 0 => (total_px - (-n)).clamp(1, total_px.saturating_sub(1)),
            _ => total_px / 2,
        };
        let old_size_px = total_px - new_size_px;
        (old_size_px as f32, new_size_px as f32)
    }

    if tree.id() == target {
        // Only `NewParent` reaches here: the target being this subtree's root
        // means `parent_combination_of` reported no parent, and the child loop
        // below never recurses for a `ReuseParent` target.
        let new_parent_seal = attachment.new_parent_seal().as_stored_slot();
        let new_before_target = placement.is_before_target();
        let _old_id = tree.id();
        let old_bounds = *tree.bounds();
        let old_window = tree.clone();
        // The new internal (parent) node takes the old window's slot, so it
        // inherits the old window's character-line position (GNU
        // `split_window`); the subsequent resize pass refines both children.
        let old_top_line = old_window.top_line();
        let old_left_col = old_window.left_col();

        if let Window::Leaf {
            buffer_id: buf_id, ..
        } = old_window
        {
            let (first_bounds, second_bounds) = match direction {
                SplitDirection::Horizontal => {
                    let (old_size, new_size) = split_sizes(old_bounds.width, size);
                    let first_width = if new_before_target {
                        new_size
                    } else {
                        old_size
                    };
                    let second_width = if new_before_target {
                        old_size
                    } else {
                        new_size
                    };
                    (
                        Rect::new(old_bounds.x, old_bounds.y, first_width, old_bounds.height),
                        Rect::new(
                            old_bounds.x + first_width,
                            old_bounds.y,
                            second_width,
                            old_bounds.height,
                        ),
                    )
                }
                SplitDirection::Vertical => {
                    let (old_size, new_size) = split_sizes(old_bounds.height, size);
                    let first_height = if new_before_target {
                        new_size
                    } else {
                        old_size
                    };
                    let second_height = if new_before_target {
                        old_size
                    } else {
                        new_size
                    };
                    (
                        Rect::new(old_bounds.x, old_bounds.y, old_bounds.width, first_height),
                        Rect::new(
                            old_bounds.x,
                            old_bounds.y + first_height,
                            old_bounds.width,
                            second_height,
                        ),
                    )
                }
            };
            let (old_leaf_bounds, new_leaf_bounds) = if new_before_target {
                (second_bounds, first_bounds)
            } else {
                (first_bounds, second_bounds)
            };

            let mut old_leaf = old_window;
            old_leaf.set_bounds(old_leaf_bounds);

            let mut new_leaf = old_leaf.clone();
            if let Window::Leaf {
                id,
                buffer_id,
                bounds,
                parameters,
                history,
                window_start,
                position_markers,
                window_end,
                point,
                old_point,
                vscroll,
                preserve_vscroll_p,
                ..
            } = &mut new_leaf
            {
                let same_buffer = new_buffer_id == buf_id;
                let inherited_window_start = *window_start;
                let inherited_point = *point;
                let inherited_old_point = *old_point;
                let inherited_vscroll = *vscroll;
                let inherited_preserve_vscroll_p = *preserve_vscroll_p;
                *id = new_id;
                *buffer_id = new_buffer_id;
                *bounds = new_leaf_bounds;
                parameters.clear();
                *history = WindowHistoryState::default();
                *window_start = if same_buffer {
                    inherited_window_start
                } else {
                    LispCharPos1::ONE
                };
                *position_markers = WindowPositionMarkerState::Detached;
                *window_end = WindowEndState::Unrecorded;
                *point = if same_buffer {
                    inherited_point
                } else {
                    LispCharPos1::ONE
                };
                *old_point = if same_buffer {
                    inherited_old_point
                } else {
                    LispCharPos1::ONE
                };
                *vscroll = if same_buffer { inherited_vscroll } else { 0 };
                *preserve_vscroll_p = same_buffer && inherited_preserve_vscroll_p;
            }

            // Capture the old leaf's pre-split normal-size
            // fractions before we mutate the children. The new
            // internal node will inherit them because it occupies
            // the slot the old leaf used to fill.
            let inherited_normal_lines = old_leaf.normal_lines();
            let inherited_normal_cols = old_leaf.normal_cols();

            // Compute the new normal-size fractions for both
            // children, mirroring GNU `Fsplit_window_internal`
            // (`src/window.c:5517-5644`). Each sibling's fraction
            // in the split direction is its bounds divided by the
            // parent. The orthogonal fraction is always 1.0
            // because both children fill the parent in that
            // direction.
            let parent_size = match direction {
                SplitDirection::Horizontal => old_bounds.width,
                SplitDirection::Vertical => old_bounds.height,
            };
            let (old_fraction, new_fraction) = if parent_size > 0.0 {
                let old_frac = match direction {
                    SplitDirection::Horizontal => old_leaf_bounds.width / parent_size,
                    SplitDirection::Vertical => old_leaf_bounds.height / parent_size,
                };
                let new_frac = match direction {
                    SplitDirection::Horizontal => new_leaf_bounds.width / parent_size,
                    SplitDirection::Vertical => new_leaf_bounds.height / parent_size,
                };
                (old_frac as f64, new_frac as f64)
            } else {
                (0.5, 0.5)
            };

            match direction {
                SplitDirection::Horizontal => {
                    old_leaf.set_normal_cols(Value::make_float(old_fraction));
                    old_leaf.set_normal_lines(Value::make_float(1.0));
                    new_leaf.set_normal_cols(Value::make_float(new_fraction));
                    new_leaf.set_normal_lines(Value::make_float(1.0));
                }
                SplitDirection::Vertical => {
                    old_leaf.set_normal_lines(Value::make_float(old_fraction));
                    old_leaf.set_normal_cols(Value::make_float(1.0));
                    new_leaf.set_normal_lines(Value::make_float(new_fraction));
                    new_leaf.set_normal_cols(Value::make_float(1.0));
                }
            }

            *tree = Window::Internal {
                id: internal_id,
                direction,
                children: if new_before_target {
                    vec![new_leaf, old_leaf]
                } else {
                    vec![old_leaf, new_leaf]
                },
                bounds: old_bounds,
                parameters: Vec::new(),
                combination_limit: new_parent_seal,
                new_pixel: None,
                new_total: None,
                new_normal: Value::NIL,
                // The new internal node takes the slot that the
                // old leaf used to fill, so it inherits the
                // leaf's pre-split proportional fractions.
                normal_lines: inherited_normal_lines,
                normal_cols: inherited_normal_cols,
                top_line: old_top_line,
                left_col: old_left_col,
            };

            return Some(());
        }

        let (first_bounds, second_bounds) = match direction {
            SplitDirection::Horizontal => {
                let (old_size, new_size) = split_sizes(old_bounds.width, size);
                let first_width = if new_before_target {
                    new_size
                } else {
                    old_size
                };
                let second_width = if new_before_target {
                    old_size
                } else {
                    new_size
                };
                (
                    Rect::new(old_bounds.x, old_bounds.y, first_width, old_bounds.height),
                    Rect::new(
                        old_bounds.x + first_width,
                        old_bounds.y,
                        second_width,
                        old_bounds.height,
                    ),
                )
            }
            SplitDirection::Vertical => {
                let (old_size, new_size) = split_sizes(old_bounds.height, size);
                let first_height = if new_before_target {
                    new_size
                } else {
                    old_size
                };
                let second_height = if new_before_target {
                    old_size
                } else {
                    new_size
                };
                (
                    Rect::new(old_bounds.x, old_bounds.y, old_bounds.width, first_height),
                    Rect::new(
                        old_bounds.x,
                        old_bounds.y + first_height,
                        old_bounds.width,
                        second_height,
                    ),
                )
            }
        };
        let (old_subtree_bounds, new_leaf_bounds) = if new_before_target {
            (second_bounds, first_bounds)
        } else {
            (first_bounds, second_bounds)
        };

        let inherited_normal_lines = old_window.normal_lines();
        let inherited_normal_cols = old_window.normal_cols();

        let mut old_subtree = old_window;
        resize_window_subtree(&mut old_subtree, old_subtree_bounds);

        // `Window::new_leaf` leaves the character edges at zero for the resize
        // passes to fill in, but the split path deliberately does not resync
        // them (see `FrameManager::split_window`).  A leaf target hands them
        // down by being cloned; an internal target has to seed them explicitly,
        // or the new window reports itself at column/line 0.
        let mut new_leaf = Window::new_leaf(new_id, new_buffer_id, new_leaf_bounds);
        new_leaf.set_left_col(old_left_col);
        new_leaf.set_top_line(old_top_line);

        let parent_size = match direction {
            SplitDirection::Horizontal => old_bounds.width,
            SplitDirection::Vertical => old_bounds.height,
        };
        let (old_fraction, new_fraction) = if parent_size > 0.0 {
            let old_frac = match direction {
                SplitDirection::Horizontal => old_subtree_bounds.width / parent_size,
                SplitDirection::Vertical => old_subtree_bounds.height / parent_size,
            };
            let new_frac = match direction {
                SplitDirection::Horizontal => new_leaf_bounds.width / parent_size,
                SplitDirection::Vertical => new_leaf_bounds.height / parent_size,
            };
            (old_frac as f64, new_frac as f64)
        } else {
            (0.5, 0.5)
        };

        match direction {
            SplitDirection::Horizontal => {
                old_subtree.set_normal_cols(Value::make_float(old_fraction));
                old_subtree.set_normal_lines(Value::make_float(1.0));
                new_leaf.set_normal_cols(Value::make_float(new_fraction));
                new_leaf.set_normal_lines(Value::make_float(1.0));
            }
            SplitDirection::Vertical => {
                old_subtree.set_normal_lines(Value::make_float(old_fraction));
                old_subtree.set_normal_cols(Value::make_float(1.0));
                new_leaf.set_normal_lines(Value::make_float(new_fraction));
                new_leaf.set_normal_cols(Value::make_float(1.0));
            }
        }

        *tree = Window::Internal {
            id: internal_id,
            direction,
            children: if new_before_target {
                vec![new_leaf, old_subtree]
            } else {
                vec![old_subtree, new_leaf]
            },
            bounds: old_bounds,
            parameters: Vec::new(),
            combination_limit: new_parent_seal,
            new_pixel: None,
            new_total: None,
            new_normal: Value::NIL,
            normal_lines: inherited_normal_lines,
            normal_cols: inherited_normal_cols,
            top_line: old_top_line,
            left_col: old_left_col,
        };

        return Some(());
    }

    // Recurse into children.
    //
    // When `attachment` is `ReuseParent`, the target's own parent combination
    // absorbs the new window as a plain sibling (GNU `p = XWINDOW (o->parent)`
    // — no `make_parent_window`).  The target need NOT be a leaf: GNU splices a
    // sibling next to an internal node just the same, which is how a side
    // window is attached beside the frame's main-window group.
    //
    // For `NewParent` we fall through to the recursive descent, which re-enters
    // this function with the target as its root and interposes the parent there.
    if let Window::Internal {
        children, bounds, ..
    } = tree
    {
        let parent_bounds = *bounds;
        let child_count = children.len();
        for i in 0..child_count {
            if children[i].id() == target && attachment.reuses_parent() {
                // Reuse parent: insert new sibling into children.
                {
                    let old_bounds = *children[i].bounds();

                    let (old_size_px, new_size_px) = split_sizes(
                        match direction {
                            SplitDirection::Horizontal => old_bounds.width,
                            SplitDirection::Vertical => old_bounds.height,
                        },
                        size,
                    );

                    let new_before_target = placement.is_before_target();
                    let (first_bounds, second_bounds) = match direction {
                        SplitDirection::Horizontal => {
                            let first_w = if new_before_target {
                                new_size_px
                            } else {
                                old_size_px
                            };
                            let second_w = if new_before_target {
                                old_size_px
                            } else {
                                new_size_px
                            };
                            (
                                Rect::new(old_bounds.x, old_bounds.y, first_w, old_bounds.height),
                                Rect::new(
                                    old_bounds.x + first_w,
                                    old_bounds.y,
                                    second_w,
                                    old_bounds.height,
                                ),
                            )
                        }
                        SplitDirection::Vertical => {
                            let first_h = if new_before_target {
                                new_size_px
                            } else {
                                old_size_px
                            };
                            let second_h = if new_before_target {
                                old_size_px
                            } else {
                                new_size_px
                            };
                            (
                                Rect::new(old_bounds.x, old_bounds.y, old_bounds.width, first_h),
                                Rect::new(
                                    old_bounds.x,
                                    old_bounds.y + first_h,
                                    old_bounds.width,
                                    second_h,
                                ),
                            )
                        }
                    };

                    let (old_leaf_bounds, new_leaf_bounds) = if new_before_target {
                        (second_bounds, first_bounds)
                    } else {
                        (first_bounds, second_bounds)
                    };

                    // Resize the target child in-place.  When the target is an
                    // internal node its whole subtree has to follow, exactly as
                    // in the `NewParent` path.
                    resize_window_subtree(&mut children[i], old_leaf_bounds);

                    // Build the new sibling.  A split always introduces a live
                    // LEAF (GNU `make_window`), so an internal target seeds a
                    // fresh one rather than being cloned.
                    let mut new_leaf = if children[i].is_leaf() {
                        children[i].clone()
                    } else {
                        // See the same seeding in the `NewParent` path above:
                        // a fresh leaf's character edges start at zero and the
                        // split path does not resync them.
                        let mut leaf = Window::new_leaf(new_id, new_buffer_id, new_leaf_bounds);
                        leaf.set_left_col(children[i].left_col());
                        leaf.set_top_line(children[i].top_line());
                        leaf
                    };
                    if let Window::Leaf {
                        id,
                        buffer_id,
                        bounds,
                        parameters,
                        window_start,
                        position_markers,
                        window_end,
                        ..
                    } = &mut new_leaf
                    {
                        *id = new_id;
                        *buffer_id = new_buffer_id;
                        *bounds = new_leaf_bounds;
                        parameters.clear();
                        *window_start = LispCharPos1::ONE;
                        *position_markers = WindowPositionMarkerState::Detached;
                        *window_end = WindowEndState::Unrecorded;
                    }

                    // Compute normal fractions for all children in parent.
                    let parent_size = match direction {
                        SplitDirection::Horizontal => parent_bounds.width,
                        SplitDirection::Vertical => parent_bounds.height,
                    };
                    if parent_size > 0.0 {
                        for child_w in children.iter_mut() {
                            let frac = match direction {
                                SplitDirection::Horizontal => child_w.bounds().width / parent_size,
                                SplitDirection::Vertical => child_w.bounds().height / parent_size,
                            } as f64;
                            match direction {
                                SplitDirection::Horizontal => {
                                    child_w.set_normal_cols(Value::make_float(frac));
                                    child_w.set_normal_lines(Value::make_float(1.0));
                                }
                                SplitDirection::Vertical => {
                                    child_w.set_normal_lines(Value::make_float(frac));
                                    child_w.set_normal_cols(Value::make_float(1.0));
                                }
                            }
                        }
                        let new_frac = match direction {
                            SplitDirection::Horizontal => new_leaf_bounds.width / parent_size,
                            SplitDirection::Vertical => new_leaf_bounds.height / parent_size,
                        } as f64;
                        match direction {
                            SplitDirection::Horizontal => {
                                new_leaf.set_normal_cols(Value::make_float(new_frac));
                                new_leaf.set_normal_lines(Value::make_float(1.0));
                            }
                            SplitDirection::Vertical => {
                                new_leaf.set_normal_lines(Value::make_float(new_frac));
                                new_leaf.set_normal_cols(Value::make_float(1.0));
                            }
                        }
                    }

                    // Insert new leaf at correct position.
                    if new_before_target {
                        children.insert(i, new_leaf);
                    } else {
                        children.insert(i + 1, new_leaf);
                    }

                    return Some(());
                }
            }
        }

        // Recursive calls: iterate again for &mut access.
        for child in children.iter_mut() {
            if split_window_in_tree(
                child,
                target,
                direction,
                internal_id,
                new_id,
                new_buffer_id,
                size,
                placement,
                attachment,
            )
            .is_some()
            {
                return Some(());
            }
        }
    }

    None
}

/// Delete a window from the tree. Returns true if found and removed.
fn delete_window_in_tree(tree: &mut Window, target: WindowId, resize: DeleteResize) -> bool {
    let is_direct_child = matches!(
        tree,
        Window::Internal { children, .. } if children.iter().any(|c| c.id() == target)
    );

    if is_direct_child {
        // Unlink the target, keeping the parent's geometry and axis. Done in
        // its own scope so the borrow ends before the re-layout, which needs
        // `tree` itself.
        let (parent_bounds, horflag, remaining) = {
            let Window::Internal {
                children,
                bounds,
                direction,
                ..
            } = tree
            else {
                unreachable!("checked above")
            };
            let horflag = matches!(*direction, SplitDirection::Horizontal);
            let parent_bounds = *bounds;
            let idx = children
                .iter()
                .position(|c| c.id() == target)
                .expect("checked above");
            children.remove(idx);
            (parent_bounds, horflag, children.len())
        };

        if remaining == 1 {
            // GNU's matryoshka case: the sole surviving sibling replaces the
            // parent and inherits its geometry (`replace_window`).  The
            // promoted child keeps its own subtree, so its descendants have to
            // be re-laid-out too -- `set_bounds` alone would move the node's
            // rect while leaving its children at the old, smaller geometry,
            // visible as windows that fail to reclaim a deleted sibling's
            // space.
            let Window::Internal { children, .. } = tree else {
                unreachable!("checked above")
            };
            let mut promoted = children.pop().expect("one child remains");
            resize_window_subtree(&mut promoted, parent_bounds);
            *tree = promoted;
        } else {
            match resize {
                // GNU `Fdelete_window_internal`: `window_resize_apply (p,
                // horflag)` commits the sizes `window.el` staged in
                // `new_pixel` and re-packs the survivors from the parent's
                // edge.  The Lisp layer already chose which sibling absorbs
                // the space; this must not second-guess it.
                DeleteResize::ApplyStaged => {
                    window_resize_apply(tree, horflag, 1.0, 1.0);
                }
                // No staged sizes to honor: spread the freed space over the
                // remaining children, then push each child's new rect down
                // through its own subtree.
                DeleteResize::Redistribute => {
                    let Window::Internal { children, .. } = tree else {
                        unreachable!("checked above")
                    };
                    redistribute_bounds(children, parent_bounds);
                    for child in children.iter_mut() {
                        let child_bounds = *child.bounds();
                        resize_window_subtree(child, child_bounds);
                    }
                }
            }
        }
        return true;
    }

    // Recurse.
    if let Window::Internal { children, .. } = tree {
        for child in children {
            if delete_window_in_tree(child, target, resize) {
                return true;
            }
        }
    }

    false
}

fn collect_window_ids(window: &Window, ids: &mut HashSet<WindowId>) {
    ids.insert(window.id());
    if let Window::Internal { children, .. } = window {
        for child in children {
            collect_window_ids(child, ids);
        }
    }
}

fn collect_window_metadata(window: &Window, windows: &mut Vec<(WindowId, WindowParameters)>) {
    windows.push((window.id(), window.parameters().clone()));
    if let Window::Internal { children, .. } = window {
        for child in children {
            collect_window_metadata(child, windows);
        }
    }
}

fn find_parent_in_tree(node: &Window, target: WindowId) -> Option<WindowId> {
    let Window::Internal { children, .. } = node else {
        return None;
    };

    for child in children {
        if child.id() == target {
            return Some(node.id());
        }
        if let Some(parent) = find_parent_in_tree(child, target) {
            return Some(parent);
        }
    }

    None
}

fn find_sibling_in_tree(node: &Window, target: WindowId, next: bool) -> Option<WindowId> {
    let Window::Internal { children, .. } = node else {
        return None;
    };

    if let Some(index) = children.iter().position(|child| child.id() == target) {
        let sibling = if next {
            children.get(index + 1)
        } else {
            index.checked_sub(1).and_then(|idx| children.get(idx))
        };
        return sibling.map(Window::id);
    }

    children
        .iter()
        .find_map(|child| find_sibling_in_tree(child, target, next))
}

fn find_first_child_in_tree(
    node: &Window,
    target: WindowId,
    direction: SplitDirection,
) -> Option<WindowId> {
    match node {
        Window::Leaf { .. } => None,
        Window::Internal {
            id,
            direction: node_direction,
            children,
            ..
        } => {
            if *id == target {
                return (*node_direction == direction)
                    .then(|| children.first().map(Window::id))
                    .flatten();
            }
            children
                .iter()
                .find_map(|child| find_first_child_in_tree(child, target, direction))
        }
    }
}

/// Return the parent of WINDOW-ID inside FRAME, if any.
pub fn window_parent_id(frame: &Frame, window_id: WindowId) -> Option<WindowId> {
    if frame.minibuffer_window == Some(window_id) {
        return None;
    }
    find_parent_in_tree(&frame.root_window, window_id)
}

/// Return the first child of WINDOW-ID when it is combined in DIRECTION.
pub fn window_first_child_id(
    frame: &Frame,
    window_id: WindowId,
    direction: SplitDirection,
) -> Option<WindowId> {
    if frame.minibuffer_window == Some(window_id) {
        return None;
    }
    find_first_child_in_tree(&frame.root_window, window_id, direction)
}

/// Return the next sibling of WINDOW-ID, if any.
pub fn window_next_sibling_id(frame: &Frame, window_id: WindowId) -> Option<WindowId> {
    if frame.minibuffer_window == Some(window_id) {
        return None;
    }
    if frame.root_window.id() == window_id && frame.minibuffer_leaf.is_some() {
        return frame.minibuffer_window;
    }
    find_sibling_in_tree(&frame.root_window, window_id, true)
}

/// Return the previous sibling of WINDOW-ID, if any.
pub fn window_prev_sibling_id(frame: &Frame, window_id: WindowId) -> Option<WindowId> {
    if frame.minibuffer_window == Some(window_id) {
        return frame
            .minibuffer_leaf
            .as_ref()
            .map(|_| frame.root_window.id());
    }
    find_sibling_in_tree(&frame.root_window, window_id, false)
}

/// Apply pixel-based resize values to a window tree.
///
/// Mirrors GNU Emacs `window_resize_apply()` in window.c:
/// - Reads `new_pixel` for each window from the provided map
/// - Sets window bounds accordingly
/// - Recursively processes children, tracking edge positions
/// - For vertical combinations: accumulates vertical edge
/// - For horizontal combinations: accumulates horizontal edge
///
/// `horflag`: true = applying horizontal sizes, false = applying vertical sizes.
///
/// The pending sizes are read from each window's own `new_pixel`
/// slot (set previously via `set-window-new-pixel`), mirroring the
/// way GNU `window_resize_apply` walks `w->new_pixel` on every
/// node it visits. After audit Structural 1 in
/// `drafts/window-system-audit.md`, the slot lives on
/// `Window::Leaf` / `Window::Internal` directly so the resize
/// function no longer needs a side-table HashMap.
pub fn window_resize_apply(
    window: &mut Window,
    horflag: bool,
    _char_width: f32,
    _char_height: f32,
) {
    // Apply new_pixel to this window's bounds.
    let new_px = window.new_pixel();
    let bounds = *window.bounds();
    if let Some(px) = new_px {
        let px = px.max(0) as f32;
        if horflag {
            window.set_bounds(Rect::new(bounds.x, bounds.y, px, bounds.height));
        } else {
            window.set_bounds(Rect::new(bounds.x, bounds.y, bounds.width, px));
        }
        // Clear the pending slot to mirror GNU's
        // `wset_new_pixel(w, make_fixnum(-1))` reset at the end of
        // `window_resize_apply`.
        window.set_new_pixel(None);
    }

    // Commit the pending normal-size fraction. GNU
    // `Fwindow_resize_apply` (`src/window.c:4826,4835`):
    //
    //   if (horflag) wset_normal_cols (w, w->new_normal);
    //   else         wset_normal_lines (w, w->new_normal);
    //
    // Audit Critical 7 in `drafts/window-system-audit.md`:
    // moving the persistent fraction onto the Window struct here
    // means `window-normal-size` reads it back instead of
    // re-deriving the ratio from current pixel bounds.
    let pending_normal = window.new_normal();
    if !pending_normal.is_nil() {
        if horflag {
            window.set_normal_cols(pending_normal);
        } else {
            window.set_normal_lines(pending_normal);
        }
        window.set_new_normal(Value::NIL);
    }

    // Get updated bounds after applying new_pixel.
    let bounds = *window.bounds();
    let edge = if horflag { bounds.x } else { bounds.y };

    if let Window::Internal {
        direction,
        children,
        ..
    } = window
    {
        let mut edge = edge;
        let dir = *direction;
        for child in children.iter_mut() {
            // Position child at current edge.
            let cb = *child.bounds();
            if horflag {
                child.set_bounds(Rect::new(edge, cb.y, cb.width, cb.height));
            } else {
                child.set_bounds(Rect::new(cb.x, edge, cb.width, cb.height));
            }

            // Recurse.
            window_resize_apply(child, horflag, _char_width, _char_height);

            // Accumulate edge in the combination direction.
            let child_bounds = *child.bounds();
            match (dir, horflag) {
                (SplitDirection::Horizontal, true) => edge += child_bounds.width,
                (SplitDirection::Vertical, false) => edge += child_bounds.height,
                _ => {}
            }
        }
    }
}

/// Check that a resize is valid: the sum of children's new_pixel values
/// must equal the parent's new_pixel value in the combination direction.
///
/// Reads each window's own `new_pixel` slot, mirroring GNU's
/// recursive walk in `window_resize_check`.
pub fn window_resize_check(window: &Window, horflag: bool) -> bool {
    let my_new = window.new_pixel().unwrap_or_else(|| {
        let b = window.bounds();
        if horflag {
            b.width as i64
        } else {
            b.height as i64
        }
    });

    match window {
        Window::Leaf { .. } => true,
        Window::Internal {
            direction,
            children,
            ..
        } => {
            // In the combination direction, sum of children must equal parent.
            let combines = (*direction == SplitDirection::Horizontal) == horflag;
            if combines {
                let child_sum: i64 = children
                    .iter()
                    .map(|c| {
                        c.new_pixel().unwrap_or_else(|| {
                            let b = c.bounds();
                            if horflag {
                                b.width as i64
                            } else {
                                b.height as i64
                            }
                        })
                    })
                    .sum();
                if child_sum != my_new {
                    return false;
                }
            }
            // All children must also pass the check.
            children.iter().all(|c| window_resize_check(c, horflag))
        }
    }
}

/// Apply character-cell-based resize values to a window tree.
///
/// Mirrors GNU Emacs `window_resize_apply_total()` in window.c:
/// - Reads `new_total` for each window from the provided map
/// - Sets character-cell sizes and positions accordingly
/// - This does NOT modify pixel bounds — it only updates the character-cell
///   grid positions used by Emacs internals.
///
/// Since neomacs uses pixel bounds as the source of truth, this function
/// converts new_total back to pixels using char_width/char_height and
/// applies the result to window bounds.
///
/// The pending size for each window is read from `w->new_total`
/// (now stored on the Window enum after audit Structural 1).
pub fn window_resize_apply_total(
    window: &mut Window,
    horflag: bool,
    char_width: f32,
    char_height: f32,
) {
    let new_total = window.new_total();

    // Apply new_total converted to pixels.
    let bounds = *window.bounds();
    if let Some(total) = new_total {
        let total = total.max(0) as f32;
        if horflag {
            let px = total * char_width;
            window.set_bounds(Rect::new(bounds.x, bounds.y, px, bounds.height));
        } else {
            let px = total * char_height;
            window.set_bounds(Rect::new(bounds.x, bounds.y, bounds.width, px));
        }
        // Mirror GNU `wset_new_total(w, make_fixnum(-1))`.
        window.set_new_total(None);
    }

    let bounds = *window.bounds();
    let edge = if horflag { bounds.x } else { bounds.y };
    // GNU `window_resize_apply_total` maintains the CHARACTER-line edge in
    // parallel with the pixel edge, starting from this window's own top_line /
    // left_col (which the caller/parent already assigned).
    let char_edge_start = if horflag {
        window.left_col()
    } else {
        window.top_line()
    };

    if let Window::Internal {
        direction,
        children,
        ..
    } = window
    {
        let mut edge = edge;
        let mut char_edge = char_edge_start;
        let dir = *direction;
        for child in children.iter_mut() {
            // Position child at current pixel edge.
            let cb = *child.bounds();
            if horflag {
                child.set_bounds(Rect::new(edge, cb.y, cb.width, cb.height));
                child.set_left_col(char_edge);
            } else {
                child.set_bounds(Rect::new(cb.x, edge, cb.width, cb.height));
                child.set_top_line(char_edge);
            }

            // Recurse.
            window_resize_apply_total(child, horflag, char_width, char_height);

            // Accumulate the pixel edge and, in the same axis, the char edge
            // by the child's total lines/cols (GNU `edge += c->total_lines`).
            let child_bounds = *child.bounds();
            match (dir, horflag) {
                (SplitDirection::Horizontal, true) => {
                    edge += child_bounds.width;
                    char_edge += (child_bounds.width / char_width).round() as i64;
                }
                (SplitDirection::Vertical, false) => {
                    edge += child_bounds.height;
                    char_edge += (child_bounds.height / char_height).round() as i64;
                }
                _ => {}
            }
        }
    }
}

/// Redistribute bounds equally among children.
fn redistribute_bounds(children: &mut [Window], parent: Rect) {
    if children.is_empty() {
        return;
    }

    fn distributed_sizes(total: f32, n: usize) -> Vec<f32> {
        let total_px = total.round().max(0.0) as i64;
        let n = n as i64;
        let base = total_px / n;
        let remainder = total_px % n;
        (0..n)
            .map(|idx| (base + if idx < remainder { 1 } else { 0 }) as f32)
            .collect()
    }

    fn distributed_sizes_preserving_fixed(
        total: f32,
        children: &[Window],
        horizontal: bool,
    ) -> Vec<f32> {
        let total_px = total.round().max(0.0);
        let mut sizes = vec![0.0; children.len()];
        let mut flexible = Vec::new();
        let mut fixed_total = 0.0;
        let mut flexible_current_total = 0.0;

        for (idx, child) in children.iter().enumerate() {
            let fixed_cells = if horizontal {
                child.fixed_width_cols()
            } else {
                child.fixed_height_lines()
            };
            let current = if horizontal {
                child.bounds().width
            } else {
                child.bounds().height
            }
            .round()
            .max(0.0);
            if fixed_cells > 0 {
                sizes[idx] = current;
                fixed_total += current;
            } else {
                flexible.push(idx);
                flexible_current_total += current;
            }
        }

        if flexible.is_empty() || fixed_total >= total_px {
            return distributed_sizes(total, children.len());
        }

        let flexible_total = total_px - fixed_total;
        if flexible_current_total <= 0.0 {
            let flexible_sizes = distributed_sizes(flexible_total, flexible.len());
            for (idx, size) in flexible.into_iter().zip(flexible_sizes) {
                sizes[idx] = size;
            }
            return sizes;
        }

        let mut assigned = 0.0;
        let last_flexible = flexible.len().saturating_sub(1);
        for (flex_idx, idx) in flexible.into_iter().enumerate() {
            let current = if horizontal {
                children[idx].bounds().width
            } else {
                children[idx].bounds().height
            }
            .round()
            .max(0.0);
            let size = if flex_idx == last_flexible {
                (flexible_total - assigned).max(0.0)
            } else {
                (flexible_total * (current / flexible_current_total))
                    .round()
                    .max(0.0)
            };
            sizes[idx] = size;
            assigned += size;
        }
        sizes
    }

    // Detect direction from first two children if possible.
    if children.len() >= 2 {
        let first = children[0].bounds();
        let second = children[1].bounds();

        if (first.x - second.x).abs() > 0.1 {
            // Horizontal split
            let widths = distributed_sizes_preserving_fixed(parent.width, children, true);
            let mut edge = parent.x.round();
            for (child, width) in children.iter_mut().zip(widths) {
                child.set_bounds(Rect::new(
                    edge,
                    parent.y.round(),
                    width,
                    parent.height.round(),
                ));
                edge += width;
            }
        } else {
            // Vertical split
            let heights = distributed_sizes_preserving_fixed(parent.height, children, false);
            let mut edge = parent.y.round();
            for (child, height) in children.iter_mut().zip(heights) {
                child.set_bounds(Rect::new(
                    parent.x.round(),
                    edge,
                    parent.width.round(),
                    height,
                ));
                edge += height;
            }
        }
    } else {
        // Single child gets full bounds.
        children[0].set_bounds(parent);
    }
}

fn resize_window_subtree(window: &mut Window, bounds: Rect) {
    window.set_bounds(bounds);
    if let Window::Internal { children, .. } = window {
        redistribute_bounds(children, bounds);
        for child in children {
            let child_bounds = *child.bounds();
            resize_window_subtree(child, child_bounds);
        }
    }
}

fn sync_window_character_edges_from_bounds(window: &mut Window, char_width: f32, char_height: f32) {
    let left_col = window.left_col();
    let top_line = window.top_line();
    sync_window_character_edges_from_bounds_at(window, left_col, top_line, char_width, char_height);
}

fn sync_window_character_edges_from_bounds_at(
    window: &mut Window,
    left_col: i64,
    top_line: i64,
    char_width: f32,
    char_height: f32,
) {
    window.set_left_col(left_col);
    window.set_top_line(top_line);

    let parent_bounds = *window.bounds();
    let char_width = char_width.max(1.0);
    let char_height = char_height.max(1.0);

    if let Window::Internal { children, .. } = window {
        for child in children {
            let child_bounds = *child.bounds();
            let child_left_col =
                left_col + ((child_bounds.x - parent_bounds.x) / char_width).round() as i64;
            let child_top_line =
                top_line + ((child_bounds.y - parent_bounds.y) / char_height).round() as i64;
            sync_window_character_edges_from_bounds_at(
                child,
                child_left_col,
                child_top_line,
                char_width,
                char_height,
            );
        }
    }
}

// ===========================================================================
// GcTrace
// ===========================================================================

impl GcTrace for FrameManager {
    fn trace_roots(&self, roots: &mut Vec<Value>) {
        // Deleted window parameter maps
        for params in self.deleted_window_parameters.values() {
            for (k, v) in params {
                roots.push(*k);
                roots.push(*v);
            }
        }
        // Frame and window tree parameters
        for frame in self.frames.values() {
            roots.push(frame.name);
            roots.push(frame.icon_name);
            roots.push(frame.focus_frame);
            roots.push(frame.parent_frame);
            roots.push(frame.title);
            if let Some(window_system) = frame.window_system {
                roots.push(window_system);
            }
            roots.extend(frame.parameters.keys().copied());
            for v in frame.parameters.values() {
                roots.push(*v);
            }
            roots.push(frame.face_hash_table);
            frame.root_window.trace_roots(roots);
            if let Some(mb) = &frame.minibuffer_leaf {
                mb.trace_roots(roots);
            }
        }
    }
}

impl GcTrace for Window {
    fn trace_roots(&self, roots: &mut Vec<Value>) {
        match self {
            Window::Leaf {
                position_markers,
                display,
                ..
            } => {
                // GNU's `mark_window` traces w->start, w->pointm, and
                // w->old_pointm. The buffer marker chain is weak, so the live
                // window itself must keep the corresponding MarkerObjs alive.
                position_markers.trace_roots(roots);
                for (key, value) in self.parameters() {
                    roots.push(*key);
                    roots.push(*value);
                }
                if let Some(history) = self.history() {
                    roots.push(history.prev_buffers);
                    roots.push(history.next_buffers);
                }
                roots.push(display.display_table);
                roots.push(display.cursor_type);
                roots.push(display.vertical_scroll_bar_type);
                roots.push(display.horizontal_scroll_bar_type);
            }
            Window::Internal { children, .. } => {
                for (key, value) in self.parameters() {
                    roots.push(*key);
                    roots.push(*value);
                }
                for child in children {
                    child.trace_roots(roots);
                }
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[path = "window_test.rs"]
mod tests;
