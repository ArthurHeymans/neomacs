//! Terminal/TTY display backend.
//!
//! Renders Neomacs frames to a terminal using ANSI escape sequences.
//! Supports:
//! - Alternate screen buffer (smcup/rmcup)
//! - Raw mode terminal setup/teardown
//! - 256-color and 24-bit true color via ANSI SGR
//! - Cursor positioning
//! - Frame diffing: maintains a previous-frame grid and only outputs changed cells
//! - Basic text rendering from Scene and FrameGlyphBuffer data

pub mod rif;

use std::io::{self, Write};

use crate::core::error::{DisplayError, DisplayResult};
use crate::core::frame_glyphs::{CursorStyle, FrameGlyph, FrameGlyphBuffer, WindowCursor};
use crate::core::scene::Scene;
use crate::core::types::Color;

// ---------------------------------------------------------------------------
// ANSI escape sequence helpers
// ---------------------------------------------------------------------------

/// ANSI escape sequence constants and builders.
///
/// All sequences target xterm-compatible terminals (virtually all modern
/// terminals). No terminfo dependency is required.
pub mod ansi {
    /// CSI (Control Sequence Introducer)
    pub const CSI: &str = "\x1b[";

    /// Enter alternate screen buffer (smcup equivalent)
    pub const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
    /// Leave alternate screen buffer (rmcup equivalent)
    pub const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";

    /// Hide cursor
    pub const HIDE_CURSOR: &str = "\x1b[?25l";
    /// Show cursor
    pub const SHOW_CURSOR: &str = "\x1b[?25h";

    /// Reset all SGR attributes
    pub const SGR_RESET: &str = "\x1b[0m";

    /// Clear entire screen
    pub const CLEAR_SCREEN: &str = "\x1b[2J";

    /// Move cursor to home position (1,1)
    pub const CURSOR_HOME: &str = "\x1b[H";

    /// Enable mouse tracking (SGR extended mode)
    pub const ENABLE_MOUSE: &str = "\x1b[?1006h\x1b[?1003h";
    /// Disable mouse tracking
    pub const DISABLE_MOUSE: &str = "\x1b[?1003l\x1b[?1006l";

    /// Enable bracketed paste mode
    pub const ENABLE_BRACKETED_PASTE: &str = "\x1b[?2004h";
    /// Disable bracketed paste mode
    pub const DISABLE_BRACKETED_PASTE: &str = "\x1b[?2004l";

    /// Move cursor to (row, col), both 1-based.
    pub fn cursor_goto(buf: &mut Vec<u8>, row: u16, col: u16) {
        use std::io::Write;
        let _ = write!(buf, "\x1b[{};{}H", row, col);
    }

    /// Set the visible terminal cursor shape using DECSCUSR (CSI Ps SP q).
    ///
    /// Uses steady shapes to avoid fighting Emacs-side blink control:
    /// 2 = block, 4 = underline, 6 = bar.
    pub fn cursor_shape(buf: &mut Vec<u8>, shape: TerminalCursorShape) {
        use std::io::Write;
        let ps = match shape {
            TerminalCursorShape::Block => 2,
            TerminalCursorShape::Underline => 4,
            TerminalCursorShape::Bar => 6,
        };
        let _ = write!(buf, "\x1b[{} q", ps);
    }

    /// Set foreground color using 24-bit true color (SGR 38;2;r;g;b).
    /// r, g, b are 0-255.
    pub fn fg_truecolor(buf: &mut Vec<u8>, r: u8, g: u8, b: u8) {
        use std::io::Write;
        let _ = write!(buf, "\x1b[38;2;{};{};{}m", r, g, b);
    }

    /// Set background color using 24-bit true color (SGR 48;2;r;g;b).
    /// r, g, b are 0-255.
    pub fn bg_truecolor(buf: &mut Vec<u8>, r: u8, g: u8, b: u8) {
        use std::io::Write;
        let _ = write!(buf, "\x1b[48;2;{};{};{}m", r, g, b);
    }

    /// Set foreground color using 256-color palette (SGR 38;5;n).
    pub fn fg_256(buf: &mut Vec<u8>, index: u8) {
        use std::io::Write;
        let _ = write!(buf, "\x1b[38;5;{}m", index);
    }

    /// Set background color using 256-color palette (SGR 48;5;n).
    pub fn bg_256(buf: &mut Vec<u8>, index: u8) {
        use std::io::Write;
        let _ = write!(buf, "\x1b[48;5;{}m", index);
    }

    /// Set bold attribute
    pub const SGR_BOLD: &str = "\x1b[1m";
    /// Set italic attribute
    pub const SGR_ITALIC: &str = "\x1b[3m";
    /// Set underline attribute
    pub const SGR_UNDERLINE: &str = "\x1b[4m";
    /// Set double underline
    pub const SGR_DOUBLE_UNDERLINE: &str = "\x1b[21m";
    /// Set curly/wave underline (not universally supported)
    pub const SGR_CURLY_UNDERLINE: &str = "\x1b[4:3m";
    /// Set dotted underline
    pub const SGR_DOTTED_UNDERLINE: &str = "\x1b[4:4m";
    /// Set dashed underline
    pub const SGR_DASHED_UNDERLINE: &str = "\x1b[4:5m";
    /// Set strikethrough attribute
    pub const SGR_STRIKETHROUGH: &str = "\x1b[9m";
    /// Set inverse/reverse video
    pub const SGR_INVERSE: &str = "\x1b[7m";

    /// Set underline color (SGR 58;2;r;g;b) -- requires terminal support.
    pub fn underline_color(buf: &mut Vec<u8>, r: u8, g: u8, b: u8) {
        use std::io::Write;
        let _ = write!(buf, "\x1b[58;2;{};{};{}m", r, g, b);
    }

    /// Build a complete SGR sequence from cell attributes, writing into `buf`.
    /// Returns the number of bytes appended.
    pub fn write_sgr(buf: &mut Vec<u8>, attrs: &CellAttrs) {
        // Always start with reset to avoid attribute leaking
        buf.extend_from_slice(SGR_RESET.as_bytes());

        if attrs.bold {
            buf.extend_from_slice(SGR_BOLD.as_bytes());
        }
        if attrs.italic {
            buf.extend_from_slice(SGR_ITALIC.as_bytes());
        }
        match attrs.underline {
            1 => buf.extend_from_slice(SGR_UNDERLINE.as_bytes()),
            2 => buf.extend_from_slice(SGR_CURLY_UNDERLINE.as_bytes()),
            3 => buf.extend_from_slice(SGR_DOUBLE_UNDERLINE.as_bytes()),
            4 => buf.extend_from_slice(SGR_DOTTED_UNDERLINE.as_bytes()),
            5 => buf.extend_from_slice(SGR_DASHED_UNDERLINE.as_bytes()),
            _ => {}
        }
        if attrs.strikethrough {
            buf.extend_from_slice(SGR_STRIKETHROUGH.as_bytes());
        }
        if attrs.inverse {
            buf.extend_from_slice(SGR_INVERSE.as_bytes());
        }

        // Foreground color (24-bit) — skip if terminal default
        if let Some((r, g, b)) = attrs.fg {
            fg_truecolor(buf, r, g, b);
        }

        // Background color (24-bit) — skip if terminal default
        if let Some((r, g, b)) = attrs.bg {
            bg_truecolor(buf, r, g, b);
        }

        // Underline color if different from fg
        if let Some((r, g, b)) = attrs.underline_color {
            underline_color(buf, r, g, b);
        }
    }

    /// Cell attributes that map to SGR sequences.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct CellAttrs {
        /// Foreground color as (R, G, B).  None = terminal default.
        pub fg: Option<(u8, u8, u8)>,
        /// Background color as (R, G, B).  None = terminal default.
        pub bg: Option<(u8, u8, u8)>,
        /// Bold
        pub bold: bool,
        /// Italic
        pub italic: bool,
        /// Underline style (0=none, 1=single, 2=wave, 3=double, 4=dotted, 5=dashed)
        pub underline: u8,
        /// Underline color override (None = use fg)
        pub underline_color: Option<(u8, u8, u8)>,
        /// Strikethrough
        pub strikethrough: bool,
        /// Inverse video
        pub inverse: bool,
    }

    /// Terminal cursor shapes expressible via DECSCUSR.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TerminalCursorShape {
        Block,
        Underline,
        Bar,
    }
}

// ---------------------------------------------------------------------------
// TTY Cell and Grid for frame diffing
// ---------------------------------------------------------------------------

/// A single cell in the TTY grid used for frame diffing.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TtyCell {
    /// Character content (may be multi-byte for wide/composed chars)
    text: String,
    /// Display width of this cell (1 for normal, 2 for wide chars)
    width: u8,
    /// Cell attributes
    attrs: ansi::CellAttrs,
}

impl Default for TtyCell {
    fn default() -> Self {
        Self {
            text: " ".to_string(),
            width: 1,
            attrs: ansi::CellAttrs::default(),
        }
    }
}

/// TTY cell grid: two-dimensional array of cells for frame diffing.
#[derive(Debug, Clone)]
struct TtyGrid {
    width: usize,
    height: usize,
    cells: Vec<TtyCell>,
}

impl TtyGrid {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![TtyCell::default(); width * height],
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.cells.resize(width * height, TtyCell::default());
    }

    fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = TtyCell::default();
        }
    }

    #[allow(dead_code)] // exercised by the TtyGrid unit tests
    fn get(&self, col: usize, row: usize) -> Option<&TtyCell> {
        if col < self.width && row < self.height {
            Some(&self.cells[row * self.width + col])
        } else {
            None
        }
    }

    fn get_mut(&mut self, col: usize, row: usize) -> Option<&mut TtyCell> {
        if col < self.width && row < self.height {
            Some(&mut self.cells[row * self.width + col])
        } else {
            None
        }
    }

    fn set(&mut self, col: usize, row: usize, cell: TtyCell) {
        if col < self.width && row < self.height {
            self.cells[row * self.width + col] = cell;
        }
    }
}

// ---------------------------------------------------------------------------
// Color conversion
// ---------------------------------------------------------------------------

/// Convert a `Color` (f32 0.0-1.0 components) to (R, G, B) in 0-255.
/// The Color may be in linear or sRGB space; for TTY output we want sRGB
/// bytes, so we clamp to [0,1] and multiply by 255.
fn color_to_rgb8(c: &Color) -> (u8, u8, u8) {
    let r = (c.r.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    (r, g, b)
}

/// Query terminal size via crossterm (cross-platform).
fn get_terminal_size() -> Option<(u16, u16)> {
    crossterm::terminal::size().ok()
}

// ---------------------------------------------------------------------------
// Frame diffing
// ---------------------------------------------------------------------------

/// Compute the minimal set of terminal writes needed to transition from
/// `prev` to `next`. Returns the escape sequence bytes to write.
///
/// The algorithm walks every cell. When a cell differs, it emits:
/// 1. A cursor-goto if the cursor isn't already at the right position
/// 2. SGR attribute changes if the attributes differ from the last emitted
/// 3. The cell text
///
/// Consecutive changed cells on the same row avoid redundant cursor-goto
/// sequences (the cursor naturally advances after printing).
fn diff_grids(prev: &TtyGrid, next: &TtyGrid) -> Vec<u8> {
    assert_eq!(prev.width, next.width);
    assert_eq!(prev.height, next.height);

    let mut out = Vec::with_capacity(4096);
    let mut last_attrs: Option<ansi::CellAttrs> = None;
    let mut cursor_row: Option<usize> = None;
    let mut cursor_col: Option<usize> = None;

    for row in 0..next.height {
        let mut col = 0;
        while col < next.width {
            let next_cell = &next.cells[row * next.width + col];
            let prev_cell = &prev.cells[row * prev.width + col];

            if next_cell != prev_cell {
                // Need to position cursor if not already here
                let need_goto = cursor_row != Some(row) || cursor_col != Some(col);
                if need_goto {
                    // ANSI uses 1-based coordinates
                    ansi::cursor_goto(&mut out, (row + 1) as u16, (col + 1) as u16);
                }

                // Emit SGR if attrs changed from last emitted
                if last_attrs.as_ref() != Some(&next_cell.attrs) {
                    ansi::write_sgr(&mut out, &next_cell.attrs);
                    last_attrs = Some(next_cell.attrs);
                }

                // Emit the character text
                out.extend_from_slice(next_cell.text.as_bytes());

                // Advance cursor tracking
                let advance = next_cell.width.max(1) as usize;
                cursor_row = Some(row);
                cursor_col = Some(col + advance);

                col += advance;
            } else {
                col += next_cell.width.max(1) as usize;
            }
        }
    }

    // Reset attributes at the end to avoid leaking into the shell
    if last_attrs.is_some() {
        out.extend_from_slice(ansi::SGR_RESET.as_bytes());
    }

    out
}

/// Full-screen render: emit every cell (used for first frame or after resize).
fn render_full(grid: &TtyGrid) -> Vec<u8> {
    let mut out = Vec::with_capacity(grid.width * grid.height * 20);

    // Home cursor
    out.extend_from_slice(ansi::CURSOR_HOME.as_bytes());

    let mut last_attrs: Option<ansi::CellAttrs> = None;

    for row in 0..grid.height {
        if row > 0 {
            // Move to start of next row
            ansi::cursor_goto(&mut out, (row + 1) as u16, 1);
        }
        let mut col = 0;
        while col < grid.width {
            let cell = &grid.cells[row * grid.width + col];

            if last_attrs.as_ref() != Some(&cell.attrs) {
                ansi::write_sgr(&mut out, &cell.attrs);
                last_attrs = Some(cell.attrs);
            }

            out.extend_from_slice(cell.text.as_bytes());
            col += cell.width.max(1) as usize;
        }
    }

    if last_attrs.is_some() {
        out.extend_from_slice(ansi::SGR_RESET.as_bytes());
    }

    out
}

// ---------------------------------------------------------------------------
// FrameGlyphBuffer -> TtyGrid rasterizer
// ---------------------------------------------------------------------------

/// Rasterize a `FrameGlyphBuffer` into a `TtyGrid`.
///
/// This maps pixel-coordinate glyphs into character-cell positions using
/// the frame's `char_width` and `char_height` as the cell dimensions.
fn rasterize_frame_glyphs(frame: &FrameGlyphBuffer, grid: &mut TtyGrid, _bg_color: (u8, u8, u8)) {
    // Clear grid — blank cells use terminal default colors (no SGR bg/fg)
    for cell in &mut grid.cells {
        cell.text = " ".to_string();
        cell.width = 1;
        cell.attrs = ansi::CellAttrs {
            ..Default::default()
        };
    }

    let cw = frame.char_width.max(1.0);
    let ch = frame.char_height.max(1.0);
    for glyph in &frame.glyphs {
        match glyph {
            FrameGlyph::Char {
                char: character,
                composed,
                x,
                y,
                face_id,
                ..
            } => {
                let col = (*x / cw) as usize;
                let row = (*y / ch) as usize;

                if col >= grid.width || row >= grid.height {
                    continue;
                }

                let text = if let Some(comp) = composed {
                    comp.to_string()
                } else {
                    character.to_string()
                };

                // Resolve the per-cell colors and decorations from the face
                // table; these used to be inlined on the glyph.
                let rf = frame.resolved_face(*face_id);
                let fg = rf.fg;
                let bg: Option<Color> = Some(rf.bg);
                let font_weight = rf.font_weight;
                let italic = rf.italic;
                let underline = rf.underline;
                let underline_color = rf.underline_color;
                let strike_through = rf.strike_through;

                let fg_rgb = color_to_rgb8(&fg);
                let bg_rgb = bg.map(|c| color_to_rgb8(&c));
                let ul_color = underline_color.map(|c| {
                    let (r, g, b) = color_to_rgb8(&c);
                    (r, g, b)
                });

                let cell = TtyCell {
                    text,
                    width: 1,
                    attrs: ansi::CellAttrs {
                        fg: Some(fg_rgb),
                        bg: bg_rgb,
                        bold: font_weight >= 700,
                        italic,
                        underline: underline.gnu_code(),
                        underline_color: ul_color,
                        strikethrough: strike_through,
                        inverse: false,
                    },
                };

                // Determine display width (approximate: use glyph pixel
                // width relative to char_width)
                let display_width = ((*x + frame.char_width - 0.5).max(0.0) / cw) as usize;
                let glyph_cols = ((glyph_pixel_width(glyph) / cw) + 0.5) as usize;
                let w = glyph_cols.clamp(1, 2) as u8;

                grid.set(col, row, TtyCell { width: w, ..cell });

                // For wide chars, mark the continuation cell
                if w == 2 && col + 1 < grid.width {
                    grid.set(
                        col + 1,
                        row,
                        TtyCell {
                            text: String::new(),
                            width: 0, // continuation cell
                            attrs: ansi::CellAttrs {
                                fg: Some(fg_rgb),
                                bg: bg_rgb,
                                ..Default::default()
                            },
                        },
                    );
                }
                let _ = display_width; // suppress unused warning
            }

            FrameGlyph::Stretch {
                x,
                y,
                width,
                height,
                bg,
                ..
            } => {
                let col_start = (*x / cw) as usize;
                let row_start = (*y / ch) as usize;
                let col_end = ((*x + *width) / cw).ceil() as usize;
                let row_end = ((*y + *height) / ch).ceil() as usize;

                let bg_rgb = color_to_rgb8(bg);

                for row in row_start..row_end.min(grid.height) {
                    for col in col_start..col_end.min(grid.width) {
                        if let Some(cell) = grid.get_mut(col, row) {
                            cell.attrs.bg = Some(bg_rgb);
                        }
                    }
                }
            }

            FrameGlyph::Background { bounds, color } => {
                let col_start = (bounds.x / cw) as usize;
                let row_start = (bounds.y / ch) as usize;
                let col_end = ((bounds.x + bounds.width) / cw).ceil() as usize;
                let row_end = ((bounds.y + bounds.height) / ch).ceil() as usize;

                let bg_rgb = color_to_rgb8(color);

                for row in row_start..row_end.min(grid.height) {
                    for col in col_start..col_end.min(grid.width) {
                        if let Some(cell) = grid.get_mut(col, row) {
                            cell.attrs.bg = Some(bg_rgb);
                        }
                    }
                }
            }

            FrameGlyph::Border {
                x,
                y,
                width,
                height,
                color,
                ..
            } => {
                let col_start = (*x / cw) as usize;
                let row_start = (*y / ch) as usize;
                let col_end = ((*x + *width) / cw).ceil() as usize;
                let row_end = ((*y + *height) / ch).ceil() as usize;

                let border_rgb = color_to_rgb8(color);

                // For vertical borders (width <= 1 cell), use box-drawing char
                let is_vertical = (col_end - col_start) <= 1;
                let is_horizontal = (row_end - row_start) <= 1;

                for row in row_start..row_end.min(grid.height) {
                    for col in col_start..col_end.min(grid.width) {
                        if let Some(cell) = grid.get_mut(col, row) {
                            cell.attrs.fg = Some(border_rgb);
                            if is_vertical {
                                cell.text = "\u{2502}".to_string(); // │
                            } else if is_horizontal {
                                cell.text = "\u{2500}".to_string(); // ─
                            } else {
                                cell.text = "\u{2588}".to_string(); // █
                            }
                        }
                    }
                }
            }

            // Non-text glyphs are not rendered in TTY mode. Fringe bitmaps are a
            // GUI-only feature (the TTY has no fringe pixel column).
            FrameGlyph::Image { .. }
            | FrameGlyph::Video { .. }
            | FrameGlyph::Xwidget { .. }
            | FrameGlyph::Surface { .. }
            | FrameGlyph::FringeBitmap { .. }
            | FrameGlyph::ScrollBar { .. } => {}

            #[cfg(feature = "neo-term")]
            FrameGlyph::Terminal { .. } => {}
        }
    }

    // One entry per window (selected window's entry is `active`); apply each.
    for cursor in &frame.window_cursors {
        apply_tty_window_cursor(grid, frame, cursor);
    }
}

/// Get the pixel width of a glyph.
fn glyph_pixel_width(glyph: &FrameGlyph) -> f32 {
    match glyph {
        FrameGlyph::Char { width, .. } => *width,
        FrameGlyph::Stretch { width, .. } => *width,
        FrameGlyph::Image { width, .. } => *width,
        FrameGlyph::Video { width, .. } => *width,
        FrameGlyph::Xwidget { width, .. } => *width,
        FrameGlyph::Surface { width, .. } => *width,
        FrameGlyph::FringeBitmap { width, .. } => *width,
        FrameGlyph::Background { bounds, .. } => bounds.width,
        FrameGlyph::Border { width, .. } => *width,
        FrameGlyph::ScrollBar { width, .. } => *width,
        #[cfg(feature = "neo-term")]
        FrameGlyph::Terminal { width, .. } => *width,
    }
}

fn apply_tty_window_cursor(grid: &mut TtyGrid, frame: &FrameGlyphBuffer, cursor: &WindowCursor) {
    let cw = frame.char_width.max(1.0);
    let ch = frame.char_height.max(1.0);
    let col = (cursor.x / cw) as usize;
    let row = (cursor.y / ch) as usize;

    if col >= grid.width || row >= grid.height {
        return;
    }

    let cursor_rgb = color_to_rgb8(&cursor.color);
    let cursor_fg_rgb = color_to_rgb8(&cursor.cursor_fg);

    match cursor.style {
        CursorStyle::FilledBox => {
            if let Some(cell) = grid.get_mut(col, row) {
                cell.attrs.fg = Some(cursor_fg_rgb);
                cell.attrs.bg = Some(cursor_rgb);
            }
        }
        CursorStyle::Bar(_) => {
            if let Some(cell) = grid.get_mut(col, row) {
                cell.attrs.inverse = true;
            }
        }
        CursorStyle::Hbar(_) => {
            if let Some(cell) = grid.get_mut(col, row) {
                cell.attrs.underline = 1;
            }
        }
        CursorStyle::Hollow => {
            if let Some(cell) = grid.get_mut(col, row) {
                cell.attrs.inverse = true;
            }
        }
    }
}

fn terminal_cursor_state(
    frame: &FrameGlyphBuffer,
) -> Option<((u16, u16), bool, Option<ansi::TerminalCursorShape>)> {
    let cw = frame.char_width.max(1.0);
    let ch = frame.char_height.max(1.0);

    frame.active_cursor().map(|cursor| {
        let col = (cursor.x / cw) as u16;
        let row = (cursor.y / ch) as u16;
        let (visible, shape) = match cursor.style {
            CursorStyle::Bar(_) => (true, Some(ansi::TerminalCursorShape::Bar)),
            CursorStyle::Hbar(_) => (true, Some(ansi::TerminalCursorShape::Underline)),
            CursorStyle::FilledBox | CursorStyle::Hollow => (false, None),
        };
        ((col, row), visible, shape)
    })
}

// ---------------------------------------------------------------------------
// TtyBackend
// ---------------------------------------------------------------------------

/// TTY backend state
pub struct TtyBackend {
    initialized: bool,
    width: u32,
    height: u32,

    /// Current frame grid (what should be on screen)
    current: TtyGrid,
    /// Previous frame grid (what is on screen)
    previous: TtyGrid,

    /// Whether next render should do a full repaint
    force_full_render: bool,

    /// Buffered output bytes to write on present()
    output_buf: Vec<u8>,

    /// True when crossterm raw mode is active (so we can restore on shutdown)
    raw_mode_enabled: bool,

    /// Cursor position to set after rendering (col, row) -- 0-based
    cursor_position: Option<(u16, u16)>,
    /// Whether to show the terminal cursor
    cursor_visible: bool,
    /// Hardware cursor shape when the terminal cursor is visible.
    cursor_shape: ansi::TerminalCursorShape,

    /// Last received FrameGlyphBuffer for rendering
    frame_glyphs: Option<FrameGlyphBuffer>,
    /// Child frames to composite on top of the main frame.
    child_frames: Vec<FrameGlyphBuffer>,
}

impl Default for TtyBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TtyBackend {
    pub fn new() -> Self {
        Self {
            initialized: false,
            width: 80,
            height: 24,
            current: TtyGrid::new(80, 24),
            previous: TtyGrid::new(80, 24),
            force_full_render: true,
            output_buf: Vec::with_capacity(65536),
            raw_mode_enabled: false,
            cursor_position: None,
            cursor_visible: false,
            cursor_shape: ansi::TerminalCursorShape::Block,
            frame_glyphs: None,
            child_frames: Vec::new(),
        }
    }

    /// Set child frames to composite on top of the main frame.
    pub fn set_child_frames(&mut self, frames: Vec<FrameGlyphBuffer>) {
        self.child_frames = frames;
    }

    /// Set a FrameGlyphBuffer to be rendered on the next render() call.
    pub fn set_frame_glyphs(&mut self, frame: FrameGlyphBuffer) {
        self.frame_glyphs = Some(frame);
    }

    /// Get the current grid dimensions in characters.
    pub fn grid_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Force a full repaint on the next render.
    pub fn force_redraw(&mut self) {
        self.force_full_render = true;
    }

    /// Internal: build output bytes from current vs previous grid.
    fn build_output(&mut self) {
        self.output_buf.clear();

        if self.force_full_render {
            self.output_buf = render_full(&self.current);
            self.force_full_render = false;
        } else {
            self.output_buf = diff_grids(&self.previous, &self.current);
        }

        // Position cursor and show/hide
        if let Some((col, row)) = self.cursor_position {
            ansi::cursor_goto(&mut self.output_buf, row + 1, col + 1);
            if self.cursor_visible {
                ansi::cursor_shape(&mut self.output_buf, self.cursor_shape);
                self.output_buf
                    .extend_from_slice(ansi::SHOW_CURSOR.as_bytes());
            } else {
                self.output_buf
                    .extend_from_slice(ansi::HIDE_CURSOR.as_bytes());
            }
        } else {
            self.output_buf
                .extend_from_slice(ansi::HIDE_CURSOR.as_bytes());
        }
    }
}

impl TtyBackend {
    pub fn init(&mut self) -> DisplayResult<()> {
        // Get terminal size
        if let Some((cols, rows)) = get_terminal_size() {
            self.width = cols as u32;
            self.height = rows as u32;
        }

        // Resize grids
        self.current
            .resize(self.width as usize, self.height as usize);
        self.previous
            .resize(self.width as usize, self.height as usize);

        // Enter raw mode (cross-platform via crossterm)
        match crossterm::terminal::enable_raw_mode() {
            Ok(()) => self.raw_mode_enabled = true,
            Err(e) => {
                return Err(DisplayError::Backend(format!(
                    "Failed to enable raw mode: {}",
                    e
                )));
            }
        }

        // Enter alternate screen, hide cursor, clear screen
        let mut stdout = io::stdout();
        let init_seq = format!(
            "{}{}{}",
            ansi::ENTER_ALT_SCREEN,
            ansi::HIDE_CURSOR,
            ansi::CLEAR_SCREEN,
        );
        stdout
            .write_all(init_seq.as_bytes())
            .map_err(|e| DisplayError::Backend(format!("Failed to write init sequence: {}", e)))?;
        stdout
            .flush()
            .map_err(|e| DisplayError::Backend(format!("Failed to flush stdout: {}", e)))?;

        self.force_full_render = true;
        self.initialized = true;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        if !self.initialized {
            return;
        }

        // Show cursor, leave alternate screen, reset attributes
        let shutdown_seq = format!(
            "{}{}{}{}",
            ansi::SGR_RESET,
            "\x1b[2 q",
            ansi::SHOW_CURSOR,
            ansi::LEAVE_ALT_SCREEN,
        );

        let mut stdout = io::stdout();
        let _ = stdout.write_all(shutdown_seq.as_bytes());
        let _ = stdout.flush();

        // Restore terminal state
        if self.raw_mode_enabled {
            let _ = crossterm::terminal::disable_raw_mode();
            self.raw_mode_enabled = false;
        }

        self.initialized = false;
    }

    pub fn render(&mut self, scene: &Scene) -> DisplayResult<()> {
        if !self.initialized {
            return Err(DisplayError::Backend("TTY backend not initialized".into()));
        }

        // Save previous grid for diffing
        self.previous = self.current.clone();

        // If we have a FrameGlyphBuffer, rasterize it
        if let Some(ref frame) = self.frame_glyphs {
            let bg_rgb = color_to_rgb8(&frame.background);
            rasterize_frame_glyphs(frame, &mut self.current, bg_rgb);

            self.cursor_position = None;
            self.cursor_visible = false;
            self.cursor_shape = ansi::TerminalCursorShape::Block;
            if let Some(((col, row), visible, shape)) = terminal_cursor_state(frame) {
                self.cursor_position = Some((col, row));
                self.cursor_visible = visible;
                if let Some(shape) = shape {
                    self.cursor_shape = shape;
                }
            }
        } else {
            let bg_rgb = color_to_rgb8(&scene.background);
            self.current.clear();
            for cell in &mut self.current.cells {
                cell.attrs.bg = Some(bg_rgb);
            }
        }

        // Build diff output
        self.build_output();

        Ok(())
    }

    pub fn present(&mut self) -> DisplayResult<()> {
        if !self.initialized {
            return Err(DisplayError::Backend("TTY backend not initialized".into()));
        }

        if self.output_buf.is_empty() {
            return Ok(());
        }

        let mut stdout = io::stdout();
        stdout
            .write_all(&self.output_buf)
            .map_err(|e| DisplayError::Backend(format!("Failed to write to stdout: {}", e)))?;
        stdout
            .flush()
            .map_err(|e| DisplayError::Backend(format!("Failed to flush stdout: {}", e)))?;

        self.output_buf.clear();

        Ok(())
    }

    pub fn name(&self) -> &'static str {
        "tty"
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.current.resize(width as usize, height as usize);
        self.previous.resize(width as usize, height as usize);
        self.force_full_render = true;
    }

    pub fn set_vsync(&mut self, _enabled: bool) {
        // No vsync on TTY
    }
}

impl Drop for TtyBackend {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[path = "tty_test.rs"]
mod tests;
