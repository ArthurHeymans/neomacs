//! Types for the Rust layout engine.
//!
//! These are the intermediate representation between buffer data and rendering.
//! The layout engine produces display rows and frame display state snapshots
//! that renderers consume without owning display semantics.

use neomacs_display_protocol::cursor::CursorBarWidth;
use neomacs_display_protocol::types::{Color, Rect};

/// Complete layout output for one frame.
/// Produced by the layout engine, consumed by the renderer.
#[derive(Debug, Clone)]
pub struct LayoutOutput {
    /// Frame dimensions in pixels
    pub width: f32,
    pub height: f32,

    /// Frame background color (sRGB, will be converted to linear)
    pub background: Color,

    /// Default character cell dimensions
    pub char_width: f32,
    pub char_height: f32,
    pub font_pixel_size: f32,

    /// Laid-out windows
    pub windows: Vec<WindowLayout>,
}

/// Layout output for a single window.
#[derive(Debug, Clone)]
pub struct WindowLayout {
    /// Window identifier (pointer cast to i64)
    pub window_id: i64,
    /// Buffer identifier (pointer cast to u64)
    pub buffer_id: u64,
    /// Frame-absolute bounds of this window
    pub bounds: Rect,
    /// Whether this is the selected (active) window
    pub selected: bool,
    /// First visible buffer position
    pub window_start: i64,
    /// Mode-line height in pixels
    pub mode_line_height: f32,

    /// Laid-out rows (visual lines)
    pub rows: Vec<LayoutRow>,

    /// Cursor position (if visible in this window)
    pub cursor: Option<CursorLayout>,

    /// Last visible buffer position (for window-end feedback)
    pub window_end_pos: i64,
}

/// A single laid-out visual line.
#[derive(Debug, Clone)]
pub struct LayoutRow {
    /// Glyphs on this row
    pub glyphs: Vec<LayoutGlyph>,
    /// Frame-absolute Y position
    pub y: f32,
    /// Row height in pixels
    pub height: f32,
    /// Font ascent for this row
    pub ascent: f32,
    /// Whether this is a mode-line/header-line/tab-line row
    pub is_mode_line: bool,
}

/// A single laid-out glyph.
#[derive(Debug, Clone)]
pub enum LayoutGlyph {
    /// Character glyph
    Char {
        /// The character
        ch: char,
        /// Frame-absolute X position
        x: f32,
        /// Pixel width
        width: f32,
        /// Face ID
        face_id: u32,
        /// Buffer position this glyph represents
        charpos: i64,
    },

    /// Stretch (whitespace) glyph
    Stretch {
        /// Frame-absolute X position
        x: f32,
        /// Pixel width
        width: f32,
        /// Face ID
        face_id: u32,
    },

    /// Image glyph
    Image {
        /// GPU image ID
        image_id: u32,
        /// Frame-absolute X position
        x: f32,
        /// Pixel width
        width: f32,
        /// Pixel height
        height: f32,
    },
}

/// Cursor layout information.
#[derive(Debug, Clone)]
pub struct CursorLayout {
    /// Frame-absolute X position
    pub x: f32,
    /// Frame-absolute Y position
    pub y: f32,
    /// Width in pixels
    pub width: f32,
    /// Height in pixels
    pub height: f32,
    /// Cursor style: 0=box, 1=bar, 2=hbar, 3=hollow
    pub style: u8,
    /// Cursor color
    pub color: Color,
    /// Character under cursor (for inverse video with filled box)
    pub char_under: Option<char>,
    /// Face ID of character under cursor
    pub char_face_id: Option<u32>,
}

/// Parameters for a window that the layout engine needs.
/// Populated from Emacs data via FFI before layout runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LayoutCharPos0(i64);

impl LayoutCharPos0 {
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

/// How the layout engine should handle text that exceeds the right edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineWrapMode {
    /// Lines that exceed the window width are truncated with a continuation
    /// glyph (e.g. `$`).
    #[default]
    Truncate,
    /// Lines that exceed the window width wrap to the next visual row.
    Wrap,
}

/// Semantic category of a window within its frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowKind {
    /// A normal editing window.
    #[default]
    Main,
    /// The minibuffer window.
    Minibuffer,
}

impl WindowKind {
    pub const fn is_minibuffer(self) -> bool {
        matches!(self, Self::Minibuffer)
    }
}

#[derive(Debug, Clone)]
pub struct WindowParams {
    /// Window identifier (pointer value)
    pub window_id: i64,
    /// Buffer identifier (pointer value)
    pub buffer_id: u64,
    /// Frame-absolute bounds
    pub bounds: Rect,
    /// Text area bounds (excludes fringes, margins, scroll bars)
    pub text_bounds: Rect,
    /// Whether this is the selected window
    pub selected: bool,
    /// What kind of window this is (main editing area or minibuffer).
    pub kind: WindowKind,

    /// First visible buffer position in layout 0-based char coordinates.
    /// Derived from GNU `marker_position (w->start)`.
    pub window_start: i64,
    /// Last visible buffer position from previous frame in layout 0-based
    /// char coordinates.  0 means unknown for this legacy raw field.
    pub window_end: i64,
    /// Point position in this window's buffer in layout 0-based char coordinates.
    pub point: i64,
    /// Accessible end (ZV) in layout 0-based exclusive char coordinates.
    pub buffer_size: i64,
    /// Accessible start (BEGV) in layout 0-based char coordinates.
    pub buffer_begv: i64,

    /// Horizontal scroll offset in columns
    pub hscroll: i32,
    /// Vertical scroll offset in pixels (shifts content up)
    pub vscroll: i32,

    /// How to handle long lines.
    pub wrap_mode: LineWrapMode,
    /// Whether to wrap at word boundaries
    pub word_wrap: bool,
    /// Tab width in columns
    pub tab_width: i32,
    /// Custom tab stop positions (column numbers), from tab-stop-list buffer-local.
    /// Empty means use fixed-width tab_width stops only.
    pub tab_stop_list: Vec<i32>,

    /// Default face foreground/background for this window
    pub default_fg: u32,
    pub default_bg: u32,

    /// Character cell dimensions
    pub char_width: f32,
    pub char_height: f32,
    /// Whether the containing frame is a real window-system frame.
    pub window_system: bool,
    /// Font pixel size
    pub font_pixel_size: f32,
    /// Font ascent
    pub font_ascent: f32,

    /// Mode-line height (0 if no mode-line)
    pub mode_line_height: f32,
    /// Header-line height (0 if no header-line)
    pub header_line_height: f32,
    /// Tab-line height (0 if no tab-line)
    pub tab_line_height: f32,

    /// Cursor kind for this window. The discriminant matches GNU's
    /// `enum text_cursor_kinds` exactly (`FilledBox=0`, `HollowBox=1`,
    /// `Bar=2`, `Hbar=3`, `NoCursor=-1`, `Default=-2`). Cursor audit
    /// Finding 1 in `drafts/cursor-audit.md` flagged the previous
    /// `u8` encoding as a silently re-numbered alias.
    pub cursor_kind: neomacs_display_protocol::frame_glyphs::CursorKind,
    /// Cursor bar width (for bar cursor)
    pub cursor_bar_width: CursorBarWidth,
    /// Whether stretch slots may use their full displayed width for box cursors.
    ///
    /// Mirrors GNU `x-stretch-cursor`.
    pub x_stretch_cursor: bool,
    /// Cursor color in sRGB pixel format.
    pub cursor_color: u32,
    /// Neomacs-specific cursor effect profile for this window.
    pub cursor_effects: Option<neomacs_display_protocol::effect_config::EffectsConfig>,
    /// Neomacs-only visual cursors for this window's buffer.
    ///
    /// These do not affect GNU point, mark, selection, command dispatch, or
    /// IME state. They are render-only cursor visuals anchored to buffer
    /// positions.
    pub visual_cursors: Vec<VisualCursorSpec>,

    /// Fringe widths in pixels
    pub left_fringe_width: f32,
    pub right_fringe_width: f32,
    /// indicate-empty-lines: 0=off, 1=left, 2=right
    pub indicate_empty_lines: i32,
    /// Whether to show trailing whitespace
    pub show_trailing_whitespace: bool,
    /// Trailing-whitespace face background color
    pub trailing_ws_bg: u32,
    /// Fill-column-indicator column (-1 = off, nonnegative = column)
    pub fill_column_indicator: i32,
    /// Fill-column-indicator character
    pub fill_column_indicator_char: char,
    /// Fill-column-indicator face foreground color
    pub fill_column_indicator_fg: u32,
    /// Extra line spacing in pixels
    pub extra_line_spacing: f32,
    /// selective-display: 0=off, >0=hide lines indented more than N columns
    pub selective_display: i32,
    /// escape-glyph face foreground color
    pub escape_glyph_fg: u32,
    /// nobreak-char-display: 0=off, 1=highlight, 2=escape notation
    pub nobreak_char_display: i32,
    /// nobreak-char face foreground color
    pub nobreak_char_fg: u32,
    /// glyphless-char face foreground color
    pub glyphless_char_fg: u32,
    /// wrap-prefix: bytes rendered at start of continuation lines
    pub wrap_prefix: Vec<u8>,
    /// line-prefix: bytes rendered at start of all visual lines
    pub line_prefix: Vec<u8>,
    /// Left margin width in pixels (0 = no margin)
    pub left_margin_width: f32,
    /// Right margin width in pixels (0 = no margin)
    pub right_margin_width: f32,

    // --- Scroll bar configuration ---
    /// Effective vertical scroll bar side for this window.
    /// `None` when disabled, `Some("left")` or `Some("right")` when enabled.
    pub vertical_scroll_bar_side: Option<String>,
    /// Whether a horizontal scroll bar is shown below the text area.
    pub horizontal_scroll_bar: bool,
    /// Vertical scroll bar track width in pixels (0 when disabled).
    pub scroll_bar_pixel_width: f32,
    /// Horizontal scroll bar track height in pixels (0 when disabled).
    pub scroll_bar_pixel_height: f32,
}

impl WindowParams {
    pub const fn is_minibuffer(&self) -> bool {
        self.kind.is_minibuffer()
    }

    pub fn window_start_charpos(&self) -> LayoutCharPos0 {
        LayoutCharPos0::new(self.window_start)
    }

    pub fn previous_window_end_charpos(&self) -> Option<LayoutCharPos0> {
        (self.window_end > 0).then(|| LayoutCharPos0::new(self.window_end))
    }

    pub fn point_charpos(&self) -> LayoutCharPos0 {
        LayoutCharPos0::new(self.point)
    }

    pub fn accessible_start_charpos(&self) -> LayoutCharPos0 {
        LayoutCharPos0::new(self.buffer_begv)
    }

    pub fn accessible_end_charpos(&self) -> LayoutCharPos0 {
        LayoutCharPos0::new(self.buffer_size)
    }
}

#[derive(Clone, Debug)]
pub struct VisualCursorSpec {
    pub id: i32,
    /// 0-based buffer character position.
    pub charpos: i64,
    pub cursor_kind: neomacs_display_protocol::frame_glyphs::CursorKind,
    pub cursor_bar_width: CursorBarWidth,
    pub color: u32,
    pub effects: Option<neomacs_display_protocol::effect_config::EffectsConfig>,
}

/// Frame-level parameters for layout.
#[derive(Debug, Clone)]
pub struct FrameParams {
    /// Frame pixel dimensions
    pub width: f32,
    pub height: f32,
    /// Frame-level menu-bar height in pixels.
    ///
    /// Mirrors GNU `FRAME_MENU_BAR_HEIGHT (f)`. On TTY frames this is
    /// `menu-bar-lines * char_height` (with `char_height = 1`); the
    /// layout engine reserves this many pixels at the top of the frame
    /// for the menu bar row, mirroring `display_menu_bar()` in xdisp.c.
    pub menu_bar_height: f32,
    /// Frame-level tool-bar height in pixels.
    pub tool_bar_height: f32,
    /// Frame-level compact-bar height in pixels.
    pub compact_bar_height: f32,
    /// Frame-level tab-bar height in pixels.
    pub tab_bar_height: f32,
    /// Default character cell dimensions
    pub char_width: f32,
    pub char_height: f32,
    /// Font pixel size
    pub font_pixel_size: f32,
    /// Whether this frame is backed by a window-system display.
    pub window_system: bool,
    /// Frame background color (sRGB pixel)
    pub background: u32,
    /// Vertical border face foreground color (sRGB pixel)
    pub vertical_border_fg: u32,
    /// Right window divider width in pixels (0 = disabled)
    pub right_divider_width: i32,
    /// Bottom window divider width in pixels (0 = disabled)
    pub bottom_divider_width: i32,
    /// Window-divider face foreground color (sRGB pixel)
    pub divider_fg: u32,
    /// Window-divider-first-pixel face foreground color (sRGB pixel)
    pub divider_first_fg: u32,
    /// Window-divider-last-pixel face foreground color (sRGB pixel)
    pub divider_last_fg: u32,
}

#[cfg(test)]
#[path = "types_test.rs"]
mod tests;
