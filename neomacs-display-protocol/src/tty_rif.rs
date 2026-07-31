//! TTY rendering backend -- reads GlyphMatrix, outputs ANSI escape sequences.
//!
//! This implements a terminal display backend matching the approach of
//! GNU Emacs's term.c. It maintains two character grids (current and desired),
//! rasterizes `FrameDisplayState` into the desired grid, then diffs against
//! current to produce minimal ANSI output.
//!
//! Runs on the evaluator thread (single-threaded, no channel needed).

use crate::face::{Face, FaceAttributes};
use crate::frame_chrome::FrameChromeContent;
use crate::frame_glyphs::CursorStyle;
use crate::glyph_matrix::*;
use crate::tty_capabilities::{TtyAttributeCapabilities, TtyCapability, TtyItalicRendition};
use crate::types::{Color, FaceId};
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
    /// True if this is a padding cell for a wide (double-width) character.
    pub padding: bool,
    /// Grapheme-cluster extenders stacked on `ch` (None for ordinary cells).
    pub extenders: Option<Box<str>>,
}

impl Default for TtyCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            attrs: CellAttrs::default(),
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
}

impl TtyGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![TtyCell::default(); width * height];
        Self {
            width,
            height,
            cells,
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
            padding: false,
            extenders: None,
        };
        for cell in &mut self.cells {
            *cell = blank.clone();
        }
    }

    /// Set a cell at (row, col). No-op if out of bounds.
    pub fn set(&mut self, row: usize, col: usize, ch: char, attrs: CellAttrs, padding: bool) {
        if row < self.height && col < self.width {
            let idx = row * self.width + col;
            self.cells[idx] = TtyCell {
                ch,
                attrs,
                padding,
                extenders: None,
            };
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
            let idx = row * self.width + col;
            let ext = if extenders.is_empty() {
                None
            } else {
                Some(Box::<str>::from(extenders))
            };
            self.cells[idx] = TtyCell {
                ch,
                attrs,
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
    /// Force the next render to repaint every terminal cell.
    force_full_render: bool,
}

fn terminal_cursor_cell(x: f32, y: f32, char_width: f32, char_height: f32) -> (u16, u16) {
    let char_width = char_width.max(1.0);
    let char_height = char_height.max(1.0);
    ((x / char_width) as u16, (y / char_height) as u16)
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
            force_full_render: true,
        }
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
        root: &crate::SealedFramePresentation,
        children_bottom_to_top: &[crate::SealedFramePresentation],
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
            let band_col = origin_col + (band.bounds().x() / char_w).round() as i64;
            let band_row = origin_row + (band.bounds().y() / char_h).round() as i64;
            match band.content() {
                FrameChromeContent::DisplayRow(content) => {
                    self.rasterize_glyph_row(band_col, band_row, content.row());
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
                // Mirror FrameDisplayState::materialize(): buffer text rows are
                // laid out relative to the GNU TEXT_AREA, while mode-line,
                // header-line, tab-line, and minibuffer chrome remain
                // window-wide.  This is the TTY side of GNU's glyph matrix
                // margin reservation in dispnew.c: text-area glyph pointers are
                // offset past left margin columns, chrome rows are not.
                let row_bounds = entry.row_pixel_bounds(glyph_row.role);
                let row_col = origin_col + (row_bounds.x / char_w).round().max(0.0) as i64;
                let row_base = origin_row + (row_bounds.y / char_h).round().max(0.0) as i64;
                // GNU keeps two coordinate domains in each glyph row:
                // VPOS/HPOS are grid coordinates, while Y/X are pixel
                // coordinates for GUI redisplay.  TTY output is written by
                // matrix row index, so pixel_y/height_px must not stretch or
                // skip terminal rows.
                self.rasterize_glyph_row(
                    row_col,
                    row_base.saturating_add(usize_to_i64_saturating(row_idx)),
                    glyph_row,
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
        menu: &crate::frame_chrome::MenuBarContent,
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

        // Synchronized output (DECSET 2026): the terminal buffers everything
        // between h/l and presents it atomically, eliminating tearing during
        // partial updates. Supported by kitty/ghostty/wezterm/tmux/Windows
        // Terminal and ignored as an unknown private mode elsewhere; GNU's
        // terminal update has no equivalent.
        self.output.extend_from_slice(b"\x1b[?2026h");

        // Hide cursor during update to avoid flicker.
        self.output.extend_from_slice(b"\x1b[?25l");

        // Vertical scroll: when a run of rows merely shifted, tell the
        // terminal to move them (DECSTBM region + SU/SD) instead of
        // retransmitting every row - the issue-206 case where one scroll
        // step redrew the whole frame. See detect_scroll for the design.
        if !self.force_full_render
            && let Some(scroll) = detect_scroll(&self.current, &self.desired)
        {
            let n = scroll.delta.unsigned_abs();
            // Region-relative scroll: set margins, scroll, reset margins.
            self.output.extend_from_slice(
                format!("\x1b[{};{}r", scroll.top + 1, scroll.bottom + 1).as_bytes(),
            );
            // SGR reset first: SU/SD fill exposed lines with the current
            // background (BCE); make that the default background.
            self.output.extend_from_slice(b"\x1b[0m");
            if scroll.delta > 0 {
                self.output
                    .extend_from_slice(format!("\x1b[{n}S").as_bytes());
            } else {
                self.output
                    .extend_from_slice(format!("\x1b[{n}T").as_bytes());
            }
            self.output.extend_from_slice(b"\x1b[r");
            // Mirror the terminal-side move in our model of the screen, and
            // poison the exposed rows so the diff below repaints them (the
            // terminal filled them with blank cells whose exact attributes
            // we choose not to depend on).
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

        let mut last_attrs: Option<CellAttrs> = None;

        for row in 0..self.desired.height {
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

            // GNU term.c writes contiguous glyph runs with a single cursor
            // position update, then lets the terminal advance naturally.
            // Repaint the changed row span the same way so wide glyphs and
            // composed clusters are emitted as adjacent terminal text rather
            // than broken up by per-cell cursor moves.
            //
            // Real terminals are not uniformly reliable when a row containing
            // grapheme clusters is rewritten with different text.  If the
            // terminal's idea of the cluster width differs from our cell grid,
            // stale glyphs can remain past the internal changed span.  GNU's
            // TTY redisplay model treats the terminal as a stateful grid and
            // clears affected ranges before writing new glyphs; for composite
            // rows, clear and repaint the whole row tail so shrunk HELLO rows
            // cannot leave visible residue.
            let composite_row =
                row_has_composite_cells(desired_row) || row_has_composite_cells(current_row);
            if composite_row {
                last_changed = desired_row.len() - 1;
                write_cursor_goto(&mut self.output, row as u16 + 1, first_changed as u16 + 1);
                write_sgr(&mut self.output, &CellAttrs::default());
                last_attrs = Some(CellAttrs::default());
                for _ in first_changed..=last_changed {
                    self.output.push(b' ');
                }
            }
            // Multi-span emission (issue 206): a row with two separate
            // change regions used to be rewritten from the first to the
            // last changed cell in one span, retransmitting the untouched
            // middle. Split the [first, last] range into changed runs and
            // coalesce runs whose gap is cheaper to retransmit than a
            // cursor motion (a goto costs ~8 bytes; an unchanged text cell
            // usually 1). GNU's update_frame_line keeps one
            // begmatch/endmatch span per line; per-run emission with a
            // byte-cost coalesce rule strictly dominates it.
            const GOTO_COST_CELLS: usize = 8;
            let mut spans: Vec<(usize, usize)> = Vec::new();
            {
                let changed =
                    |col: usize| !desired_row[col].padding && desired_row[col] != current_row[col];
                let mut col = first_changed;
                while col <= last_changed {
                    if changed(col) {
                        let start = col;
                        while col <= last_changed && changed(col) {
                            col += 1;
                        }
                        match spans.last_mut() {
                            Some((_, end)) if start - *end <= GOTO_COST_CELLS => *end = col,
                            _ => spans.push((start, col)),
                        }
                    } else {
                        col += 1;
                    }
                }
            }
            if self.force_full_render || composite_row {
                // The composite path just space-cleared the whole tail on
                // the terminal, so every cell of the range must be
                // rewritten regardless of what `current` recorded.
                spans = vec![(first_changed, last_changed + 1)];
            }

            for &(start, end) in &spans {
                write_cursor_goto(&mut self.output, row as u16 + 1, start as u16 + 1);
                for desired in &desired_row[start..end] {
                    if desired.padding {
                        continue;
                    }

                    if last_attrs.as_ref() != Some(&desired.attrs) {
                        write_sgr(&mut self.output, &desired.attrs);
                        last_attrs = Some(desired.attrs);
                    }

                    write_cell_contents(&mut self.output, desired);
                }
            }
        }

        // Reset attributes after all updates.
        self.output.extend_from_slice(b"\x1b[0m");

        // Position cursor and show it if visible.
        if self.cursor_visible {
            write_cursor_goto(&mut self.output, self.cursor_row + 1, self.cursor_col + 1);
            write_cursor_shape(&mut self.output, self.cursor_shape);
            self.output.extend_from_slice(b"\x1b[?25h");
        }

        // End synchronized update: present the frame atomically.
        self.output.extend_from_slice(b"\x1b[?2026l");

        // Swap: current now reflects what is on screen.
        std::mem::swap(&mut self.current, &mut self.desired);
        self.force_full_render = false;
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
        screen_col_start: i64,
        screen_row: i64,
        glyph_row: &GlyphRow,
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

        for area_idx in 0..3 {
            let glyphs = &glyph_row.glyphs[area_idx];
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
                        self.desired.set(
                            screen_row,
                            col,
                            ' ',
                            attrs,
                            preceding_wide_base_visible.take().unwrap_or(true),
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
                                self.desired.set(screen_row, col, ' ', attrs, false);
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
                                self.desired.set(screen_row, col, ch, attrs, false);
                            }
                            col = col.saturating_add(1);
                        }
                    }
                    _ => {
                        let ch = glyph_to_char(glyph);
                        if let Some(col) = visible_cell(col, self.desired.width) {
                            self.desired.set(screen_row, col, ch, attrs, false);
                        }
                        col = col.saturating_add(1);

                        let next_is_explicit_padding = glyph.wide
                            && glyphs
                                .get(glyph_idx + 1)
                                .is_some_and(|next_glyph| next_glyph.padding);
                        if glyph.wide && !next_is_explicit_padding && col < screen_width {
                            if let Some(col) = visible_cell(col, self.desired.width) {
                                self.desired.set(screen_row, col, ' ', attrs, base_visible);
                            }
                            col = col.saturating_add(1);
                        }
                    }
                }
                preceding_wide_base_visible = glyph.wide.then_some(base_visible);
                glyph_idx += 1;
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
fn detect_scroll(current: &TtyGrid, desired: &TtyGrid) -> Option<DetectedScroll> {
    const MIN_RUN: usize = 4;
    let (w, h) = (desired.width, desired.height);
    if w != current.width || h != current.height || h < MIN_RUN + 1 {
        return None;
    }
    let old_hash: Vec<u64> = (0..h)
        .map(|r| row_hash(&current.cells[r * w..(r + 1) * w]))
        .collect();
    let new_hash: Vec<u64> = (0..h)
        .map(|r| row_hash(&desired.cells[r * w..(r + 1) * w]))
        .collect();

    // Changed band: rows outside it already match in place.
    let top = (0..h).find(|&r| old_hash[r] != new_hash[r])?;
    let bottom = (0..h).rfind(|&r| old_hash[r] != new_hash[r])?;

    // Vote for deltas using positions of equal hashes inside the band.
    let mut by_hash: rustc_hash::FxHashMap<u64, Vec<usize>> = rustc_hash::FxHashMap::default();
    for r in top..=bottom {
        by_hash.entry(old_hash[r]).or_default().push(r);
    }
    let mut votes: rustc_hash::FxHashMap<isize, usize> = rustc_hash::FxHashMap::default();
    for i in top..=bottom {
        if let Some(js) = by_hash.get(&new_hash[i]) {
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

    // Longest contiguous run where desired[i] == current[i + delta], with
    // real cell equality (hashes only route). Composite rows are excluded:
    // the conservative full-tail repaint path owns them.
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
        } else if let Some(lo) = run_lo.take() {
            if i - lo > best_len {
                best_lo = lo;
                best_len = i - lo;
            }
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
    let srgb = c.linear_to_srgb();
    (
        (srgb.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (srgb.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (srgb.b.clamp(0.0, 1.0) * 255.0).round() as u8,
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
static CAPABILITIES: std::sync::RwLock<TtyAttributeCapabilities> =
    std::sync::RwLock::new(TtyAttributeCapabilities::full());

/// The capabilities registered for this terminal.
pub fn capabilities() -> TtyAttributeCapabilities {
    CAPABILITIES
        .read()
        .map(|caps| *caps)
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

/// Nearest xterm-256 palette index for an RGB triple (16 system + 6x6x6 cube +
/// 24-step grayscale).
pub fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    if max - min < 8 {
        if r < 8 {
            return 16;
        }
        if r > 238 {
            return 231;
        }
        return 232 + ((r as u16 - 8) / 10).min(23) as u8;
    }
    let level = |v: u8| -> u16 {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            ((v as u16 - 35) / 40).min(5)
        }
    };
    (16 + 36 * level(r) + 6 * level(g) + level(b)) as u8
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
        buf.extend_from_slice(b"\x1b[7m");
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
#[path = "tty_rif_test.rs"]
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
            "tty glyph dump: cursor_visible={} cursor_row={} cursor_col={} cursor_shape={:?}",
            self.cursor_visible,
            self.cursor_row,
            self.cursor_col,
            self.cursor_shape
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
                    "tty matrix row window={} idx={} role={:?} enabled={} pixel_y={:.1} height={:.1} ascent={:.1} used=({},{},{}) text={:?}",
                    entry.window_id,
                    row_idx,
                    row.role,
                    row.enabled,
                    row.pixel_y,
                    row.height_px,
                    row.ascent_px,
                    row.used(GlyphArea::LeftMargin),
                    row.used(GlyphArea::Text),
                    row.used(GlyphArea::RightMargin),
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
