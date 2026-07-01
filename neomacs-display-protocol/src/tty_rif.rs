//! TTY rendering backend -- reads GlyphMatrix, outputs ANSI escape sequences.
//!
//! This implements a terminal display backend matching the approach of
//! GNU Emacs's term.c. It maintains two character grids (current and desired),
//! rasterizes `FrameDisplayState` into the desired grid, then diffs against
//! current to produce minimal ANSI output.
//!
//! Runs on the evaluator thread (single-threaded, no channel needed).

use crate::face::{Face, FaceAttributes};
use crate::frame_glyphs::CursorStyle;
use crate::glyph_matrix::*;
use crate::types::Color;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Cell attributes
// ---------------------------------------------------------------------------

/// Attributes for a single terminal cell (maps to ANSI SGR sequences).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl Default for CellAttrs {
    fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: 0,
            strikethrough: false,
            inverse: false,
        }
    }
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
    faces: HashMap<u32, Face>,
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
    pub fn set_faces(&mut self, faces: HashMap<u32, Face>) {
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
        let default_face = self.faces.get(&0);
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
        self.install_state_faces(root);
        self.desired.clear(self.default_bg);
        self.cursor_visible = false;
        self.cursor_shape = TerminalCursorShape::Block;

        self.rasterize_state_at(root, 0, 0, false);

        for child in children_bottom_to_top {
            if child.parent_id != root.frame_id {
                continue;
            }
            let origin_col = child.parent_x.max(0.0).round() as usize;
            let origin_row = child.parent_y.max(0.0).round() as usize;
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
        origin_col: usize,
        origin_row: usize,
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
            let max_row = origin_row
                .saturating_add(state.frame_rows)
                .min(self.desired.height);
            let max_col = origin_col
                .saturating_add(state.frame_cols)
                .min(self.desired.width);
            for row in origin_row..max_row {
                for col in origin_col..max_col {
                    self.desired.set(row, col, ' ', attrs, false);
                }
            }
        }

        if let Some(cursor) = state.phys_cursor.as_ref() {
            let (cursor_col, cursor_row) =
                terminal_cursor_cell(cursor.x, cursor.y, state.char_width, state.char_height);
            self.cursor_row = cursor_row.saturating_add(origin_row as u16);
            self.cursor_col = cursor_col.saturating_add(origin_col as u16);
            self.cursor_visible = true;
            self.cursor_shape = match cursor.style {
                CursorStyle::FilledBox | CursorStyle::Hollow => TerminalCursorShape::Block,
                CursorStyle::Bar(_) => TerminalCursorShape::Bar,
                CursorStyle::Hbar(_) => TerminalCursorShape::Underline,
            };
        }

        for fill in &state.face_fills {
            self.rasterize_face_fill(origin_col, origin_row, state, fill);
        }

        for frame_row in &state.frame_chrome_rows {
            let char_w = state.char_width.max(1.0);
            let win_col = origin_col + (frame_row.pixel_bounds.x / char_w) as usize;
            self.rasterize_glyph_row(
                win_col,
                origin_row + frame_row.row_index as usize,
                &frame_row.row,
            );
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
                let row_col = origin_col + (row_bounds.x / char_w).round().max(0.0) as usize;
                let row_base = origin_row + (row_bounds.y / char_h).round().max(0.0) as usize;
                // GNU keeps two coordinate domains in each glyph row:
                // VPOS/HPOS are grid coordinates, while Y/X are pixel
                // coordinates for GUI redisplay.  TTY output is written by
                // matrix row index, so pixel_y/height_px must not stretch or
                // skip terminal rows.
                self.rasterize_glyph_row(row_col, row_base + row_idx, glyph_row);
            }
        }

        // Frame-level menu bar.  Mirrors GNU `display_menu_bar`
        // (`xdisp.c:27444`): the menu bar is independent of any window's
        // glyph matrix and is drawn at the very top of the frame using
        // the `menu` face for every cell.  We rasterize it AFTER the
        // window matrices so the menu bar always wins at row 0..lines-1
        // (the layout engine has already shifted the root window down to
        // make room).
        if let Some(menu_bar) = state.menu_bar.as_ref() {
            self.rasterize_menu_bar_at(menu_bar, origin_col, origin_row, state.frame_cols);
        }

        // GNU's TTY redisplay does not paint a cursor glyph into the
        // frame matrix.  It writes ordinary glyph cells, then
        // `tty_set_cursor` moves the hardware cursor and
        // `tty_update_end` shows it.  Keep cursor state separate from
        // cell attributes so blank cells retain the terminal-default
        // background.
    }

    fn draw_child_border(
        &mut self,
        child: &FrameDisplayState,
        origin_col: usize,
        origin_row: usize,
    ) {
        if child.undecorated {
            return;
        }
        self.install_state_faces(child);
        let attrs = self.resolve_attrs(0);
        let width = child.frame_cols;
        let height = child.frame_rows;
        if width == 0 || height == 0 {
            return;
        }

        let left = origin_col.saturating_sub(1);
        let right = origin_col.saturating_add(width);
        let top = origin_row.saturating_sub(1);
        let bottom = origin_row.saturating_add(height);

        if origin_row > 0 {
            for col in origin_col..origin_col.saturating_add(width).min(self.desired.width) {
                self.desired.set(top, col, '-', attrs, false);
            }
            if origin_col > 0 {
                self.desired.set(top, left, '+', attrs, false);
            }
            if right < self.desired.width {
                self.desired.set(top, right, '+', attrs, false);
            }
        }

        if bottom < self.desired.height {
            for col in origin_col..origin_col.saturating_add(width).min(self.desired.width) {
                self.desired.set(bottom, col, '-', attrs, false);
            }
            if origin_col > 0 {
                self.desired.set(bottom, left, '+', attrs, false);
            }
            if right < self.desired.width {
                self.desired.set(bottom, right, '+', attrs, false);
            }
        }

        let row_end = origin_row.saturating_add(height).min(self.desired.height);
        for row in origin_row..row_end {
            if origin_col > 0 {
                self.desired.set(row, left, '|', attrs, false);
            }
            if right < self.desired.width {
                self.desired.set(row, right, '|', attrs, false);
            }
        }
    }

    /// Paint the TTY menu bar into rows `0..menu_bar.lines`.
    ///
    /// Layout matches GNU `display_menu_bar`:
    ///
    /// * One leading space, then each item label followed by one
    ///   trailing space (so items render as `" File  Edit  Options ..."`
    ///   when the leading-space-of-the-next-item visually doubles as
    ///   the trailing space of the previous one — see GNU's
    ///   `display_string (NULL, string, Qnil, 0, 0, &it, SCHARS (string) + 1, ...)`
    ///   pattern).
    /// * Remainder of the row filled with spaces using the `menu` face,
    ///   matching GNU's `display_string ("", Qnil, ...)` tail call.
    /// * Items past the visible width are silently truncated; we
    ///   record `hpos = u16::MAX` for any item that didn't fit, so a
    ///   future hit-tester can ignore them (mirrors GNU storing the
    ///   item's column in slot `i+3` and only valid columns being
    ///   reachable via `tty_menu_activate`).
    fn rasterize_menu_bar_at(
        &mut self,
        menu_bar: &TtyMenuBarState,
        origin_col: usize,
        origin_row: usize,
        frame_cols: usize,
    ) {
        let attrs = CellAttrs {
            fg: (!menu_bar.use_default_foreground).then(|| rgb_pixel_to_tuple(menu_bar.fg)),
            bg: (!menu_bar.use_default_background).then(|| rgb_pixel_to_tuple(menu_bar.bg)),
            bold: menu_bar.bold,
            italic: false,
            underline: 0,
            strikethrough: false,
            inverse: menu_bar.inverse,
        };

        let lines = (menu_bar.lines as usize).min(self.desired.height.saturating_sub(origin_row));
        let width = frame_cols
            .min(self.desired.width.saturating_sub(origin_col))
            .max(0);
        if lines == 0 || width == 0 {
            return;
        }

        // Only line 0 of the menu bar carries items today.  Additional
        // wrap-rows would be filled with spaces; mirrors GNU which
        // also displays only the first menu-bar line on TTYs.
        for row in 0..lines {
            for col in 0..width {
                self.desired
                    .set(origin_row + row, origin_col + col, ' ', attrs, false);
            }
        }

        let menu_row = origin_row;
        let mut col: usize = 0;
        // GNU starts with the first item at column 0 (no leading
        // padding); the per-item label is `string + " "` (label plus
        // exactly one trailing space, see `SCHARS (string) + 1` in
        // `display_menu_bar`).
        for item in &menu_bar.items {
            if col >= width {
                break;
            }
            let item_start = col;
            let label_end = col + item.label.chars().count();
            for ch in item.label.chars() {
                if col >= width {
                    break;
                }
                self.desired
                    .set(menu_row, origin_col + col, ch, attrs, false);
                col += 1;
            }
            // Trailing space after the label, but only if there's room.
            // The space itself is part of the item's run so it shares
            // the menu face attrs (already painted as the row fill).
            if col < width && col == label_end {
                col += 1;
            }
            // Remember where we placed this item so a future hit-tester
            // can map screen column back to a key.
            let _ = item_start;
        }
    }

    /// Resolve face_id into terminal cell attributes.
    fn resolve_attrs(&self, face_id: u32) -> CellAttrs {
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
    fn write_grapheme_cell(&mut self, row: usize, col: &mut usize, text: &str, attrs: CellAttrs) {
        let mut chars = text.chars().filter(|c| !is_tty_skippable_format(*c));
        let base = chars.next().unwrap_or(' ');
        let rest: String = chars.collect();
        self.desired
            .set_cluster(row, *col, base, &rest, attrs, false);
        *col += 1;
    }

    /// Diff the desired grid against the current grid and generate ANSI escape
    /// sequences for the changed cells.
    ///
    /// After this call, `current` is swapped to reflect what is now on screen.
    /// Retrieve the buffered output with [`take_output`].
    pub fn diff_and_render(&mut self) {
        self.output.clear();

        // Hide cursor during update to avoid flicker.
        self.output.extend_from_slice(b"\x1b[?25l");

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
            if row_has_composite_cells(desired_row) || row_has_composite_cells(current_row) {
                last_changed = desired_row.len() - 1;
                write_cursor_goto(&mut self.output, row as u16 + 1, first_changed as u16 + 1);
                write_sgr(&mut self.output, &CellAttrs::default());
                last_attrs = Some(CellAttrs::default());
                for _ in first_changed..=last_changed {
                    self.output.push(b' ');
                }
            }
            write_cursor_goto(&mut self.output, row as u16 + 1, first_changed as u16 + 1);

            for desired in &desired_row[first_changed..=last_changed] {
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

        // Reset attributes after all updates.
        self.output.extend_from_slice(b"\x1b[0m");

        // Position cursor and show it if visible.
        if self.cursor_visible {
            write_cursor_goto(&mut self.output, self.cursor_row + 1, self.cursor_col + 1);
            write_cursor_shape(&mut self.output, self.cursor_shape);
            self.output.extend_from_slice(b"\x1b[?25h");
        }

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
        origin_col: usize,
        origin_row: usize,
        state: &FrameDisplayState,
        fill: &FaceFillItem,
    ) {
        let char_w = state.char_width.max(1.0);
        let char_h = state.char_height.max(1.0);
        let start_col = origin_col + (fill.bounds.x / char_w).round().max(0.0) as usize;
        let start_row = origin_row + (fill.bounds.y / char_h).round().max(0.0) as usize;
        let width_cols = (fill.bounds.width / char_w).ceil().max(0.0) as usize;
        let height_rows = (fill.bounds.height / char_h).ceil().max(0.0) as usize;
        if width_cols == 0 || height_rows == 0 {
            return;
        }

        let attrs = self.resolve_attrs(fill.face_id);
        let max_row = start_row
            .saturating_add(height_rows)
            .min(self.desired.height);
        let max_col = start_col.saturating_add(width_cols).min(self.desired.width);
        for row in start_row..max_row {
            for col in start_col..max_col {
                self.desired.set(row, col, ' ', attrs, false);
            }
        }
    }

    fn rasterize_glyph_row(
        &mut self,
        screen_col_start: usize,
        screen_row: usize,
        glyph_row: &GlyphRow,
    ) {
        if !glyph_row.enabled || screen_row >= self.desired.height {
            return;
        }

        let mut col = screen_col_start;

        for area_idx in 0..3 {
            let glyphs = &glyph_row.glyphs[area_idx];
            let mut glyph_idx = 0;
            while glyph_idx < glyphs.len() {
                let glyph = &glyphs[glyph_idx];
                if col >= self.desired.width {
                    break;
                }

                if glyph.padding {
                    let attrs = self.resolve_attrs(glyph.face_id);
                    self.desired.set(screen_row, col, ' ', attrs, true);
                    col += 1;
                    glyph_idx += 1;
                    continue;
                }

                let attrs = self.resolve_attrs(glyph.face_id);
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
                                if col >= self.desired.width {
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
                            if col >= self.desired.width {
                                break;
                            }
                            self.desired.set(screen_row, col, ' ', attrs, false);
                            col += 1;
                        }
                    }
                    _ => {
                        let ch = glyph_to_char(glyph);
                        self.desired.set(screen_row, col, ch, attrs, false);
                        col += 1;

                        let next_is_explicit_padding = glyph.wide
                            && glyphs
                                .get(glyph_idx + 1)
                                .is_some_and(|next_glyph| next_glyph.padding);
                        if glyph.wide && !next_is_explicit_padding && col < self.desired.width {
                            self.desired.set(screen_row, col, ' ', attrs, true);
                            col += 1;
                        }
                    }
                }
                glyph_idx += 1;
            }
        }

        // On TTY frames GNU has one terminal cursor, positioned after
        // glyph output by `tty_set_cursor`; row cursor markers do not
        // become painted cell attributes.
    }
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

/// Write ANSI SGR (select graphic rendition) escape sequences for the given
/// attributes. Always resets first, then enables the needed attributes.
fn write_sgr(buf: &mut Vec<u8>, attrs: &CellAttrs) {
    use std::io::Write;
    // Reset all attributes first.
    buf.extend_from_slice(b"\x1b[0m");

    if attrs.bold {
        buf.extend_from_slice(b"\x1b[1m");
    }
    if attrs.italic {
        buf.extend_from_slice(b"\x1b[3m");
    }
    match attrs.underline {
        1 => buf.extend_from_slice(b"\x1b[4m"),   // single underline
        2 => buf.extend_from_slice(b"\x1b[4:2m"), // double underline
        3 => buf.extend_from_slice(b"\x1b[4:3m"), // curly/wave underline
        4 => buf.extend_from_slice(b"\x1b[4:4m"), // dotted underline
        5 => buf.extend_from_slice(b"\x1b[4:5m"), // dashed underline
        _ => {}
    }
    if attrs.strikethrough {
        buf.extend_from_slice(b"\x1b[9m");
    }
    if attrs.inverse {
        buf.extend_from_slice(b"\x1b[7m");
    }

    // GNU term.c only emits color SGR for specified TTY colors.
    // `None` mirrors FACE_TTY_DEFAULT_FG_COLOR/BG_COLOR.
    if let Some((r, g, b)) = attrs.fg {
        let _ = write!(buf, "\x1b[38;2;{r};{g};{b}m");
    } else {
        buf.extend_from_slice(b"\x1b[39m");
    }
    if let Some((r, g, b)) = attrs.bg {
        let _ = write!(buf, "\x1b[48;2;{r};{g};{b}m");
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
        GlyphType::Image { .. } => ' ',
        GlyphType::Glyphless { ch } => *ch,
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
        origin_col: usize,
        origin_row: usize,
    ) {
        tracing::info!(
            target: "neomacs_display_protocol::tty_rif",
            "tty matrix dump: frame={} origin=({}, {}) windows={}",
            state.frame_id,
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
                    text.extend(std::iter::repeat(' ').take(usize::from((*width_cols).max(1))));
                }
                _ => text.push(glyph_to_char(glyph)),
            }
        }
    }
    text
}
