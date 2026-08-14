//! TTY rendering backend -- reads GlyphMatrix, outputs ANSI escape sequences.
//!
//! This implements a terminal display backend matching the approach of
//! GNU Emacs's term.c. It maintains two character grids (current and desired),
//! rasterizes `FrameDisplayState` into the desired grid, then diffs against
//! current to produce minimal ANSI output.
//!
//! Runs on the evaluator thread (single-threaded, no channel needed).

use neomacs_display_protocol::face::{Face, FaceAttributes};
use neomacs_display_protocol::frame_chrome::FrameChromeContent;
use neomacs_display_protocol::frame_glyphs::CursorStyle;
use neomacs_display_protocol::glyph_matrix::*;
use neomacs_display_protocol::tty_capabilities::{
    TtyAttributeCapabilities, TtyCapability, TtyItalicRendition,
};
use neomacs_display_protocol::types::{Color, FaceId};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Cell attributes
// ---------------------------------------------------------------------------

/// Attributes for a single terminal cell (maps to ANSI SGR sequences).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CellAttrs {
    pub fg: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
    pub bold: bool,
    pub italic: bool,
    /// 0=none, 1=single, 2=curly/wave, 3=double, 4=dotted, 5=dashed
    pub underline: u8,
    pub strikethrough: bool,
    pub inverse: bool,
}

// ---------------------------------------------------------------------------
// TtyCell
// ---------------------------------------------------------------------------

/// A single cell in the terminal grid.
///
/// Normally holds one base character in `ch`. When the cell hosts a
/// grapheme cluster (base + combining marks / ZWJ sequence), the
/// extender codepoints are stored in `extenders` and emitted to the
/// terminal immediately after `ch`. Mirrors GNU's `COMPOSITE_GLYPH`:
/// the base character's cell carries the whole cluster, the combining
/// marks never occupy their own terminal cells.
#[derive(Clone, Debug, PartialEq)]
pub struct TtyCell {
    pub ch: char,
    pub attrs: CellAttrs,
    /// Whether GNU's `CHAR_GLYPH_SPACE_P` may treat this blank as implicit.
    /// A filtered TTY line-end face can have default SGR attributes while
    /// retaining non-default face identity, so attributes alone cannot decide.
    pub blank_erase: BlankErase,
    /// How this visually blank-compatible cell exists on the real terminal.
    /// GNU distinguishes a written space glyph from a cell produced by EL;
    /// terminal snapshots and later insert/delete operations observe it too.
    pub materialization: CellMaterialization,
    /// True if this is a padding cell for a wide (double-width) character.
    pub padding: bool,
    /// Grapheme-cluster extenders stacked on `ch` (None for ordinary cells).
    pub extenders: Option<Box<str>>,
}

/// Logical erase eligibility of one cell.
///
/// This is separate from [`CellMaterialization`]: `DefaultFace` says an EL
/// operation is semantically allowed, while materialization records whether
/// the real terminal cell was most recently erased or written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlankErase {
    /// A default-face space: GNU may omit it from the logical row tail.
    DefaultFace,
    /// Nonblank content or a space carrying non-default logical face identity.
    Explicit,
}

/// Physical provenance of a terminal cell.
///
/// This is deliberately an enum rather than an `explicit_space` flag: every
/// cell has exactly one physical state, and new planner operations must choose
/// one exhaustively when they update the screen model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellMaterialization {
    /// The terminal created this cell by clearing/erasing the row.
    Erased,
    /// A glyph (including an ordinary space glyph) was written here.
    Written,
}

impl Default for TtyCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            attrs: CellAttrs::default(),
            blank_erase: BlankErase::DefaultFace,
            materialization: CellMaterialization::Erased,
            padding: false,
            extenders: None,
        }
    }
}

// ---------------------------------------------------------------------------
// TtyGrid
// ---------------------------------------------------------------------------

/// Terminal character grid.
#[derive(Clone, Debug)]
pub struct TtyGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<TtyCell>,
    /// Rows written through `set`/`set_cluster` this frame. Reset by `clear`.
    /// Together with `row_carried` this proves a row identical to the screen:
    /// a carried, unwritten row was copied verbatim from the current grid and
    /// then never touched by any painter.
    row_written: Vec<bool>,
    /// Rows carried verbatim from the previous frame's grid (the
    /// `RowDamage::Reused` fast path) instead of being re-rasterized.
    row_carried: Vec<bool>,
}

impl TtyGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![TtyCell::default(); width * height];
        Self {
            width,
            height,
            cells,
            row_written: vec![false; height],
            row_carried: vec![false; height],
        }
    }

    /// Clear all cells to spaces with the given background color.
    pub fn clear(&mut self, bg: Option<(u8, u8, u8)>) {
        let blank = TtyCell {
            ch: ' ',
            attrs: CellAttrs {
                bg,
                ..CellAttrs::default()
            },
            blank_erase: BlankErase::DefaultFace,
            materialization: CellMaterialization::Erased,
            padding: false,
            extenders: None,
        };
        for cell in &mut self.cells {
            *cell = blank.clone();
        }
        self.row_written.iter_mut().for_each(|w| *w = false);
        self.row_carried.iter_mut().for_each(|c| *c = false);
    }

    /// Whether this row is PROVABLY identical to the previous frame's grid:
    /// carried verbatim from it and never subsequently written by any
    /// painter. The planner may skip such rows without comparing cells.
    pub fn row_provably_unchanged(&self, row: usize) -> bool {
        self.row_carried.get(row).copied().unwrap_or(false)
            && !self.row_written.get(row).copied().unwrap_or(true)
    }

    /// Copy `[col, col+len)` of `row` verbatim from `source` (the previous
    /// frame's grid) and mark the row carried. Does NOT mark it written —
    /// carries are invisible to the write tracking so a later real painter
    /// on the same row cancels the provably-unchanged claim, not the copy.
    /// Returns false (copying nothing) when out of bounds or shape-mismatched.
    pub fn carry_row_span_from(
        &mut self,
        source: &TtyGrid,
        row: usize,
        col: usize,
        len: usize,
    ) -> bool {
        if self.width != source.width
            || self.height != source.height
            || row >= self.height
            || col.saturating_add(len) > self.width
        {
            return false;
        }
        let start = row * self.width + col;
        self.cells[start..start + len].clone_from_slice(&source.cells[start..start + len]);
        if let Some(flag) = self.row_carried.get_mut(row) {
            *flag = true;
        }
        true
    }

    /// Set a cell at (row, col). No-op if out of bounds.
    pub fn set(&mut self, row: usize, col: usize, ch: char, attrs: CellAttrs, padding: bool) {
        let blank_erase = if ch == ' ' && !padding {
            BlankErase::DefaultFace
        } else {
            BlankErase::Explicit
        };
        self.set_with_blank_erase(row, col, ch, attrs, padding, blank_erase);
    }

    fn set_with_blank_erase(
        &mut self,
        row: usize,
        col: usize,
        ch: char,
        attrs: CellAttrs,
        padding: bool,
        blank_erase: BlankErase,
    ) {
        if row < self.height && col < self.width {
            if let Some(written) = self.row_written.get_mut(row) {
                *written = true;
            }
            if !padding {
                self.neutralize_overwrite(row, col);
            }
            let idx = row * self.width + col;
            self.cells[idx] = TtyCell {
                ch,
                attrs,
                blank_erase,
                materialization: CellMaterialization::Written,
                padding,
                extenders: None,
            };
        }
    }

    /// GNU's neutralize_wide_char (dispnew.c): overwriting one half of a
    /// wide base+padding pair must space-fill the surviving half, or the
    /// terminal blanks it while the model keeps it — a divergence no later
    /// diff can see. Called before every non-padding single-cell write
    /// (child borders, frame-rect clears, face fills, menus); a padding
    /// write needs no neutralization because its base was written
    /// immediately before it and already cleaned both sides.
    fn neutralize_overwrite(&mut self, row: usize, col: usize) {
        let row_start = row * self.width;
        let blank = |cell: &mut TtyCell| {
            cell.ch = ' ';
            cell.blank_erase = BlankErase::Explicit;
            cell.materialization = CellMaterialization::Written;
            cell.padding = false;
            cell.extenders = None;
        };
        // The old cell is a padding half: blank leftward through its base.
        if self.cells[row_start + col].padding {
            let mut base = col;
            while base > 0 && self.cells[row_start + base].padding {
                base -= 1;
            }
            for cell in &mut self.cells[row_start + base..row_start + col] {
                blank(cell);
            }
        }
        // Any padding run to the right belonged to the overwritten cell's
        // pair (a padding cell is always preceded by base-or-padding):
        // blank the orphans.
        for offset in col + 1..self.width {
            if !self.cells[row_start + offset].padding {
                break;
            }
            blank(&mut self.cells[row_start + offset]);
        }
    }

    /// Set a cluster cell at (row, col): a base character `ch` plus
    /// `extenders` (combining marks / ZWJ sequence) to be emitted in
    /// the same terminal cell. No-op if out of bounds.
    pub fn set_cluster(
        &mut self,
        row: usize,
        col: usize,
        ch: char,
        extenders: &str,
        attrs: CellAttrs,
        padding: bool,
    ) {
        if row < self.height && col < self.width {
            if let Some(written) = self.row_written.get_mut(row) {
                *written = true;
            }
            let idx = row * self.width + col;
            let ext = if extenders.is_empty() {
                None
            } else {
                Some(Box::<str>::from(extenders))
            };
            if !padding {
                self.neutralize_overwrite(row, col);
            }
            self.cells[idx] = TtyCell {
                ch,
                attrs,
                blank_erase: BlankErase::Explicit,
                materialization: CellMaterialization::Written,
                padding,
                extenders: ext,
            };
        }
    }

    /// Resize the grid, filling new cells with blanks.
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.cells.resize(width * height, TtyCell::default());
        self.row_written = vec![false; height];
        self.row_carried = vec![false; height];
    }
}

// ---------------------------------------------------------------------------
// TtyRif
// ---------------------------------------------------------------------------

/// TTY Redisplay Interface implementation.
///
/// Usage pattern:
/// 1. `rasterize(&state)` -- convert FrameDisplayState into the desired grid
/// 2. `diff_and_render()` -- diff desired vs current, emit ANSI sequences
/// 3. `take_output()` -- get the buffered bytes to write to stdout
pub struct TtyRif {
    /// What is currently displayed on the terminal.
    current: TtyGrid,
    /// What we want to display.
    desired: TtyGrid,
    /// Buffered output bytes (ANSI sequences).
    output: Vec<u8>,
    /// Cursor row to set after rendering.
    cursor_row: u16,
    /// Cursor column to set after rendering.
    cursor_col: u16,
    /// Whether the cursor should be visible.
    cursor_visible: bool,
    /// Visible terminal cursor shape when the hardware cursor is shown.
    cursor_shape: TerminalCursorShape,
    /// Face lookup table (face_id -> Face).
    faces: HashMap<FaceId, Face>,
    /// Default background color (r, g, b).
    default_bg: Option<(u8, u8, u8)>,
    /// Default foreground color (r, g, b).
    default_fg: Option<(u8, u8, u8)>,
    /// What the connected terminal can do; see [`TermCaps`].
    caps: TermCaps,
    /// Accounting for the most recent encoded frame.
    frame_stats: TtyFrameStats,
    /// Semantic scroll seed for the next diff: the layout engine's own
    /// verdict that rows shifted by this many lines (from
    /// RowDamage::ReusedShifted; TTY char metrics are 1x1, so the pixel
    /// dvpos IS the line delta). Only a SEED: detect_scroll verifies the
    /// hinted delta cell-by-cell before trusting it and falls back to hash
    /// voting, because the hint's coverage is narrower than the grid diff
    /// (selected window only, forward scroll only, invalid under
    /// overlapping child frames).
    scroll_seed: Option<isize>,
    /// Force the next render to repaint every terminal cell.
    force_full_render: bool,
}

fn terminal_cursor_cell(x: f32, y: f32, char_width: f32, char_height: f32) -> (u16, u16) {
    let char_width = char_width.max(1.0);
    let char_height = char_height.max(1.0);
    ((x / char_width) as u16, (y / char_height) as u16)
}

/// Terminal capabilities the update PLANNER is allowed to rely on.
///
/// Constructed once by the frontend from the terminal environment and handed
/// to [`TtyRif`]; the planner never proposes an operation the terminal
/// cannot execute, which keeps the encoder total (every planned op always
/// encodes). Defaults are the capabilities of every modern terminal;
/// synchronized output is additionally safe to over-claim (an unknown DEC
/// private mode is ignored), while scroll regions are not (a terminal
/// without DECSTBM would render SU as full-screen scroll), hence the split.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TermCaps {
    /// How region scrolls may be encoded, or `None` to refuse them. The
    /// method is chosen here, at capability time, and carried inside the
    /// planned op, so the encoder never consults a capability it might
    /// disagree with (the SU/SD-on-vt220 trap: `cs` attests DECSTBM but
    /// says nothing about CSI S/T).
    pub scroll_region: Option<RegionScrollMethod>,
    /// Parameterized insert/delete character (ANSI ICH/DCH): in-line
    /// horizontal shifts for the typing-echo case. True only when the
    /// terminfo entry's own insert/delete strings ARE the ANSI forms the
    /// encoder emits, not merely present (tvi955 has `ic`, but it is not
    /// `ESC[@`).
    pub insert_delete_char: bool,
    /// How GNU's row updater must materialize trailing default-face blanks.
    /// This combines termcap `in` (spaces must be written) with the exact
    /// ANSI `ce` capability the encoder supports, so the planner cannot
    /// accidentally claim both mutually exclusive paths.
    pub blank_tail: BlankTailMethod,
    /// DECSET 2026 synchronized-output bracketing.
    pub synchronized_output: bool,
}

impl Default for TermCaps {
    /// The capabilities of every modern terminal emulator; used directly by
    /// tests and by callers that already know the terminal. Real TTY startup
    /// resolves from terminfo instead, and unreadable terminfo falls back to
    /// [`TermCaps::unknown_terminal`], not to this.
    fn default() -> Self {
        Self {
            scroll_region: Some(RegionScrollMethod::SuSd),
            insert_delete_char: true,
            blank_tail: BlankTailMethod::EraseToEol {
                back_color_erase: true,
            },
            synchronized_output: true,
        }
    }
}

impl TermCaps {
    /// The safe floor for a terminal we know nothing about (unset TERM,
    /// unreadable terminfo): refuse every optimization whose bytes an
    /// arbitrary terminal may not implement, keep only synchronized output,
    /// which is spec-safe to over-claim (an unknown DEC private mode is
    /// ignored). Costs nothing but bytes: every refusal falls back to the
    /// ordinary write path.
    pub fn unknown_terminal() -> Self {
        Self {
            scroll_region: None,
            insert_delete_char: false,
            blank_tail: BlankTailMethod::WriteSpaces,
            synchronized_output: true,
        }
    }
}

/// How trailing default-face blanks are put on the terminal.
///
/// GNU derives this choice from `must_write_spaces` (`tgetflag ("in")`) and
/// clear-to-EOL support.  Keeping it closed makes the row planner handle the
/// two behaviors exhaustively instead of coordinating independent booleans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlankTailMethod {
    /// The terminal requires real space glyphs, or Neomacs cannot encode its
    /// clear-to-EOL capability.
    WriteSpaces,
    /// ANSI EL (`ESC[K`) may replace a uniform blank tail.  BCE records
    /// whether EL uses the active SGR background for colored blank tails.
    EraseToEol { back_color_erase: bool },
}

impl BlankTailMethod {
    fn can_erase(self, background: Option<(u8, u8, u8)>) -> bool {
        match self {
            Self::WriteSpaces => false,
            Self::EraseToEol { back_color_erase } => background.is_none() || back_color_erase,
        }
    }
}

/// How a region scroll is put on the wire. Chosen at capability-resolution
/// time from what the terminfo entry actually attests, and carried in the
/// planned op — the encoder is total over this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionScrollMethod {
    /// DECSTBM + cursor to the region edge + n single-line index/reverse
    /// index controls (IND `ESC D` / RI `ESC M`) — GNU term.c's
    /// `tty_ins_del_lines` fallback, the form every vt100-class entry
    /// attests via `cs` + `sr`.
    Index,
    /// DECSTBM + parameterized SU/SD (`CSI n S` / `CSI n T`), attested by
    /// terminfo `indn`/`rin`. One control regardless of distance.
    SuSd,
}

/// Vertical scroll direction and magnitude. Zero is unrepresentable: a
/// "scroll by nothing" cannot be planned, encoded, or replayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollDir {
    /// Content moves up (the viewport scrolled down through the buffer).
    Up(std::num::NonZeroU16),
    /// Content moves down.
    Down(std::num::NonZeroU16),
}

/// One planned terminal-update operation.
///
/// The planner (grid diff) decides WHAT changes; [`TtyRif::encode_ops`] is
/// the single place deciding HOW (escape selection, SGR dedup) and the
/// single place producing bytes — which also makes per-frame byte
/// accounting a fold over the op stream. Golden tests can assert on the
/// plan structurally instead of pattern-matching escape strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TermOp {
    /// Scroll grid rows `top..=bottom`: encoded as one balanced
    /// DECSTBM + scroll + reset sequence, so an unbalanced region cannot be
    /// expressed. The wire form is the capability-resolved `method`.
    ScrollRows {
        top: u16,
        bottom: u16,
        dir: ScrollDir,
        method: RegionScrollMethod,
    },
    /// Move the cursor and rewrite desired cells `start..end` of `row`.
    WriteRun { row: u16, start: u16, end: u16 },
    /// Composite-row repaint: space-clear `start..end` of `row` first, then
    /// rewrite it — the terminal's cluster-width opinion may differ from the
    /// cell grid, so the range is wiped before new glyphs land.
    ClearThenWriteRun { row: u16, start: u16, end: u16 },
    /// Erase from `from` to the physical end of `row` (ESC[K), filling with
    /// `bg` via back-color-erase. Plannable only when every desired cell of
    /// that tail is an erasable blank of that background — see
    /// `erasable_blank` for what "erasable" excludes and why.
    EraseToEol {
        row: u16,
        from: u16,
        bg: Option<(u8, u8, u8)>,
    },
    /// Shift the tail of `row` right by `count` from column `at` (ICH,
    /// ESC[n@): the typing-echo case. The planner replays the shift on the
    /// screen model and poisons the opened gap, so the ordinary span diff
    /// writes exactly the inserted cells; the terminal's pushed-off tail
    /// matches the model's dropped tail by the planner's full-suffix
    /// equality check.
    InsertCells {
        row: u16,
        at: u16,
        count: std::num::NonZeroU16,
    },
    /// Shift the tail of `row` left by `count` at column `at` (DCH,
    /// ESC[nP) — the in-line deletion case. The revealed right-edge cells
    /// are poisoned in the model and repainted by the span diff.
    DeleteCells {
        row: u16,
        at: u16,
        count: std::num::NonZeroU16,
    },
}

/// Byte/op accounting for the most recent frame, folded during encoding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TtyFrameStats {
    pub scroll_ops: u32,
    pub write_runs: u32,
    pub erase_ops: u32,
    pub cells_written: u32,
    pub bytes: u32,
    /// Layout's semantic scroll hint was verified against real cell
    /// content and used (skipping delta voting).
    pub scroll_seed_accepted: u32,
    /// The hint was present but failed cell verification (stale or
    /// conflicting); the voting fallback ran instead.
    pub scroll_seed_rejected: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalCursorShape {
    Block,
    Underline,
    Bar,
}

impl TtyRif {
    /// Create a new TtyRif for a terminal of the given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            current: TtyGrid::new(width, height),
            desired: TtyGrid::new(width, height),
            output: Vec::with_capacity(4096),
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: false,
            cursor_shape: TerminalCursorShape::Block,
            faces: HashMap::new(),
            default_bg: None,
            default_fg: None,
            caps: TermCaps::default(),
            frame_stats: TtyFrameStats::default(),
            scroll_seed: None,
            force_full_render: true,
        }
    }

    /// Create a TtyRif for a terminal with known capabilities.
    pub fn new_with_caps(width: usize, height: usize, caps: TermCaps) -> Self {
        let mut rif = Self::new(width, height);
        rif.caps = caps;
        rif
    }

    /// Declare what the connected terminal can do. The planner consults this
    /// before proposing operations; see [`TermCaps`].
    pub fn set_caps(&mut self, caps: TermCaps) {
        self.caps = caps;
    }

    /// Accounting for the most recently rendered frame.
    pub fn frame_stats(&self) -> TtyFrameStats {
        self.frame_stats
    }

    /// Resize the terminal grids. Clears both grids (forces full redraw).
    pub fn resize(&mut self, width: usize, height: usize) {
        self.current = TtyGrid::new(width, height);
        self.desired = TtyGrid::new(width, height);
        self.force_full_render = true;
    }

    /// Force the next [`diff_and_render`](Self::diff_and_render) call to emit
    /// every cell.  This matches GNU TTY menus' saved-matrix restore path:
    /// transient terminal writes outside the normal redisplay grid must be
    /// overwritten even when the logical desired grid did not change.
    pub fn force_redraw(&mut self) {
        self.force_full_render = true;
    }

    /// Set the face table for resolving face_ids.
    pub fn set_faces(&mut self, faces: HashMap<FaceId, Face>) {
        self.faces = faces;
    }

    /// Width of the terminal grid.
    pub fn width(&self) -> usize {
        self.desired.width
    }

    /// Height of the terminal grid.
    pub fn height(&self) -> usize {
        self.desired.height
    }

    fn install_state_faces(&mut self, state: &FrameDisplayState) {
        self.faces = state.faces.clone();
        let default_face = self.faces.get(&FaceId::new(0));
        self.default_bg = if default_face.is_some_and(|face| face.use_default_background) {
            None
        } else {
            Some(color_to_rgb8(&state.background))
        };
        self.default_fg = if default_face.is_some_and(|face| face.use_default_foreground) {
            None
        } else {
            default_face.map(|face| color_to_rgb8(&face.foreground))
        };
    }

    /// Rasterize a `FrameDisplayState` into the desired grid.
    ///
    /// Converts each window's `GlyphMatrix` rows into `TtyGrid` cells by
    /// iterating over glyph areas (left margin, text, right margin) and
    /// resolving face attributes.
    pub fn rasterize(&mut self, state: &FrameDisplayState) {
        self.rasterize_frame_tree(state, &[]);
    }

    /// Rasterize a root TTY frame and its visible child frames.
    ///
    /// This mirrors GNU's `combine_updates_for_frame`: the root frame is
    /// painted first, then child frame matrices are copied over it in
    /// bottom-to-top z-order.  Decorated TTY children get the same single-cell
    /// ASCII box that GNU draws around non-`undecorated` children.
    pub fn rasterize_frame_tree(
        &mut self,
        root: &FrameDisplayState,
        children_bottom_to_top: &[FrameDisplayState],
    ) {
        self.rasterize_frame_tree_states(root, children_bottom_to_top.iter());
    }

    /// Rasterize only frames that crossed the immutable presentation boundary.
    /// This is the production TTY adapter; it consumes the same sealed revision
    /// as the GUI runtime instead of accepting mutable layout state.
    pub fn rasterize_presentations(
        &mut self,
        root: &neomacs_display_protocol::SealedFramePresentation,
        children_bottom_to_top: &[neomacs_display_protocol::SealedFramePresentation],
    ) {
        self.rasterize_frame_tree_states(
            root.state(),
            children_bottom_to_top.iter().map(|child| child.state()),
        );
    }

    fn rasterize_frame_tree_states<'a>(
        &mut self,
        root: &FrameDisplayState,
        children_bottom_to_top: impl IntoIterator<Item = &'a FrameDisplayState>,
    ) {
        self.install_state_faces(root);
        self.desired.clear(self.default_bg);
        self.cursor_visible = false;
        self.cursor_shape = TerminalCursorShape::Block;

        self.rasterize_state_at(root, 0, 0, false);

        for child in children_bottom_to_top {
            if child.frame_placement.parent() != Some(root.frame_placement.frame()) {
                continue;
            }
            let outer = child.frame_placement.outer_in_parent();
            let origin_col = outer.x().round() as i64;
            let origin_row = outer.y().round() as i64;
            self.draw_child_border(child, origin_col, origin_row);
            self.rasterize_state_at(child, origin_col, origin_row, true);
        }

        if std::env::var_os("NEOMACS_DUMP_TTY_GLYPHS").is_some() {
            self.dump_tty_glyphs_to_log();
        }
    }

    fn rasterize_state_at(
        &mut self,
        state: &FrameDisplayState,
        origin_col: i64,
        origin_row: i64,
        clear_frame_rect: bool,
    ) {
        self.install_state_faces(state);

        if std::env::var_os("NEOMACS_DUMP_TTY_GLYPHS").is_some() {
            self.dump_frame_display_state_to_log(state, origin_col, origin_row);
        }

        if clear_frame_rect {
            let attrs = CellAttrs {
                bg: self.default_bg,
                ..CellAttrs::default()
            };
            let visible_rows =
                visible_cell_range(origin_row, state.frame_rows, self.desired.height);
            let visible_cols = visible_cell_range(origin_col, state.frame_cols, self.desired.width);
            for row in visible_rows {
                for col in visible_cols.clone() {
                    self.desired.set(row, col, ' ', attrs, false);
                }
            }
        }

        if let Some(cursor) = state.phys_cursor.as_ref() {
            let (cursor_col, cursor_row) =
                terminal_cursor_cell(cursor.x, cursor.y, state.char_width, state.char_height);
            let cursor_row = origin_row.saturating_add(i64::from(cursor_row));
            let cursor_col = origin_col.saturating_add(i64::from(cursor_col));
            self.cursor_visible = visible_cell(cursor_row, self.desired.height).is_some()
                && visible_cell(cursor_col, self.desired.width).is_some();
            if self.cursor_visible {
                self.cursor_row = u16::try_from(cursor_row).unwrap_or(u16::MAX);
                self.cursor_col = u16::try_from(cursor_col).unwrap_or(u16::MAX);
            }
            self.cursor_shape = match cursor.style {
                CursorStyle::FilledBox | CursorStyle::Hollow => TerminalCursorShape::Block,
                CursorStyle::Bar(_) => TerminalCursorShape::Bar,
                CursorStyle::Hbar(_) => TerminalCursorShape::Underline,
            };
        }

        for fill in &state.face_fills {
            self.rasterize_face_fill(origin_col, origin_row, state, fill);
        }

        let char_w = state.char_width.max(1.0);
        let char_h = state.char_height.max(1.0);
        for band in state.frame_chrome.bands() {
            let bounds = band.bounds().raw();
            let band_col = origin_col + (bounds.x / char_w).round() as i64;
            let band_row = origin_row + (bounds.y / char_h).round() as i64;
            match band.content() {
                FrameChromeContent::DisplayRow(content) => {
                    self.rasterize_glyph_row(
                        origin_col,
                        band_col,
                        band_row,
                        content.row(),
                        GlyphRowAreaLayout::unpartitioned(bounds, bounds),
                        char_w,
                    );
                }
                FrameChromeContent::MenuBar(content) => {
                    let cols = (band.bounds().width() / char_w).round().max(0.0) as usize;
                    let rows = (band.bounds().height() / char_h).round().max(1.0) as usize;
                    self.rasterize_frame_menu_content(content, band_col, band_row, cols, rows);
                }
                FrameChromeContent::ToolBar(_) | FrameChromeContent::CompactBar(_) => {}
            }
        }

        for entry in &state.window_matrices {
            let char_w = state.char_width.max(1.0);
            let char_h = state.char_height.max(1.0);
            for (row_idx, glyph_row) in entry.matrix.rows.iter().enumerate() {
                if let neomacs_display_protocol::glyph_matrix::RowDamage::ReusedShifted { dvpos } =
                    entry.matrix.row_damage(row_idx)
                    && char_h == 1.0
                {
                    // dvpos is the uniform pixel shift (negative = content
                    // moved up); at 1x1 TTY metrics it is the line delta.
                    let delta = -(dvpos.get().round() as isize);
                    if delta != 0 {
                        self.scroll_seed = Some(delta);
                    }
                }
                // Mirror FrameDisplayState::materialize(): buffer text rows are
                // laid out relative to the GNU TEXT_AREA, while mode-line,
                // header-line, tab-line, and minibuffer chrome remain
                // window-wide.  This is the TTY side of GNU's glyph matrix
                // margin reservation in dispnew.c: text-area glyph pointers are
                // offset past left margin columns, chrome rows are not.
                let row_bounds = entry.row_pixel_bounds(glyph_row.role);
                let area_layout = state.glyph_row_area_layout(entry, glyph_row.role);
                let row_col = origin_col + (row_bounds.x / char_w).round().max(0.0) as i64;
                let row_base = origin_row + (row_bounds.y / char_h).round().max(0.0) as i64;
                // GNU keeps two coordinate domains in each glyph row:
                // VPOS/HPOS are grid coordinates, while Y/X are pixel
                // coordinates for GUI redisplay.  TTY output is written by
                // matrix row index, so pixel_y/height_px must not stretch or
                // skip terminal rows.
                let grid_row = row_base.saturating_add(usize_to_i64_saturating(row_idx));
                // Damage-aware carry (GNU dispnew: update_window never touches
                // rows the desired matrix left disabled). A row the layout
                // engine reused VERBATIM rasterizes to exactly what the
                // previous frame rasterized, which is exactly what the current
                // grid holds — so copy the cells instead of re-resolving faces
                // and re-writing glyphs, and mark the row carried so the
                // planner can skip it without a cell compare (unless another
                // painter writes to it afterwards). Only at 1x1 TTY metrics
                // (grid row indices == matrix row indices) and never on a
                // forced full render (the current grid may be stale).
                if matches!(
                    entry.matrix.row_damage(row_idx),
                    neomacs_display_protocol::glyph_matrix::RowDamage::Reused
                ) && char_h == 1.0
                    && !self.force_full_render
                    && glyph_row.enabled
                    && grid_row >= 0
                {
                    let coverage = area_layout.structural_coverage().unwrap_or(row_bounds);
                    let carry_col = origin_col + (coverage.x / char_w).round() as i64;
                    if carry_col >= 0 {
                        let span_cols = (coverage.width / char_w).round().max(0.0) as usize;
                        if self.desired.carry_row_span_from(
                            &self.current,
                            grid_row as usize,
                            carry_col as usize,
                            span_cols,
                        ) {
                            continue;
                        }
                    }
                }
                self.rasterize_glyph_row(
                    origin_col,
                    row_col,
                    grid_row,
                    glyph_row,
                    area_layout,
                    char_w,
                );
            }
        }

        // GNU's TTY redisplay does not paint a cursor glyph into the
        // frame matrix.  It writes ordinary glyph cells, then
        // `tty_set_cursor` moves the hardware cursor and
        // `tty_update_end` shows it.  Keep cursor state separate from
        // cell attributes so blank cells retain the terminal-default
        // background.
    }

    fn draw_child_border(&mut self, child: &FrameDisplayState, origin_col: i64, origin_row: i64) {
        if child.undecorated {
            return;
        }
        self.install_state_faces(child);
        let attrs = self.resolve_attrs(FaceId::new(0));
        let width = child.frame_cols;
        let height = child.frame_rows;
        if width == 0 || height == 0 {
            return;
        }

        let width = usize_to_i64_saturating(width);
        let height = usize_to_i64_saturating(height);
        let left = origin_col.saturating_sub(1);
        let right = origin_col.saturating_add(width);
        let top = origin_row.saturating_sub(1);
        let bottom = origin_row.saturating_add(height);
        let visible_cols = visible_cell_range(origin_col, child.frame_cols, self.desired.width);
        let visible_rows = visible_cell_range(origin_row, child.frame_rows, self.desired.height);
        if visible_cols.is_empty() || visible_rows.is_empty() {
            return;
        }

        if let Some(top) = visible_cell(top, self.desired.height) {
            for col in visible_cols.clone() {
                self.desired.set(top, col, '-', attrs, false);
            }
            if let Some(left) = visible_cell(left, self.desired.width) {
                self.desired.set(top, left, '+', attrs, false);
            }
            if let Some(right) = visible_cell(right, self.desired.width) {
                self.desired.set(top, right, '+', attrs, false);
            }
        }

        if let Some(bottom) = visible_cell(bottom, self.desired.height) {
            for col in visible_cols {
                self.desired.set(bottom, col, '-', attrs, false);
            }
            if let Some(left) = visible_cell(left, self.desired.width) {
                self.desired.set(bottom, left, '+', attrs, false);
            }
            if let Some(right) = visible_cell(right, self.desired.width) {
                self.desired.set(bottom, right, '+', attrs, false);
            }
        }

        for row in visible_rows {
            if let Some(left) = visible_cell(left, self.desired.width) {
                self.desired.set(row, left, '|', attrs, false);
            }
            if let Some(right) = visible_cell(right, self.desired.width) {
                self.desired.set(row, right, '|', attrs, false);
            }
        }
    }

    /// Paint positioned menu items into the published frame-chrome band.
    ///
    /// Layout matches GNU `display_menu_bar`:
    ///
    /// * Each item label is followed by its published spacing (see GNU's
    ///   `display_string (NULL, string, Qnil, 0, 0, &it, SCHARS (string) + 1, ...)`
    ///   pattern).
    /// * Remainder of the row filled with spaces using the `menu` face,
    ///   matching GNU's `display_string ("", Qnil, ...)` tail call.
    /// * Items past the visible width are silently clipped to the band.
    fn rasterize_frame_menu_content(
        &mut self,
        menu: &neomacs_display_protocol::frame_chrome::MenuBarContent,
        origin_col: i64,
        origin_row: i64,
        frame_cols: usize,
        lines: usize,
    ) {
        let attrs = menu.terminal_style().map_or_else(
            || CellAttrs {
                fg: Some(color_to_rgb_tuple(menu.foreground())),
                bg: Some(color_to_rgb_tuple(menu.background())),
                ..CellAttrs::default()
            },
            |style| CellAttrs {
                fg: (!style.use_default_foreground).then(|| rgb_pixel_to_tuple(style.fg)),
                bg: (!style.use_default_background).then(|| rgb_pixel_to_tuple(style.bg)),
                bold: style.bold,
                italic: false,
                underline: 0,
                strikethrough: false,
                inverse: style.inverse,
            },
        );
        let visible_rows = visible_cell_range(origin_row, lines, self.desired.height);
        let visible_cols = visible_cell_range(origin_col, frame_cols, self.desired.width);
        if visible_rows.is_empty() || visible_cols.is_empty() {
            return;
        }
        for row in visible_rows {
            for col in visible_cols.clone() {
                self.desired.set(row, col, ' ', attrs, false);
            }
        }
        let mut col = 0;
        for positioned in menu.items() {
            for ch in positioned.item().label.chars() {
                if col >= frame_cols {
                    return;
                }
                if let (Some(row), Some(screen_col)) = (
                    visible_cell(origin_row, self.desired.height),
                    visible_cell(
                        origin_col.saturating_add(usize_to_i64_saturating(col)),
                        self.desired.width,
                    ),
                ) {
                    self.desired.set(row, screen_col, ch, attrs, false);
                }
                col += 1;
            }
            if col < frame_cols {
                col += 1;
            }
        }
    }

    /// Resolve face_id into terminal cell attributes.
    fn resolve_attrs(&self, face_id: FaceId) -> CellAttrs {
        if let Some(face) = self.faces.get(&face_id) {
            CellAttrs {
                fg: (!face.use_default_foreground).then(|| color_to_rgb8(&face.foreground)),
                bg: (!face.use_default_background).then(|| color_to_rgb8(&face.background)),
                bold: face.is_bold(),
                italic: face.is_italic(),
                underline: face.underline_style.gnu_code(),
                strikethrough: face.attributes.contains(FaceAttributes::STRIKE_THROUGH),
                inverse: face.attributes.contains(FaceAttributes::INVERSE),
            }
        } else {
            CellAttrs {
                fg: self.default_fg,
                bg: self.default_bg,
                ..CellAttrs::default()
            }
        }
    }

    /// Classify a resolved face for GNU's TTY trailing-space rule.
    ///
    /// Layout interns faces by stable presentation identity, so the resolved
    /// default face carried by a newline is not guaranteed to retain numeric
    /// `FaceId(0)` (the Python fixture's default newline is `FaceId(21)`). The
    /// TTY boundary canonicalizes those identities by their terminal-facing
    /// attributes before assigning the closed erase class.
    fn blank_erase_for_face(&self, face_id: FaceId) -> BlankErase {
        if self.resolve_attrs(face_id) == self.resolve_attrs(FaceId::new(0)) {
            BlankErase::DefaultFace
        } else {
            BlankErase::Explicit
        }
    }

    /// GNU's `nlen` for a desired row: the column past the last cell
    /// `write_row` still considers content.
    ///
    /// `write_row` trims trailing `CHAR_GLYPH_SPACE_P` cells only when it may
    /// leave them to `ce` (`src/dispnew.c:6019-6022`, guarded by
    /// `write_spaces_p`); everything before that column is content, blank or
    /// not.  A row whose right window margin reaches the last column has no
    /// such tail at all, so `nlen` stays the full row and its interior gap is
    /// content GNU writes.  Returning a single column keeps that one decision
    /// in one place for every caller that needs it.
    fn desired_row_content_end(&self, row: &[TtyCell], search_start: usize) -> usize {
        match uniform_erasable_tail(row, search_start) {
            Some((split, bg)) if self.caps.blank_tail.can_erase(bg) => split,
            _ => row.len(),
        }
    }

    /// Install one logical glyph cell while preserving the real terminal's
    /// erased/written state when GNU considers a default-face blank implicit.
    /// Non-default blanks are always written, even when their SGR attributes
    /// happen to equal the terminal defaults.
    fn set_desired_glyph_cell(
        &mut self,
        row: usize,
        col: usize,
        ch: char,
        attrs: CellAttrs,
        padding: bool,
        blank_erase: BlankErase,
    ) {
        let preserved_materialization =
            (blank_erase == BlankErase::DefaultFace && ch == ' ' && !padding)
                .then(|| self.current.cells.get(row * self.current.width + col))
                .flatten()
                .filter(|current| {
                    current.ch == ' '
                        && current.attrs == attrs
                        && !current.padding
                        && current.extenders.is_none()
                        && current.blank_erase == BlankErase::DefaultFace
                })
                .map(|current| current.materialization);
        self.desired
            .set_with_blank_erase(row, col, ch, attrs, padding, blank_erase);
        if let Some(materialization) = preserved_materialization {
            self.desired.cells[row * self.desired.width + col].materialization = materialization;
        }
    }

    /// Write one grapheme into the cell at `*col` (advancing it), as a base
    /// character plus combining extenders. Zero-width format joiners/selectors
    /// (ZWJ, ZWNJ, variation selectors) that the GUI shaper would consume are
    /// dropped — a terminal would otherwise show them as their own mark.
    fn write_grapheme_cell(&mut self, row: usize, col: &mut i64, text: &str, attrs: CellAttrs) {
        let mut chars = text.chars().filter(|c| !is_tty_skippable_format(*c));
        let base = chars.next().unwrap_or(' ');
        let rest: String = chars.collect();
        if let Some(col) = visible_cell(*col, self.desired.width) {
            self.desired
                .set_cluster(row, col, base, &rest, attrs, false);
        }
        *col = col.saturating_add(1);
    }

    /// Diff the desired grid against the current grid and generate ANSI escape
    /// sequences for the changed cells.
    ///
    /// After this call, `current` is swapped to reflect what is now on screen.
    /// Retrieve the buffered output with [`take_output`].
    pub fn diff_and_render(&mut self) {
        self.output.clear();
        self.frame_stats = TtyFrameStats::default();

        // Plan first (pure decision + screen-model replay), then encode (the
        // only place bytes are produced). The planner consults `self.caps`,
        // so every planned op is encodable on the connected terminal.
        let ops = self.plan_frame();

        if self.caps.synchronized_output {
            // Synchronized output (DECSET 2026): the terminal buffers
            // everything between h/l and presents it atomically. Supported
            // by kitty/ghostty/wezterm/tmux/Windows Terminal and ignored as
            // an unknown private mode elsewhere; GNU's terminal update has
            // no equivalent.
            self.output.extend_from_slice(b"\x1b[?2026h");
        }
        // Hide cursor during update to avoid flicker.
        self.output.extend_from_slice(b"\x1b[?25l");

        self.encode_ops(&ops);

        // The desired grid begins as logical GNU glyph content. Reconcile the
        // physical side effects of the chosen operations before it becomes the
        // next current-screen model: writing an erased-looking blank makes it
        // written, while EL makes even a previously written blank erased.
        self.reconcile_desired_materialization(&ops);

        // Reset attributes after all updates.
        self.output.extend_from_slice(b"\x1b[0m");

        // Position cursor and show it if visible.
        if self.cursor_visible {
            write_cursor_goto(&mut self.output, self.cursor_row + 1, self.cursor_col + 1);
            write_cursor_shape(&mut self.output, self.cursor_shape);
            self.output.extend_from_slice(b"\x1b[?25h");
        }

        if self.caps.synchronized_output {
            // End synchronized update: present the frame atomically.
            self.output.extend_from_slice(b"\x1b[?2026l");
        }

        self.frame_stats.bytes = self.output.len() as u32;

        // Swap: current now reflects what is on screen.
        std::mem::swap(&mut self.current, &mut self.desired);
        self.force_full_render = false;
    }

    /// Plan a frame without encoding, for structural tests: what would be
    /// done, as typed operations. Mutates the screen model exactly like a
    /// real render's planning stage.
    #[cfg(test)]
    fn plan_for_test(&mut self) -> Vec<TermOp> {
        self.frame_stats = TtyFrameStats::default();
        self.plan_frame()
    }

    #[cfg(test)]
    pub(crate) fn set_scroll_seed_for_test(&mut self, seed: Option<isize>) {
        self.scroll_seed = seed;
    }

    /// Decide the frame's update operations and replay their effects on the
    /// screen model (`self.current`), so the per-row diff below each
    /// decision sees the post-op screen.
    fn plan_frame(&mut self) -> Vec<TermOp> {
        let mut ops = Vec::new();

        // A zero-area grid (TIOCGWINSZ reporting 0, mid-resize race) has no
        // cells; every op references at least one, so the empty plan is the
        // only correct plan — and the indexing below assumes area > 0.
        if self.desired.width == 0 || self.desired.height == 0 {
            return ops;
        }

        self.normalize_desired_blank_tails();

        // Vertical scroll: when a run of rows merely shifted, move them with
        // the terminal (one region scroll) instead of retransmitting every
        // row - the issue-206 case where one scroll step redrew the whole
        // frame. See detect_scroll for the design.
        let seed = self.scroll_seed.take();
        if let Some(method) = self.caps.scroll_region
            && !self.force_full_render
            && let Some(scroll) = {
                let scroll = detect_scroll(&self.current, &self.desired, seed);
                // Seed disposition is observable: a stale or conflicting
                // layout hint must show up as rejected, not silently ride
                // the voting fallback.
                if let Some(delta) = seed.filter(|delta| *delta != 0) {
                    match &scroll {
                        Some(found) if found.delta == delta => {
                            self.frame_stats.scroll_seed_accepted += 1;
                        }
                        _ => self.frame_stats.scroll_seed_rejected += 1,
                    }
                }
                scroll
            }
        {
            let n = scroll.delta.unsigned_abs();
            let dir = std::num::NonZeroU16::new(n as u16).map(|n| {
                if scroll.delta > 0 {
                    ScrollDir::Up(n)
                } else {
                    ScrollDir::Down(n)
                }
            });
            if let Some(dir) = dir {
                ops.push(TermOp::ScrollRows {
                    top: scroll.top as u16,
                    bottom: scroll.bottom as u16,
                    dir,
                    method,
                });
                // Mirror the terminal-side move in the screen model, and
                // poison the exposed rows so the row diff repaints them (the
                // terminal filled them with blank cells whose exact
                // attributes we choose not to depend on).
                let w = self.current.width;
                let poison = TtyCell {
                    ch: '\0',
                    ..TtyCell::default()
                };
                if scroll.delta > 0 {
                    for i in scroll.top..=scroll.bottom - n {
                        let (dst, src) = (i * w, (i + n) * w);
                        for col in 0..w {
                            self.current.cells[dst + col] = self.current.cells[src + col].clone();
                        }
                    }
                    for i in scroll.bottom + 1 - n..=scroll.bottom {
                        self.current.cells[i * w..(i + 1) * w].fill(poison.clone());
                    }
                } else {
                    for i in (scroll.top + n..=scroll.bottom).rev() {
                        let (dst, src) = (i * w, (i - n) * w);
                        for col in 0..w {
                            self.current.cells[dst + col] = self.current.cells[src + col].clone();
                        }
                    }
                    for i in scroll.top..scroll.top + n {
                        self.current.cells[i * w..(i + 1) * w].fill(poison.clone());
                    }
                }
            }
        }

        // Rows the scroll replay mutated in the model: their current-grid
        // content no longer matches what a carry copied earlier this frame.
        let model_touched = ops
            .iter()
            .filter_map(|op| match op {
                TermOp::ScrollRows { top, bottom, .. } => Some(*top as usize..=*bottom as usize),
                _ => None,
            })
            .collect::<Vec<_>>();

        for row in 0..self.desired.height {
            // Damage-aware skip: a carried, unwritten row is byte-identical
            // to the screen model by construction — no cell compare needed.
            if !self.force_full_render
                && self.desired.row_provably_unchanged(row)
                && !model_touched.iter().any(|range| range.contains(&row))
            {
                continue;
            }
            let row_start = row * self.desired.width;
            let desired_row = &self.desired.cells[row_start..row_start + self.desired.width];
            let current_row = &self.current.cells[row_start..row_start + self.desired.width];

            let Some(first_changed) = (if self.force_full_render {
                Some(0)
            } else {
                desired_row
                    .iter()
                    .zip(current_row.iter())
                    .position(|(desired, current)| !desired.padding && desired != current)
            }) else {
                continue;
            };

            let mut last_changed = if self.force_full_render {
                desired_row.len().saturating_sub(1)
            } else {
                desired_row
                    .iter()
                    .zip(current_row.iter())
                    .rposition(|(desired, current)| !desired.padding && desired != current)
                    .expect("row with first changed cell must also have a last changed cell")
            };

            // Real terminals are not uniformly reliable when a row containing
            // grapheme clusters is rewritten with different text. If the
            // terminal's idea of the cluster width differs from our cell
            // grid, stale glyphs can remain past the internal changed span.
            // Clear and repaint the whole row tail for composite rows so
            // shrunk cluster rows cannot leave visible residue; the clear
            // also means every cell of the range must be rewritten
            // regardless of what the model recorded.
            let composite_row =
                row_has_composite_cells(desired_row) || row_has_composite_cells(current_row);
            if composite_row {
                last_changed = desired_row.len() - 1;
                ops.push(TermOp::ClearThenWriteRun {
                    row: row as u16,
                    start: first_changed as u16,
                    end: last_changed as u16 + 1,
                });
                continue;
            }
            if self.force_full_render {
                // GNU dispnew.c:5991-6013 trims trailing default-face spaces
                // when termcap `in` is absent, writes the meaningful prefix,
                // then clears the rest with `ce`.  The erased-vs-written
                // distinction is observable in a raw terminal snapshot.
                if let Some((split, bg)) = uniform_erasable_tail(desired_row, 0)
                    && self.caps.blank_tail.can_erase(bg)
                {
                    if split > 0 {
                        ops.push(TermOp::WriteRun {
                            row: row as u16,
                            start: 0,
                            end: split as u16,
                        });
                    }
                    ops.push(TermOp::EraseToEol {
                        row: row as u16,
                        from: split as u16,
                        bg,
                    });
                } else {
                    ops.push(TermOp::WriteRun {
                        row: row as u16,
                        start: 0,
                        end: desired_row.len() as u16,
                    });
                }
                continue;
            }

            // GNU dispnew.c:6062-6079 treats an enabled row whose effective
            // old length is zero uniformly: skip implicit leading blanks and
            // write everything from there to `nlen` in ONE run, then return.
            // Content provenance is irrelevant, and so is whether the interior
            // happens to match the blank row already on screen: the run is not
            // a cell-by-cell diff.  Only the trailing blanks trimmed off
            // `nlen` stay erased.
            let current_row_is_erased = current_row
                .iter()
                .all(|cell| cell.materialization == CellMaterialization::Erased);
            if current_row_is_erased {
                let write_end = self.desired_row_content_end(desired_row, first_changed);
                if first_changed < write_end {
                    ops.push(TermOp::WriteRun {
                        row: row as u16,
                        start: first_changed as u16,
                        end: write_end as u16,
                    });
                }
                continue;
            }

            // In-line horizontal shift (ICH/DCH): one char typed or deleted
            // mid-line shifts the whole tail; detecting it turns a
            // tail-rewrite into one escape plus the changed cells. The
            // shift must match to the PHYSICAL end of the row (a split
            // window's divider breaks the suffix equality and correctly
            // refuses), and any wide-char padding in the row refuses
            // outright: a wide base landing on the right edge with its
            // padding pushed off is blanked by the terminal while the model
            // keeps it — a divergence no later diff can see. The model
            // replays the shift and poisons the opened/revealed cells, so
            // the ordinary span diff below emits exactly the fresh content.
            let phys_width = self.desired.width;
            if self.caps.insert_delete_char
                && let Some((op, shifted_row)) = detect_row_shift(
                    row as u16,
                    &self.desired.cells[row_start..row_start + phys_width],
                    &self.current.cells[row_start..row_start + phys_width],
                    first_changed,
                )
            {
                ops.push(op);
                self.current.cells[row_start..row_start + phys_width]
                    .clone_from_slice(&shifted_row);
                let desired_row = &self.desired.cells[row_start..row_start + phys_width];
                let current_row = &self.current.cells[row_start..row_start + phys_width];
                let Some(fresh_first) = desired_row
                    .iter()
                    .zip(current_row.iter())
                    .position(|(desired, current)| !desired.padding && desired != current)
                else {
                    continue;
                };
                let fresh_last = desired_row
                    .iter()
                    .zip(current_row.iter())
                    .rposition(|(desired, current)| !desired.padding && desired != current)
                    .expect("shift left at least the poisoned cells changed");
                ops.push(TermOp::WriteRun {
                    row: row as u16,
                    start: fresh_first as u16,
                    end: fresh_last as u16 + 1,
                });
                continue;
            }

            // Erase-to-EOL: when the desired row's physical tail is one
            // uniform run of erasable blanks (see erasable_blank) and the
            // changed span reaches that tail, finish the logical-line update
            // with ESC[K.  GNU does this even when the abstract blank cells
            // in the tail compare equal: the erase changes physically written
            // spaces into unwritten cells, which is observable terminal state.
            // Correctness needs the WHOLE tail erasable, not just its changed
            // part. Without back-color-erase the terminal fills with its
            // default background, so a colored tail stays on the write path.
            const MIN_ERASE_CELLS: usize = 4;
            let mut erase_from: Option<usize> = None;
            {
                if let Some((split, bg)) = uniform_erasable_tail(desired_row, first_changed)
                    && desired_row.len().saturating_sub(split) >= MIN_ERASE_CELLS
                    && split <= last_changed + 1
                    && self.caps.blank_tail.can_erase(bg)
                {
                    erase_from = Some(split);
                    last_changed = split.saturating_sub(1).max(first_changed);
                    if split <= first_changed {
                        // The whole changed range is the erase.
                        ops.push(TermOp::EraseToEol {
                            row: row as u16,
                            from: split as u16,
                            bg,
                        });
                        continue;
                    }
                }
            }

            // Multi-span emission (issue 206): a row with two separate
            // change regions used to be rewritten from the first to the
            // last changed cell in one span, retransmitting the untouched
            // middle. Split the [first, last] range into changed runs and
            // coalesce runs whose gap is cheaper to retransmit than a
            // cursor motion (a goto costs ~8 bytes; an unchanged text cell
            // usually 1).
            //
            // GNU emits ONE span here: `write_glyphs (f, nbody + nsp +
            // begmatch, nlen - tem)` (`src/dispnew.c:6180-6186`), which
            // physically writes every cell of the span whether or not it
            // changed. Skipping an unchanged interior cell is invisible only
            // when the terminal already has a glyph there; a cell the
            // terminal ERASED stays unwritten, which a raw cell capture sees
            // as empty where GNU has a space. So physical materialization,
            // not just logical equality, decides what the run must cover —
            // the byte-cost rule may only skip already-written cells.
            const GOTO_COST_CELLS: usize = 8;
            let changed = |col: usize| {
                !desired_row[col].padding
                    && (desired_row[col] != current_row[col]
                        || current_row[col].materialization == CellMaterialization::Erased)
            };
            let mut col = first_changed;
            let row_op_floor = ops.len();
            while col <= last_changed {
                if changed(col) {
                    let start = col;
                    while col <= last_changed && changed(col) {
                        col += 1;
                    }
                    let coalesced = ops.len() > row_op_floor
                        && matches!(ops.last(), Some(TermOp::WriteRun { end, .. })
                            if start - *end as usize <= GOTO_COST_CELLS);
                    if coalesced {
                        if let Some(TermOp::WriteRun { end, .. }) = ops.last_mut() {
                            *end = col as u16;
                        }
                    } else {
                        ops.push(TermOp::WriteRun {
                            row: row as u16,
                            start: start as u16,
                            end: col as u16,
                        });
                    }
                } else {
                    col += 1;
                }
            }
            if let Some(from) = erase_from {
                let bg = desired_row[from].attrs.bg;
                ops.push(TermOp::EraseToEol {
                    row: row as u16,
                    from: from as u16,
                    bg,
                });
            }
        }

        ops
    }

    /// Apply GNU's logical-row trimming to the physical desired model.
    ///
    /// Rasterization deliberately creates written space glyphs for the full
    /// window-matrix slice. Only the final uniform default-face blank suffix of
    /// the complete frame row is implicit and may remain erased.
    fn normalize_desired_blank_tails(&mut self) {
        if matches!(self.caps.blank_tail, BlankTailMethod::WriteSpaces) {
            return;
        }
        for row in self.desired.cells.chunks_mut(self.desired.width) {
            let Some((split, bg)) = uniform_erasable_tail(row, 0) else {
                continue;
            };
            if !self.caps.blank_tail.can_erase(bg) {
                continue;
            }
            for cell in &mut row[split..] {
                cell.materialization = CellMaterialization::Erased;
            }
        }
    }

    /// Make the desired grid describe what the encoded operations physically
    /// leave on the terminal. The exhaustive operation match is a compile-time
    /// reminder to model the physical effect of every future operation.
    fn reconcile_desired_materialization(&mut self, ops: &[TermOp]) {
        for op in ops {
            match *op {
                TermOp::WriteRun { row, start, end }
                | TermOp::ClearThenWriteRun { row, start, end } => {
                    let row = row as usize;
                    let start = row * self.desired.width + start as usize;
                    let end = row * self.desired.width + end as usize;
                    for cell in &mut self.desired.cells[start..end] {
                        cell.materialization = CellMaterialization::Written;
                    }
                }
                TermOp::EraseToEol { row, from, .. } => {
                    let row = row as usize;
                    let start = row * self.desired.width + from as usize;
                    let end = (row + 1) * self.desired.width;
                    for cell in &mut self.desired.cells[start..end] {
                        cell.materialization = CellMaterialization::Erased;
                    }
                }
                TermOp::ScrollRows { .. }
                | TermOp::InsertCells { .. }
                | TermOp::DeleteCells { .. } => {
                    // Planning already replayed the move in `current` and
                    // verified the retained cells against desired, including
                    // their materialization. Fresh cells are covered by a
                    // following write/erase operation.
                }
            }
        }
    }

    /// Encode planned operations into escape bytes. The single place bytes
    /// are produced for grid content; owns the cross-run SGR dedup state and
    /// folds the per-frame accounting.
    fn encode_ops(&mut self, ops: &[TermOp]) {
        let mut last_attrs: Option<CellAttrs> = None;
        for op in ops {
            match *op {
                TermOp::ScrollRows {
                    top,
                    bottom,
                    dir,
                    method,
                } => {
                    self.frame_stats.scroll_ops += 1;
                    // Balanced by construction: region set, scroll, reset in
                    // one atomic encoding.
                    self.output
                        .extend_from_slice(format!("\x1b[{};{}r", top + 1, bottom + 1).as_bytes());
                    // SGR reset first: the exposed lines fill with the
                    // current background (BCE); make that the default.
                    self.output.extend_from_slice(b"\x1b[0m");
                    last_attrs = Some(CellAttrs::default());
                    match (method, dir) {
                        (RegionScrollMethod::SuSd, ScrollDir::Up(n)) => self
                            .output
                            .extend_from_slice(format!("\x1b[{n}S").as_bytes()),
                        (RegionScrollMethod::SuSd, ScrollDir::Down(n)) => self
                            .output
                            .extend_from_slice(format!("\x1b[{n}T").as_bytes()),
                        // GNU tty_ins_del_lines fallback: cursor to the
                        // region edge, then n single-line index controls.
                        // IND/RI are core VT100, attested by the
                        // DECSTBM-shaped cs that put us here.
                        (RegionScrollMethod::Index, ScrollDir::Up(n)) => {
                            write_cursor_goto(&mut self.output, bottom + 1, 1);
                            for _ in 0..n.get() {
                                self.output.extend_from_slice(b"\x1bD");
                            }
                        }
                        (RegionScrollMethod::Index, ScrollDir::Down(n)) => {
                            write_cursor_goto(&mut self.output, top + 1, 1);
                            for _ in 0..n.get() {
                                self.output.extend_from_slice(b"\x1bM");
                            }
                        }
                    }
                    self.output.extend_from_slice(b"\x1b[r");
                }
                TermOp::ClearThenWriteRun { row, start, end } => {
                    write_cursor_goto(&mut self.output, row + 1, start + 1);
                    write_sgr(&mut self.output, &CellAttrs::default());
                    last_attrs = Some(CellAttrs::default());
                    for _ in start..end {
                        self.output.push(b' ');
                    }
                    self.encode_write_run(row, start, end, &mut last_attrs);
                }
                TermOp::WriteRun { row, start, end } => {
                    self.encode_write_run(row, start, end, &mut last_attrs);
                }
                TermOp::InsertCells { row, at, count } => {
                    write_cursor_goto(&mut self.output, row + 1, at + 1);
                    self.output
                        .extend_from_slice(format!("\x1b[{count}@").as_bytes());
                }
                TermOp::DeleteCells { row, at, count } => {
                    write_cursor_goto(&mut self.output, row + 1, at + 1);
                    self.output
                        .extend_from_slice(format!("\x1b[{count}P").as_bytes());
                }
                TermOp::EraseToEol { row, from, bg } => {
                    self.frame_stats.erase_ops += 1;
                    write_cursor_goto(&mut self.output, row + 1, from + 1);
                    // Establish the BCE fill color: write_sgr resets then
                    // sets bg explicitly, so the erase paints exactly the
                    // tail's background.
                    let attrs = CellAttrs {
                        bg,
                        ..CellAttrs::default()
                    };
                    write_sgr(&mut self.output, &attrs);
                    last_attrs = Some(attrs);
                    self.output.extend_from_slice(b"\x1b[K");
                }
            }
        }
    }

    fn encode_write_run(
        &mut self,
        row: u16,
        start: u16,
        end: u16,
        last_attrs: &mut Option<CellAttrs>,
    ) {
        self.frame_stats.write_runs += 1;
        write_cursor_goto(&mut self.output, row + 1, start + 1);
        let row_start = row as usize * self.desired.width;
        for col in start as usize..end as usize {
            let desired = &self.desired.cells[row_start + col];
            if desired.padding {
                continue;
            }
            if last_attrs.as_ref() != Some(&desired.attrs) {
                write_sgr(&mut self.output, &desired.attrs);
                *last_attrs = Some(desired.attrs);
            }
            let cell = desired.clone();
            write_cell_contents(&mut self.output, &cell);
            self.frame_stats.cells_written += 1;
        }
    }

    /// Take the buffered output bytes. The caller writes these to stdout.
    ///
    /// After calling this, the internal buffer is empty.
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.output)
    }

    fn rasterize_face_fill(
        &mut self,
        origin_col: i64,
        origin_row: i64,
        state: &FrameDisplayState,
        fill: &FaceFillItem,
    ) {
        let char_w = state.char_width.max(1.0);
        let char_h = state.char_height.max(1.0);
        let start_col = origin_col + (fill.bounds.x / char_w).round().max(0.0) as i64;
        let start_row = origin_row + (fill.bounds.y / char_h).round().max(0.0) as i64;
        let width_cols = (fill.bounds.width / char_w).ceil().max(0.0) as usize;
        let height_rows = (fill.bounds.height / char_h).ceil().max(0.0) as usize;
        if width_cols == 0 || height_rows == 0 {
            return;
        }

        let attrs = self.resolve_attrs(fill.face_id);
        let visible_rows = visible_cell_range(start_row, height_rows, self.desired.height);
        let visible_cols = visible_cell_range(start_col, width_cols, self.desired.width);
        for row in visible_rows {
            for col in visible_cols.clone() {
                self.desired.set(row, col, ' ', attrs, false);
            }
        }
    }

    fn rasterize_glyph_row(
        &mut self,
        frame_origin_col: i64,
        screen_col_start: i64,
        screen_row: i64,
        glyph_row: &GlyphRow,
        area_layout: GlyphRowAreaLayout,
        char_width: f32,
    ) {
        let Some(screen_row) = visible_cell(screen_row, self.desired.height) else {
            return;
        };
        if !glyph_row.enabled {
            return;
        }

        // A row's horizontal start and source slot are one authoritative
        // placement.  TTY cells have no sub-cell positioning, so project the
        // pixel offset to its already-resolved display column.
        let mut col = screen_col_start.saturating_add(i64::from(glyph_row.start_col));
        let screen_width = usize_to_i64_saturating(self.desired.width);

        for area in GlyphArea::ALL {
            if let GlyphAreaPlacement::Structural(geometry) = area_layout.placement(area) {
                col = frame_origin_col
                    .saturating_add((geometry.bounds().x / char_width).round() as i64);
                if area == GlyphArea::Text {
                    col = col.saturating_add(i64::from(glyph_row.start_col));
                }
            }
            let glyphs = &glyph_row.glyphs[area.index()];
            let mut glyph_idx = 0;
            let mut preceding_wide_base_visible = None;
            while glyph_idx < glyphs.len() {
                let glyph = &glyphs[glyph_idx];
                if col >= screen_width {
                    break;
                }

                if glyph.padding {
                    let attrs = self.resolve_attrs(glyph.face_id);
                    if let Some(col) = visible_cell(col, self.desired.width) {
                        self.set_desired_glyph_cell(
                            screen_row,
                            col,
                            ' ',
                            attrs,
                            preceding_wide_base_visible.take().unwrap_or(true),
                            BlankErase::Explicit,
                        );
                    }
                    col = col.saturating_add(1);
                    glyph_idx += 1;
                    continue;
                }

                let attrs = self.resolve_attrs(glyph.face_id);
                let base_visible = visible_cell(col, self.desired.width).is_some();
                // Composite glyphs (base char + grapheme-cluster
                // extenders) occupy one cell whose content is the full
                // cluster string, mirroring GNU's COMPOSITE_GLYPH.
                match &glyph.glyph_type {
                    GlyphType::Composite { text } => {
                        // A contextual-shaping run (Arabic, Indic) is the base
                        // Composite followed by one per-letter grapheme padding
                        // cell per following letter. The GUI shapes the whole
                        // run from the base Composite, but a terminal cannot —
                        // so lay the run out one grapheme per column, visually
                        // reversed for right-to-left, mirroring GNU's term.c.
                        // A plain grapheme cluster (emoji, base+combining) has no
                        // such grapheme paddings and stays a single cell.
                        let run_paddings: Vec<String> = glyphs[glyph_idx + 1..]
                            .iter()
                            .take_while(|g| is_run_member_padding_cell(g))
                            .map(cell_grapheme_string)
                            .collect();
                        if run_paddings.is_empty() {
                            self.write_grapheme_cell(screen_row, &mut col, text, attrs);
                        } else {
                            // Paddings hold the run's letters after the base, in
                            // logical order; the base cell's own grapheme is the
                            // run text with that suffix removed.
                            let tail: String = run_paddings.concat();
                            let g0 = text.strip_suffix(tail.as_str()).unwrap_or(text);
                            let mut graphemes: Vec<&str> =
                                Vec::with_capacity(run_paddings.len() + 1);
                            graphemes.push(g0);
                            graphemes.extend(run_paddings.iter().map(String::as_str));
                            if glyph.bidi_level & 1 == 1 {
                                graphemes.reverse();
                            }
                            let consumed = graphemes.len() - 1;
                            for grapheme in graphemes {
                                if col >= screen_width {
                                    break;
                                }
                                self.write_grapheme_cell(screen_row, &mut col, grapheme, attrs);
                            }
                            glyph_idx += consumed;
                        }
                    }
                    GlyphType::Stretch { width_cols } => {
                        let width_cols = usize::from((*width_cols).max(1));
                        for _ in 0..width_cols {
                            if col >= screen_width {
                                break;
                            }
                            if let Some(col) = visible_cell(col, self.desired.width) {
                                self.set_desired_glyph_cell(
                                    screen_row,
                                    col,
                                    ' ',
                                    attrs,
                                    false,
                                    self.blank_erase_for_face(glyph.face_id),
                                );
                            }
                            col = col.saturating_add(1);
                        }
                    }
                    GlyphType::Surface { width_cols, .. } => {
                        // A shader surface is GPU-only; a terminal cannot draw
                        // it. Fill its reserved columns with a visible labeled
                        // placeholder instead of blank space (surfaces are a
                        // neomacs extension, so there is no GNU TTY behavior to
                        // match). This also occupies the full width_cols, which
                        // the single-char fallthrough arm would not.
                        let width_cols = usize::from((*width_cols).max(1));
                        for ch in surface_tty_placeholder(width_cols).chars() {
                            if col >= screen_width {
                                break;
                            }
                            if let Some(col) = visible_cell(col, self.desired.width) {
                                self.set_desired_glyph_cell(
                                    screen_row,
                                    col,
                                    ch,
                                    attrs,
                                    false,
                                    BlankErase::Explicit,
                                );
                            }
                            col = col.saturating_add(1);
                        }
                    }
                    _ => {
                        // A wide base in the final screen column has no room
                        // for its padding half: the terminal blanks it while
                        // the model would keep it. GNU never emits a
                        // partially visible multi-column glyph; write the
                        // space it would show instead.
                        let ch = if glyph.wide && col + 1 >= screen_width {
                            ' '
                        } else {
                            glyph_to_char(glyph)
                        };
                        if let Some(col) = visible_cell(col, self.desired.width) {
                            self.set_desired_glyph_cell(
                                screen_row,
                                col,
                                ch,
                                attrs,
                                false,
                                if ch == ' ' {
                                    self.blank_erase_for_face(glyph.face_id)
                                } else {
                                    BlankErase::Explicit
                                },
                            );
                        }
                        col = col.saturating_add(1);

                        let next_is_explicit_padding = glyph.wide
                            && glyphs
                                .get(glyph_idx + 1)
                                .is_some_and(|next_glyph| next_glyph.padding);
                        if glyph.wide && !next_is_explicit_padding && col < screen_width {
                            if let Some(col) = visible_cell(col, self.desired.width) {
                                self.set_desired_glyph_cell(
                                    screen_row,
                                    col,
                                    ' ',
                                    attrs,
                                    base_visible,
                                    BlankErase::Explicit,
                                );
                            }
                            col = col.saturating_add(1);
                        }
                    }
                }
                preceding_wide_base_visible = glyph.wide.then_some(base_visible);
                glyph_idx += 1;
            }

            if area == GlyphArea::Text {
                let right_edge = if glyph_row.glyphs[GlyphArea::RightMargin.index()].is_empty() {
                    match area_layout.placement(GlyphArea::Text) {
                        GlyphAreaPlacement::Structural(geometry) => {
                            frame_origin_col.saturating_add(
                                (geometry.bounds().right() / char_width).round() as i64,
                            )
                        }
                        GlyphAreaPlacement::FollowingPreviousArea => screen_width,
                    }
                } else {
                    match area_layout.placement(GlyphArea::RightMargin) {
                        GlyphAreaPlacement::Structural(geometry) => frame_origin_col
                            .saturating_add((geometry.bounds().x / char_width).round() as i64),
                        GlyphAreaPlacement::FollowingPreviousArea => frame_origin_col
                            .saturating_add(
                                (area_layout
                                    .structural_coverage()
                                    .map(|coverage| coverage.right())
                                    .unwrap_or(self.desired.width as f32)
                                    / char_width)
                                    .round() as i64,
                            ),
                    }
                };
                if col < right_edge {
                    // GNU's terminal branch of `extend_face_to_end_of_line`
                    // materializes the whole remaining text area. Without an
                    // explicit `:extend` face, its attr-filtered face LOOKS
                    // default. It is nevertheless a non-default logical face
                    // when the newline/source face was non-default, and thus
                    // fails CHAR_GLYPH_SPACE_P (the font-lock comment case).
                    let blank_erase = if glyph_row.ends_at_zv {
                        BlankErase::DefaultFace
                    } else {
                        glyphs
                            .iter()
                            .rfind(|glyph| !glyph.padding)
                            .map(|glyph| self.blank_erase_for_face(glyph.face_id))
                            .unwrap_or(BlankErase::DefaultFace)
                    };
                    while col < right_edge && col < screen_width {
                        if let Some(col) = visible_cell(col, self.desired.width) {
                            // A published FaceFillItem owns the resolved
                            // background behind this matrix slot. GNU's
                            // default-like line filler materializes that slot;
                            // it does not erase the fill's background.
                            let attrs =
                                self.desired.cells[screen_row * self.desired.width + col].attrs;
                            self.set_desired_glyph_cell(
                                screen_row,
                                col,
                                ' ',
                                attrs,
                                false,
                                blank_erase,
                            );
                        }
                        col = col.saturating_add(1);
                    }
                }
            }
        }

        // On TTY frames GNU has one terminal cursor, positioned after
        // glyph output by `tty_set_cursor`; row cursor markers do not
        // become painted cell attributes.
    }
}

fn usize_to_i64_saturating(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn visible_cell(value: i64, limit: usize) -> Option<usize> {
    usize::try_from(value).ok().filter(|value| *value < limit)
}

fn visible_cell_range(start: i64, extent: usize, limit: usize) -> std::ops::Range<usize> {
    let limit = usize_to_i64_saturating(limit);
    let end = start.saturating_add(usize_to_i64_saturating(extent));
    let visible_start = start.clamp(0, limit);
    let visible_end = end.clamp(visible_start, limit);
    visible_start as usize..visible_end as usize
}

/// Hash of one grid row, used only to ACCELERATE scroll matching; equality
/// of the actual cells is always verified before a match is trusted, so a
/// collision can cost time but never correctness.
fn row_hash(row: &[TtyCell]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = rustc_hash::FxHasher::default();
    for c in row {
        h.write_u32(c.ch as u32);
        c.attrs.fg.hash(&mut h);
        c.attrs.bg.hash(&mut h);
        h.write_u8(
            (c.attrs.bold as u8)
                | ((c.attrs.italic as u8) << 1)
                | ((c.attrs.strikethrough as u8) << 2)
                | ((c.attrs.inverse as u8) << 3)
                | ((c.padding as u8) << 4),
        );
        h.write_u8(match c.blank_erase {
            BlankErase::DefaultFace => 0,
            BlankErase::Explicit => 1,
        });
        h.write_u8(match c.materialization {
            CellMaterialization::Erased => 0,
            CellMaterialization::Written => 1,
        });
        h.write_u8(c.attrs.underline);
        if let Some(e) = &c.extenders {
            h.write(e.as_bytes());
        }
    }
    h.finish()
}

/// A vertical scroll detected between the current and desired grids:
/// desired row `i` equals current row `i + delta` for every `i` in
/// `rows.start .. rows.end - delta.max(0)` (and symmetrically for negative
/// delta). Emitting a terminal region scroll makes those rows identical
/// without retransmitting them.
struct DetectedScroll {
    top: usize,
    bottom: usize, // inclusive
    delta: isize,  // >0: content moves up (scroll down through the buffer)
}

/// Find the single dominant vertical shift between the grids, if any.
///
/// GNU infers scrolls with an O(rows^2) dynamic program over
/// baud-rate-based insert/delete-line cost matrices (scroll.c
/// calculate_scrolling), a design for terminals where IL/DL had per-line
/// padding costs. Modern terminals all support region scrolls (DECSTBM +
/// SU/SD), so the decision collapses to "is there a shift with a long
/// matching run": vote for candidate deltas by row-hash equality, verify
/// the best run cell-by-cell, done in O(rows) hashes + one run of row
/// comparisons. (Neovim receives scroll deltas semantically from its core;
/// a layout-provided hint can replace the inference here the same way
/// later.)
fn detect_scroll(
    current: &TtyGrid,
    desired: &TtyGrid,
    seed: Option<isize>,
) -> Option<DetectedScroll> {
    const MIN_RUN: usize = 4;
    let (w, h) = (desired.width, desired.height);
    if w != current.width || h != current.height || h < MIN_RUN + 1 {
        return None;
    }
    // A carried, unwritten desired row is byte-identical to the current row
    // at the same index by construction: give BOTH sides a per-row sentinel
    // instead of hashing 2x row cells. Default blank rows get the same
    // stationary treatment. Moving them saves no output (EL already clears a
    // whole row), and counting a large blank band as a shifted content run
    // makes us scroll where GNU's cost model chooses a repaint. Row-unique
    // sentinels keep both kinds stationary (old == new at r) while never
    // matching across rows.
    const CARRIED_SENTINEL: u64 = 1 << 63;
    const DEFAULT_BLANK_SENTINEL: u64 = 1 << 62;
    let carried_sentinel = |r: usize| desired.row_provably_unchanged(r).then_some(r as u64);
    let default_blank_sentinel = |grid: &TtyGrid, r: usize| {
        let row = &grid.cells[r * w..(r + 1) * w];
        row.iter()
            .all(|cell| cell == &TtyCell::default())
            .then_some(DEFAULT_BLANK_SENTINEL | r as u64)
    };
    let old_hash: Vec<u64> = (0..h)
        .map(|r| {
            carried_sentinel(r)
                .map(|row| CARRIED_SENTINEL | row)
                .or_else(|| default_blank_sentinel(current, r))
                .unwrap_or_else(|| row_hash(&current.cells[r * w..(r + 1) * w]))
        })
        .collect();
    let new_hash: Vec<u64> = (0..h)
        .map(|r| {
            carried_sentinel(r)
                .map(|row| CARRIED_SENTINEL | row)
                .or_else(|| default_blank_sentinel(desired, r))
                .unwrap_or_else(|| row_hash(&desired.cells[r * w..(r + 1) * w]))
        })
        .collect();

    // Changed band: rows outside it already match in place.
    let top = (0..h).find(|&r| old_hash[r] != new_hash[r])?;
    let bottom = (0..h).rfind(|&r| old_hash[r] != new_hash[r])?;

    // The layout engine's own scroll verdict, when present, names the delta
    // outright: verify it with the same run machinery and skip the voting.
    // A wrong or stale seed simply fails verification and costs nothing.
    if let Some(delta) = seed
        && delta != 0
        && let Some(found) =
            verify_delta(current, desired, &old_hash, &new_hash, top, bottom, delta)
    {
        return Some(found);
    }

    // Vote for deltas using positions of equal hashes inside the band.
    let mut by_hash: rustc_hash::FxHashMap<u64, Vec<usize>> = rustc_hash::FxHashMap::default();
    for (r, &hash) in old_hash.iter().enumerate().take(bottom + 1).skip(top) {
        by_hash.entry(hash).or_default().push(r);
    }
    let mut votes: rustc_hash::FxHashMap<isize, usize> = rustc_hash::FxHashMap::default();
    for (i, hash) in new_hash.iter().enumerate().take(bottom + 1).skip(top) {
        if let Some(js) = by_hash.get(hash) {
            for &j in js {
                if i != j {
                    *votes.entry(j as isize - i as isize).or_default() += 1;
                }
            }
        }
    }
    let (&delta, &n) = votes.iter().max_by_key(|entry| *entry.1)?;
    if n < MIN_RUN || delta == 0 {
        return None;
    }
    verify_delta(current, desired, &old_hash, &new_hash, top, bottom, delta)
}

/// Verify a candidate scroll delta: find the longest contiguous run where
/// desired row `i` equals current row `i + delta` with REAL cell equality
/// (hashes only route; a collision can cost time, never correctness), and
/// return the covering region when the run is long enough to pay for a
/// region scroll. Composite rows are excluded: the conservative full-tail
/// repaint path owns them.
fn verify_delta(
    current: &TtyGrid,
    desired: &TtyGrid,
    old_hash: &[u64],
    new_hash: &[u64],
    top: usize,
    bottom: usize,
    delta: isize,
) -> Option<DetectedScroll> {
    const MIN_RUN: usize = 4;
    let (w, h) = (desired.width, desired.height);
    let row_eq = |i: usize| -> bool {
        let j = i as isize + delta;
        if j < 0 || j as usize >= h {
            return false;
        }
        let j = j as usize;
        if new_hash[i] != old_hash[j] {
            return false;
        }
        let d = &desired.cells[i * w..(i + 1) * w];
        let c = &current.cells[j * w..(j + 1) * w];
        d == c && !row_has_composite_cells(d)
    };
    let (mut best_lo, mut best_len) = (0usize, 0usize);
    let mut run_lo: Option<usize> = None;
    for i in top..=bottom + 1 {
        if i <= bottom && row_eq(i) {
            run_lo.get_or_insert(i);
        } else if let Some(lo) = run_lo.take()
            && i - lo > best_len
        {
            best_lo = lo;
            best_len = i - lo;
        }
    }
    if best_len < MIN_RUN {
        return None;
    }
    // The region covers the matched run plus the rows the scroll exposes.
    let (top, bottom) = if delta > 0 {
        (best_lo, best_lo + best_len - 1 + delta as usize)
    } else {
        (
            best_lo.checked_sub((-delta) as usize)?,
            best_lo + best_len - 1,
        )
    };
    if bottom >= h {
        return None;
    }
    Some(DetectedScroll { top, bottom, delta })
}

/// Detect an in-line horizontal shift of the row tail (one insertion or
/// deletion of up to MAX_SHIFT cells at `first_changed`), returning the op
/// and the post-shift model row (shifted content with the fresh cells
/// poisoned). Refusal rules — each one load-bearing:
/// - any padding cell in either row: a wide base pushed against or off the
///   right edge is blanked by the terminal but kept by the model, and no
///   later diff can see the difference (both grids agree);
/// - any composite cell: cluster-width bookkeeping differs per terminal
///   (the same exclusion every other optimization applies);
/// - the suffix equality runs to the PHYSICAL row end, never a sub-span, so
///   vertically split windows (divider glyphs) refuse naturally.
fn detect_row_shift(
    row: u16,
    desired_row: &[TtyCell],
    current_row: &[TtyCell],
    first_changed: usize,
) -> Option<(TermOp, Vec<TtyCell>)> {
    const MAX_SHIFT: usize = 8;
    const MIN_SAVED_CELLS: usize = 8;
    let width = desired_row.len();
    if desired_row
        .iter()
        .any(|c| c.padding || c.extenders.is_some())
        || current_row
            .iter()
            .any(|c| c.padding || c.extenders.is_some())
    {
        return None;
    }
    let poison = TtyCell {
        ch: '\0',
        ..TtyCell::default()
    };
    for d in 1..=MAX_SHIFT.min(width.saturating_sub(first_changed + 1)) {
        // Insertion: desired[first+d..] == current[first..width-d].
        let suffix = width - first_changed - d;
        if suffix >= MIN_SAVED_CELLS
            && desired_row[first_changed + d..] == current_row[first_changed..width - d]
            && carries_shiftable_content(&current_row[first_changed..width - d])
        {
            let mut shifted = current_row.to_vec();
            // TtyCell is not Copy (cluster extenders); rotate moves.
            shifted[first_changed..].rotate_right(d);
            shifted[first_changed..first_changed + d].fill(poison.clone());
            let count = std::num::NonZeroU16::new(d as u16)?;
            return Some((
                TermOp::InsertCells {
                    row,
                    at: first_changed as u16,
                    count,
                },
                shifted,
            ));
        }
        // Deletion: desired[first..width-d] == current[first+d..].
        if suffix >= MIN_SAVED_CELLS
            && desired_row[first_changed..width - d] == current_row[first_changed + d..]
            && carries_shiftable_content(&current_row[first_changed + d..])
        {
            let mut shifted = current_row.to_vec();
            shifted[first_changed..].rotate_left(d);
            shifted[width - d..].fill(poison.clone());
            let count = std::num::NonZeroU16::new(d as u16)?;
            return Some((
                TermOp::DeleteCells {
                    row,
                    at: first_changed as u16,
                    count,
                },
                shifted,
            ));
        }
    }
    None
}

/// A desired cell that erase-to-EOL may paint instead of writing.
///
/// The erased cell a terminal produces is "space in the current background":
/// Exact terminal parity preserves more than visible pixels: EL creates an
/// unwritten cell in the active background, so it cannot replace a space whose
/// foreground or attributes were explicitly written even when those settings
/// are visually inert on a blank. The background must match the erase's BCE
/// fill, which the caller guarantees by grouping the tail by one uniform bg.
fn erasable_blank(cell: &TtyCell) -> bool {
    cell.ch == ' '
        && cell.blank_erase == BlankErase::DefaultFace
        && !cell.padding
        && cell.extenders.is_none()
        && cell.attrs.fg.is_none()
        && !cell.attrs.bold
        && !cell.attrs.italic
        && cell.attrs.underline == 0
        && !cell.attrs.strikethrough
        && !cell.attrs.inverse
}

/// Whether a run a horizontal shift would preserve is worth preserving.
///
/// GNU's `update_frame_line` (dispnew.c) strips trailing spaces from both the
/// old and the new row before it computes `begmatch`/`endmatch`, so blanks past
/// a row's logical end never count toward what an insert/delete-char saves. A
/// run of default blanks trivially matches any other run of default blanks, so
/// without this the detector "saves" a suffix carrying nothing: a row going
/// wholly blank matches as a left shift of exactly its old content's width.
/// That costs more bytes than the erase GNU emits and leaves a physically
/// different terminal — DCH shifts written blanks in, EL leaves cells
/// unwritten. A space that carries a background is real content and still
/// shifts, matching GNU's `colored_spaces_p`.
fn carries_shiftable_content(run: &[TtyCell]) -> bool {
    run.iter()
        .any(|cell| !erasable_blank(cell) || cell.attrs.bg.is_some())
}

/// Return the start and background of the physical row's uniform erasable
/// blank tail, without scanning before `search_start`.
fn uniform_erasable_tail(
    row: &[TtyCell],
    search_start: usize,
) -> Option<(usize, Option<(u8, u8, u8)>)> {
    let mut cells = row.get(search_start..)?.iter().enumerate().rev();
    let (last_offset, last) = cells.next()?;
    if !erasable_blank(last) {
        return None;
    }

    let background = last.attrs.bg;
    let mut split = search_start + last_offset;
    for (offset, cell) in cells {
        if !erasable_blank(cell) || cell.attrs.bg != background {
            break;
        }
        split = search_start + offset;
    }
    Some((split, background))
}

fn row_has_composite_cells(row: &[TtyCell]) -> bool {
    row.iter().any(|cell| cell.extenders.is_some())
}

// ---------------------------------------------------------------------------
// ANSI helper functions
// ---------------------------------------------------------------------------

/// Convert a display-protocol `Color` (linear f32 0.0-1.0) to an 8-bit
/// sRGB tuple suitable for a 24-bit ANSI color escape sequence.
///
/// `Color` values in the display protocol are stored in **linear
/// space** because the wgpu GPU surface (`Bgra8UnormSrgb`)
/// expects linear input and applies the linear-to-sRGB
/// conversion automatically at the framebuffer. The TTY output
/// path has no such automatic conversion — terminals interpret
/// the 8-bit values as **sRGB** — so we must apply
/// `linear_to_srgb` here to undo the `srgb_to_linear` that
/// `Color::from_pixel` applied when the Emacs pixel value was
/// loaded.
///
/// Without this conversion every face color is darker than
/// GNU's by an exact gamma-2.4 amount:
///
///   mode-line bg:  GNU=grey75 (191) neomacs=grey52 (132)
///   vertical-border fg: GNU=grey20 (51) neomacs=8
///
/// With the conversion the emitted bytes match GNU's sRGB pixel
/// values exactly, since `linear_to_srgb(srgb_to_linear(x)) ≈ x`
/// (modulo f32 rounding).
///
/// Mirrors GNU `src/term.c::tty_defined_color` which stores and
/// emits face colors as sRGB pixel values with no conversion.
fn color_to_rgb8(c: &Color) -> (u8, u8, u8) {
    (
        Color::linear_component_to_srgb_u8(c.r),
        Color::linear_component_to_srgb_u8(c.g),
        Color::linear_component_to_srgb_u8(c.b),
    )
}

/// Decompose a 24-bit sRGB pixel (`0x00RRGGBB`) into its byte channels.
/// Used for the TTY menu bar where colours arrive as packed pixels from
/// the layout-engine `FaceResolver` rather than as float `Color`s.
fn rgb_pixel_to_tuple(pixel: u32) -> (u8, u8, u8) {
    (
        ((pixel >> 16) & 0xFF) as u8,
        ((pixel >> 8) & 0xFF) as u8,
        (pixel & 0xFF) as u8,
    )
}

fn color_to_rgb_tuple(color: Color) -> (u8, u8, u8) {
    (
        (color.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.b.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

/// Write an ANSI CUP (cursor position) escape sequence.
/// Row and col are 1-based.
fn write_cursor_goto(buf: &mut Vec<u8>, row: u16, col: u16) {
    use std::io::Write;
    let _ = write!(buf, "\x1b[{};{}H", row, col);
}

fn write_cursor_shape(buf: &mut Vec<u8>, shape: TerminalCursorShape) {
    use std::io::Write;
    let ps = match shape {
        TerminalCursorShape::Block => 2,
        TerminalCursorShape::Underline => 4,
        TerminalCursorShape::Bar => 6,
    };
    let _ = write!(buf, "\x1b[{} q", ps);
}

// --- Terminal color depth (issue #154) ------------------------------------
//
// GNU downsamples realized face colors to the terminal's palette
// (`tty_default_color_cells` / `tty-color-approximate`). Emitting 24-bit
// `38;2;r;g;b` on every terminal means a terminal that doesn't support
// truecolor (Linux console, tmux without `Tc`, strict 256-color emulators)
// silently drops the color -> no syntax highlighting at all with `neomacs -nw`.
// Pick the SGR form from the detected color-cell count instead.
const TIER_NONE: u8 = 0;
const TIER_BASIC: u8 = 1; // 8/16 ANSI colors
const TIER_256: u8 = 2;
const TIER_TRUECOLOR: u8 = 3;

/// This terminal's capabilities, as GNU keeps them on `struct tty_display_info`.
///
/// One record, read by every emission path, so the color depth and the attribute
/// capabilities cannot be answered from two different places. Defaults to
/// [`TtyAttributeCapabilities::full`] so an uninitialised path (a test, a
/// terminfo entry that cannot be read) keeps the previous behavior instead of
/// silently dropping highlighting.
static CAPABILITIES: std::sync::LazyLock<std::sync::RwLock<TtyAttributeCapabilities>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(TtyAttributeCapabilities::full()));

/// The capabilities registered for this terminal.
pub fn capabilities() -> TtyAttributeCapabilities {
    CAPABILITIES
        .read()
        .map(|caps| caps.clone())
        .unwrap_or_else(|_| TtyAttributeCapabilities::full())
}

/// Register what this terminal can render — the terminfo answers GNU reads in
/// `init_tty`. Called once at TTY init from the frontend.
pub fn set_capabilities(caps: TtyAttributeCapabilities) {
    if let Ok(mut slot) = CAPABILITIES.write() {
        *slot = caps;
    }
}

/// Set the color half of the capabilities from the detected color-cell count
/// (GNU `tty_default_color_cells`): >=2^24 truecolor, >=256 indexed, >=8 basic
/// ANSI, else monochrome. Color depth is detected separately from the terminfo
/// attribute strings, so it has its own setter over the same record.
pub fn set_color_tier(color_cells: i64) {
    if let Ok(mut slot) = CAPABILITIES.write() {
        slot.color_cells = color_cells;
    }
}

/// GNU's color-depth buckets, from the capability record's cell count.
fn color_tier(caps: &TtyAttributeCapabilities) -> u8 {
    if caps.color_cells >= 16_777_216 {
        TIER_TRUECOLOR
    } else if caps.color_cells >= 256 {
        TIER_256
    } else if caps.color_cells >= 8 {
        TIER_BASIC
    } else {
        TIER_NONE
    }
}

/// The 6 channel intensities of the xterm 6x6x6 color cube.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// The 16 aixterm/xterm system colors, GNU `xterm-standard-colors`
/// (lisp/term/xterm.el:748-768). They lead `tty-color-alist`, so on an exact
/// tie they win over the cube entry with the same RGB -- which is why GNU
/// answers 0 for #000000 and 15 for #ffffff rather than 16 and 231.
const SYSTEM_COLORS: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (205, 0, 0),
    (0, 205, 0),
    (205, 205, 0),
    (0, 0, 238),
    (205, 0, 205),
    (0, 205, 205),
    (229, 229, 229),
    (127, 127, 127),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (92, 92, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];

/// RGB of xterm-256 palette entry INDEX: the system colors (0..=15), the
/// 6x6x6 cube (16..=231) or the grayscale ramp (232..=255).
fn xterm_256_entry(index: u8) -> (u8, u8, u8) {
    if index < 16 {
        return SYSTEM_COLORS[index as usize];
    }
    if index >= 232 {
        let gray = 8 + 10 * (index - 232);
        return (gray, gray, gray);
    }
    let offset = index - 16;
    (
        CUBE_LEVELS[(offset / 36) as usize],
        CUBE_LEVELS[((offset / 6) % 6) as usize],
        CUBE_LEVELS[(offset % 6) as usize],
    )
}

/// Angle between a color and the gray diagonal of the RGB cube, GNU
/// `tty-color-off-gray-diag` (lisp/term/tty-colors.el:866-873).
fn off_gray_diagonal(r: u8, g: u8, b: u8) -> f64 {
    let (r, g, b) = (r as f64, g as f64, b as f64);
    let magnitude = (3.0 * (r * r + g * g + b * b)).sqrt();
    if magnitude < 1.0 {
        return 0.0;
    }
    ((r + g + b) / magnitude).clamp(-1.0, 1.0).acos()
}

/// Nearest xterm-256 palette index for an RGB triple, over the whole palette.
///
/// GNU does not quantize in the writer at all: `turn_on_face` (src/term.c)
/// emits the index the realized face already carries, and that index came from
/// `tty-color-approximate` (lisp/term/tty-colors.el:875-915), which scans the
/// WHOLE palette for the smallest squared 8-bit RGB distance -- skipping
/// candidates that sit on the gray diagonal whenever the REQUESTED color is
/// 0.065 radians or more off it.
///
/// This used to short-circuit every near-gray straight into the grayscale
/// ramp, which is not a nearest search: #afafaf has an EXACT cube entry at 145,
/// but the ramp branch answered 248 (#a8a8a8, distance 147). That is Gruvbox
/// light's comment / org-verbatim / org-document-info-keyword foreground, and
/// it is why probing `tty-color-approximate` itself never reproduced the
/// divergence -- the Lisp palette was right and only the writer disagreed.
/// The scan is 256 candidates of integer arithmetic, bounded and allocation
/// free, in the same ascending order GNU's alist is consulted so that exact
/// ties resolve to the same index.
pub fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    let favor_non_gray = off_gray_diagonal(r, g, b) >= 0.065;
    let mut best_index = 0_u8;
    let mut best_distance = u32::MAX;
    for index in 0..=255_u8 {
        let (cr, cg, cb) = xterm_256_entry(index);
        if favor_non_gray && cr == cg && cg == cb {
            continue;
        }
        let difference = |lhs: u8, rhs: u8| -> u32 {
            let delta = lhs as i32 - rhs as i32;
            (delta * delta) as u32
        };
        let distance = difference(r, cr) + difference(g, cg) + difference(b, cb);
        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
    }
    best_index
}

/// Nearest basic-ANSI color as `(base 0..7, bright)` for an RGB triple.
pub fn rgb_to_ansi_basic(r: u8, g: u8, b: u8) -> (u8, bool) {
    let on = |v: u8| -> u8 { u8::from(v > 100) };
    let base = on(r) | (on(g) << 1) | (on(b) << 2);
    (base, r.max(g).max(b) > 170)
}

fn write_fg(buf: &mut Vec<u8>, r: u8, g: u8, b: u8, caps: &TtyAttributeCapabilities) {
    use std::io::Write;
    match color_tier(caps) {
        TIER_TRUECOLOR => {
            let _ = write!(buf, "\x1b[38;2;{r};{g};{b}m");
        }
        TIER_256 => {
            let _ = write!(buf, "\x1b[38;5;{}m", rgb_to_256(r, g, b));
        }
        TIER_BASIC => {
            let (base, bright) = rgb_to_ansi_basic(r, g, b);
            let _ = write!(buf, "\x1b[{}m", if bright { 90 + base } else { 30 + base });
        }
        _ => {}
    }
}

fn write_bg(buf: &mut Vec<u8>, r: u8, g: u8, b: u8, caps: &TtyAttributeCapabilities) {
    use std::io::Write;
    match color_tier(caps) {
        TIER_TRUECOLOR => {
            let _ = write!(buf, "\x1b[48;2;{r};{g};{b}m");
        }
        TIER_256 => {
            let _ = write!(buf, "\x1b[48;5;{}m", rgb_to_256(r, g, b));
        }
        TIER_BASIC => {
            let (base, bright) = rgb_to_ansi_basic(r, g, b);
            let _ = write!(buf, "\x1b[{}m", if bright { 100 + base } else { 40 + base });
        }
        _ => {}
    }
}

/// Write ANSI SGR (select graphic rendition) escape sequences for the given
/// attributes, using the capabilities registered for this terminal.
fn write_sgr(buf: &mut Vec<u8>, attrs: &CellAttrs) {
    write_sgr_with_capabilities(buf, attrs, &capabilities());
}

/// GNU `turn_on_face` (src/term.c): emit each attribute only when the terminal
/// has the capability for it, with GNU's fallbacks — a slant becomes `dim` where
/// there is no `sitm`, and a styled underline becomes a plain one where there is
/// no `Smulx`.
///
/// Always resets first, then enables the needed attributes.
pub fn write_sgr_with_capabilities(
    buf: &mut Vec<u8>,
    attrs: &CellAttrs,
    caps: &TtyAttributeCapabilities,
) {
    // Reset all attributes first.
    buf.extend_from_slice(b"\x1b[0m");

    if attrs.bold && caps.supports(TtyCapability::Bold) {
        buf.extend_from_slice(b"\x1b[1m");
    }
    if attrs.italic {
        match caps.italic_rendition() {
            TtyItalicRendition::Italic => buf.extend_from_slice(b"\x1b[3m"),
            // GNU: "Italics not supported, use dim instead."
            TtyItalicRendition::Dim => buf.extend_from_slice(b"\x1b[2m"),
            TtyItalicRendition::None => {}
        }
    }
    if attrs.underline != 0 && caps.supports(TtyCapability::Underline) {
        let styled = caps.supports(TtyCapability::UnderlineStyled);
        match attrs.underline {
            // A styled underline needs `Smulx`; without it GNU emits the plain
            // `smul` sequence rather than a parameter the terminal cannot read.
            2 if styled => buf.extend_from_slice(b"\x1b[4:2m"), // double underline
            3 if styled => buf.extend_from_slice(b"\x1b[4:3m"), // curly/wave underline
            4 if styled => buf.extend_from_slice(b"\x1b[4:4m"), // dotted underline
            5 if styled => buf.extend_from_slice(b"\x1b[4:5m"), // dashed underline
            _ => buf.extend_from_slice(b"\x1b[4m"),             // single underline
        }
    }
    if attrs.strikethrough && caps.supports(TtyCapability::StrikeThrough) {
        buf.extend_from_slice(b"\x1b[9m");
    }
    if attrs.inverse && caps.supports(TtyCapability::Inverse) {
        buf.extend_from_slice(
            caps.standout_sequence
                .as_deref()
                .expect("supported standout has a control sequence"),
        );
    }

    // GNU term.c only emits color SGR for specified TTY colors.
    // `None` mirrors FACE_TTY_DEFAULT_FG_COLOR/BG_COLOR.
    if let Some((r, g, b)) = attrs.fg {
        write_fg(buf, r, g, b, caps);
    } else {
        buf.extend_from_slice(b"\x1b[39m");
    }
    if let Some((r, g, b)) = attrs.bg {
        write_bg(buf, r, g, b, caps);
    } else {
        buf.extend_from_slice(b"\x1b[49m");
    }
}

fn write_cell_contents(buf: &mut Vec<u8>, cell: &TtyCell) {
    let mut bytes = [0u8; 4];
    let s = cell.ch.encode_utf8(&mut bytes);
    buf.extend_from_slice(s.as_bytes());
    if let Some(ext) = cell.extenders.as_deref() {
        buf.extend_from_slice(ext.as_bytes());
    }
}

/// Convert a `Glyph` to its display character.
/// Whether `glyph` is a complex-run member's padding cell carrying its own
/// per-cell grapheme (a non-blank `Char` or a `Composite`), as opposed to a
/// blank wide-character padding slot. These cells let the terminal decompose
/// a contextual-shaping run that the GUI renders as one shaped Composite.
fn is_run_member_padding_cell(glyph: &Glyph) -> bool {
    glyph.padding
        && match &glyph.glyph_type {
            GlyphType::Char { ch } => *ch != ' ',
            GlyphType::Composite { .. } => true,
            _ => false,
        }
}

/// The per-cell grapheme text carried by a run-member padding cell.
fn cell_grapheme_string(glyph: &Glyph) -> String {
    match &glyph.glyph_type {
        GlyphType::Char { ch } => ch.to_string(),
        GlyphType::Composite { text } => text.to_string(),
        _ => String::new(),
    }
}

/// Zero-width format joiners/selectors a terminal should not draw as their own
/// glyph: ZWJ, ZWNJ, and the variation selectors (incl. the supplement).
fn is_tty_skippable_format(ch: char) -> bool {
    matches!(
        ch as u32,
        0x200C | 0x200D | 0xFE00..=0xFE0F | 0xE0100..=0xE01EF
    )
}

fn glyph_to_char(glyph: &Glyph) -> char {
    match &glyph.glyph_type {
        GlyphType::Char { ch } => *ch,
        GlyphType::Composite { text } => text.chars().next().unwrap_or(' '),
        GlyphType::Stretch { .. } => ' ',
        GlyphType::Image { .. }
        | GlyphType::Video { .. }
        | GlyphType::Xwidget { .. }
        | GlyphType::Surface { .. } => ' ',
        GlyphType::Glyphless { ch } => *ch,
    }
}

/// A `width_cols`-wide TTY placeholder for a shader surface: `[shader]`
/// centered in a light-shade fill, or just the fill when the reserved width is
/// too narrow for the label. Surfaces are GPU-only, so a terminal shows this
/// marker rather than the blank space the reserved columns would otherwise be.
fn surface_tty_placeholder(width_cols: usize) -> String {
    const LABEL: &str = "[shader]";
    let label_len = LABEL.chars().count();
    if width_cols >= label_len {
        let fill = width_cols - label_len;
        let left = fill / 2;
        let right = fill - left;
        let mut s = String::with_capacity(width_cols + LABEL.len());
        s.extend(std::iter::repeat_n('░', left));
        s.push_str(LABEL);
        s.extend(std::iter::repeat_n('░', right));
        s
    } else {
        std::iter::repeat_n('░', width_cols).collect()
    }
}

#[cfg(test)]
#[path = "rif_test.rs"]
mod tests;

impl TtyRif {
    /// Debug: dump the desired grid content as plain text lines.
    pub fn dump_desired(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for row in 0..self.desired.height {
            let mut line = String::new();
            for col in 0..self.desired.width {
                let idx = row * self.desired.width + col;
                line.push(self.desired.cells[idx].ch);
            }
            lines.push(line);
        }
        lines
    }

    fn dump_tty_glyphs_to_log(&self) {
        tracing::info!(
            target: "neomacs_display_protocol::tty_rif",
            "tty glyph dump: cursor_visible={} cursor_row={} cursor_col={} cursor_shape={:?} default_bg={:?} default_face={:?} blank_tail={:?}",
            self.cursor_visible,
            self.cursor_row,
            self.cursor_col,
            self.cursor_shape,
            self.default_bg,
            self.faces.get(&FaceId::new(0)).map(|face| (
                face.use_default_foreground,
                face.use_default_background,
            )),
            self.caps.blank_tail,
        );
        for (row, line) in self.dump_desired().iter().enumerate() {
            tracing::info!(
                target: "neomacs_display_protocol::tty_rif",
                "tty row {:03}: {:?}",
                row,
                line
            );
        }
    }

    fn dump_frame_display_state_to_log(
        &self,
        state: &FrameDisplayState,
        origin_col: i64,
        origin_row: i64,
    ) {
        tracing::info!(
            target: "neomacs_display_protocol::tty_rif",
            "tty matrix dump: frame={} origin=({}, {}) windows={}",
            state.frame_placement.frame(),
            origin_col,
            origin_row,
            state.window_matrices.len()
        );
        for entry in &state.window_matrices {
            tracing::info!(
                target: "neomacs_display_protocol::tty_rif",
                "tty matrix window={} selected={} bounds=({:.1},{:.1},{:.1},{:.1}) text_bounds=({:.1},{:.1},{:.1},{:.1}) rows={}",
                entry.window_id,
                entry.selected,
                entry.pixel_bounds.x,
                entry.pixel_bounds.y,
                entry.pixel_bounds.width,
                entry.pixel_bounds.height,
                entry.text_pixel_bounds.x,
                entry.text_pixel_bounds.y,
                entry.text_pixel_bounds.width,
                entry.text_pixel_bounds.height,
                entry.matrix.rows.len()
            );
            for (row_idx, row) in entry.matrix.rows.iter().enumerate() {
                if !row.enabled && row.total_glyphs() == 0 {
                    continue;
                }
                tracing::info!(
                    target: "neomacs_display_protocol::tty_rif",
                    "tty matrix row window={} idx={} role={:?} enabled={} ends_at_zv={} pixel_y={:.1} height={:.1} ascent={:.1} used=({},{},{}) last_text={:?} text={:?}",
                    entry.window_id,
                    row_idx,
                    row.role,
                    row.enabled,
                    row.ends_at_zv,
                    row.pixel_y,
                    row.height_px,
                    row.ascent_px,
                    row.used(GlyphArea::LeftMargin),
                    row.used(GlyphArea::Text),
                    row.used(GlyphArea::RightMargin),
                    row.glyphs[GlyphArea::Text.index()].last().map(|glyph| (
                        glyph.face_id,
                        glyph.legacy_charpos(),
                        &glyph.glyph_type,
                    )),
                    glyph_row_debug_text(row)
                );
            }
        }
    }
}

fn glyph_row_debug_text(row: &GlyphRow) -> String {
    let mut text = String::new();
    for area in &row.glyphs {
        for glyph in area {
            match &glyph.glyph_type {
                GlyphType::Composite { text: cluster } => text.push_str(cluster),
                GlyphType::Stretch { width_cols } => {
                    text.extend(std::iter::repeat_n(' ', usize::from((*width_cols).max(1))));
                }
                _ => text.push(glyph_to_char(glyph)),
            }
        }
    }
    text
}
