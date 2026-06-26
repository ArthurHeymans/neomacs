//! GNU Emacs-compatible glyph matrix types for the shared display path.
//!
//! These types match the architecture of GNU Emacs's `dispextern.h`:
//! `struct glyph`, `struct glyph_row`, `struct glyph_matrix`.
//!
//! The glyph matrix is character-grid native for terminal output, but also
//! carries each glyph's realized pixel width.  GNU's `struct glyph` stores
//! `pixel_width`; GUI backends must use that rather than reconstructing every
//! glyph as one frame column.

use super::effect_config::EffectsConfig;
use super::face::{Face, FaceAttributes};
use super::frame_glyphs::{
    CursorStyle, DisplaySlotId, FrameGlyph, FrameGlyphBuffer, FrameTabBarState, FringeBitmapData,
    FringeSide, GlyphRowRole, MaterializedFaceData, PhysCursor, StipplePattern, WindowCursor,
    WindowEffectHint, WindowInfo, WindowTransitionHint,
};
use super::types::{Color, DisplayFrameId, DisplayWindowId, ImageId, Rect, VideoId, XwidgetId};
use super::ui_types::{MenuBarItem, ToolBarItem};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::collections::HashMap;

/// What kind of content this glyph represents.
/// Matches GNU's `enum glyph_type` in `dispextern.h`.
#[derive(Clone, Debug, PartialEq)]
pub enum GlyphType {
    /// Regular character (including multibyte).
    Char { ch: char },
    /// Composed grapheme cluster (ligatures, emoji ZWJ, combining marks).
    Composite { text: Box<str> },
    /// Whitespace/filler — occupies `width_cols` character cells.
    Stretch { width_cols: u16 },
    /// Inline image referenced by ID.
    Image { image_id: i32 },
    /// Character with no available glyph (rendered as hex code or thin-space).
    Glyphless { ch: char },
}

/// GNU `enum glyph_type` discriminants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum GlyphTypeKind {
    Char = 0,
    Composite = 1,
    Glyphless = 2,
    Image = 3,
    Stretch = 4,
    Xwidget = 5,
}

impl GlyphTypeKind {
    pub fn from_gnu_code(code: u8) -> Option<Self> {
        Self::try_from(code).ok()
    }

    pub fn gnu_code(self) -> u8 {
        self.into()
    }
}

impl GlyphType {
    pub fn gnu_kind(&self) -> GlyphTypeKind {
        match self {
            GlyphType::Char { .. } => GlyphTypeKind::Char,
            GlyphType::Composite { .. } => GlyphTypeKind::Composite,
            GlyphType::Glyphless { .. } => GlyphTypeKind::Glyphless,
            GlyphType::Image { .. } => GlyphTypeKind::Image,
            GlyphType::Stretch { .. } => GlyphTypeKind::Stretch,
        }
    }
}

/// Three areas within a glyph row, matching GNU's `enum glyph_row_area`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum GlyphArea {
    LeftMargin = 0,
    Text = 1,
    RightMargin = 2,
}

impl GlyphArea {
    pub fn index(self) -> usize {
        usize::from(u8::from(self))
    }

    pub fn from_gnu_code(code: u8) -> Option<Self> {
        Self::try_from(code).ok()
    }

    pub fn gnu_code(self) -> u8 {
        self.into()
    }
}

/// Sentinel `charpos` for synthetic glyphs that map to no buffer position.
///
/// Glyphs appended by `extend_face_to_end_of_line` (the leading face-anchor
/// space on an empty row and the trailing background stretch) fill the
/// highlighted `:extend` background past end-of-line but cover no buffer
/// character. They carry this sentinel so cursor placement can exclude them,
/// mirroring GNU's `NILP (glyph->object)` test in `set_cursor_from_row`
/// (src/xdisp.c). A literal `0` cannot be used: real buffer text begins at
/// 0-based `charpos` `0`, so `0` is a valid position for the first glyph.
pub const NO_BUFFER_POSITION_CHARPOS: usize = usize::MAX;

/// One character cell on screen.
/// Equivalent to GNU's `struct glyph` in `dispextern.h`.
///
/// Grid-native: no pixel coordinates. Screen position is determined by
/// the row index in `GlyphRow` and position within the area's glyph vector.
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    /// What this glyph displays.
    pub glyph_type: GlyphType,
    /// Face ID for looking up colors, font, and decoration.
    pub face_id: u32,
    /// Buffer position this glyph maps to (for cursor placement, mouse clicks).
    pub charpos: usize,
    /// Bidirectional resolved level (0 = LTR base, 1 = RTL, etc.).
    pub bidi_level: u8,
    /// True for double-width characters (CJK, etc.).
    pub wide: bool,
    /// Realized glyph advance in pixels.
    ///
    /// `0.0` means "not explicitly measured"; materialization falls back to
    /// character-grid width.  TTY backends ignore this field.
    pub pixel_width: f32,
    /// Stretch-glyph height in pixels.
    ///
    /// GNU's `struct glyph` stores stretch height/ascent in
    /// `glyph->u.stretch`.  `0.0` means "use the containing row height".
    pub pixel_height: f32,
    /// Stretch-glyph ascent in pixels.
    ///
    /// Used with `pixel_height`; materialization positions the stretch
    /// relative to the row baseline.  `0.0` falls back to row ascent.
    pub pixel_ascent: f32,
    /// Glyph vertical offset in pixels.
    ///
    /// Mirrors GNU `struct glyph::voffset`: negative values raise the
    /// glyph, positive values lower it.
    pub vertical_offset_px: f32,
    /// Padding glyph — second cell of a wide character.
    pub padding: bool,
}

impl Glyph {
    /// Create a simple character glyph with default attributes.
    pub fn char(ch: char, face_id: u32, charpos: usize) -> Self {
        Self {
            glyph_type: GlyphType::Char { ch },
            face_id,
            charpos,
            bidi_level: 0,
            wide: false,
            pixel_width: 0.0,
            pixel_height: 0.0,
            pixel_ascent: 0.0,
            vertical_offset_px: 0.0,
            padding: false,
        }
    }

    /// Create a stretch (whitespace) glyph.
    pub fn stretch(width_cols: u16, face_id: u32) -> Self {
        Self {
            glyph_type: GlyphType::Stretch { width_cols },
            face_id,
            charpos: 0,
            bidi_level: 0,
            wide: false,
            pixel_width: 0.0,
            pixel_height: 0.0,
            pixel_ascent: 0.0,
            vertical_offset_px: 0.0,
            padding: false,
        }
    }

    /// Create a padding glyph (second cell of a wide character).
    pub fn padding_for(face_id: u32, charpos: usize) -> Self {
        Self {
            glyph_type: GlyphType::Char { ch: ' ' },
            face_id,
            charpos,
            bidi_level: 0,
            wide: false,
            pixel_width: 0.0,
            pixel_height: 0.0,
            pixel_ascent: 0.0,
            vertical_offset_px: 0.0,
            padding: true,
        }
    }

    /// Return a copy with explicit GUI pixel advance.
    pub fn with_pixel_width(mut self, pixel_width: f32) -> Self {
        self.pixel_width = if pixel_width.is_finite() && pixel_width > 0.0 {
            pixel_width
        } else {
            0.0
        };
        self
    }

    /// Return a copy with explicit GUI stretch geometry.
    pub fn with_pixel_geometry(
        mut self,
        pixel_width: f32,
        pixel_height: f32,
        pixel_ascent: f32,
    ) -> Self {
        self.pixel_width = if pixel_width.is_finite() && pixel_width > 0.0 {
            pixel_width
        } else {
            0.0
        };
        self.pixel_height = if pixel_height.is_finite() && pixel_height > 0.0 {
            pixel_height
        } else {
            0.0
        };
        self.pixel_ascent = if self.pixel_height > 0.0 && pixel_ascent.is_finite() {
            pixel_ascent.max(0.0).min(self.pixel_height)
        } else {
            0.0
        };
        self
    }

    pub fn with_vertical_offset(mut self, vertical_offset_px: f32) -> Self {
        self.vertical_offset_px = if vertical_offset_px.is_finite() {
            vertical_offset_px
        } else {
            0.0
        };
        self
    }
}

/// One screen row. Equivalent to GNU's `struct glyph_row`.
///
/// Contains three glyph areas (left margin, text, right margin) matching
/// GNU's layout. Row hashing enables fast diff: if hashes match, the rows
/// are likely identical; if they differ, the row needs redrawing.
#[derive(Clone, Debug)]
pub struct GlyphRow {
    /// Glyphs per area: [left_margin, text, right_margin].
    pub glyphs: [Vec<Glyph>; 3],
    /// Row hash for fast diff. 0 = not yet computed.
    pub hash: u64,
    /// Row is valid and should be displayed.
    pub enabled: bool,
    /// Semantic role: text body, mode-line, header-line, tab-line, etc.
    pub role: GlyphRowRole,
    /// Cursor column in this row, if cursor is here.
    pub cursor_col: Option<u16>,
    /// Cursor type when cursor is in this row.
    pub cursor_type: Option<CursorStyle>,
    /// Row has been truncated on the left.
    pub truncated_left: bool,
    /// Row has a continuation mark on the right.
    pub continued: bool,
    /// Row's paragraph base direction is right-to-left. GNU `reversed_p`: such
    /// rows are displayed flush to the right margin, with the empty space to
    /// the left of the leftmost glyph filled by the background. Row
    /// materialization offsets the glyphs to the right edge accordingly.
    pub reversed_p: bool,
    /// Row displays actual buffer text (not blank filler).
    pub displays_text: bool,
    /// Row ends at end of buffer.
    pub ends_at_zv: bool,
    /// This is a mode-line, header-line, or tab-line row.
    pub mode_line: bool,
    /// Row top relative to the containing window's origin.
    ///
    /// Mirrors GNU `struct glyph_row::y`. `height_px == 0.0` means
    /// the row still relies on legacy implicit grid placement.
    pub pixel_y: f32,
    /// Authoritative row height in pixels.
    ///
    /// Mirrors GNU `struct glyph_row::height`. `0.0` means unset.
    pub height_px: f32,
    /// Authoritative baseline ascent from row top in pixels.
    ///
    /// Mirrors GNU `struct glyph_row::ascent`. `0.0` means unset.
    pub ascent_px: f32,
    /// Buffer position at start of this row.
    pub start_charpos: usize,
    /// Buffer position at end of this row.
    pub end_charpos: usize,
    /// Fringe bitmap to draw in this row's LEFT fringe, if any. GNU records the
    /// per-row fringe bitmap on `struct glyph_row::left_fringe_bitmap`.
    pub left_fringe_bitmap: Option<FringeBitmapInfo>,
    /// Fringe bitmap to draw in this row's RIGHT fringe, if any. Reserved for
    /// the right-fringe path (not yet emitted downstream).
    pub right_fringe_bitmap: Option<FringeBitmapInfo>,
}

/// Per-row fringe-bitmap reference: the resolved registry index and the face id
/// used for its foreground/background colors. The actual bits live once per
/// frame in `FrameGlyphBuffer::fringe_bitmaps`, keyed by `bitmap_index`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FringeBitmapInfo {
    pub bitmap_index: u16,
    pub face_id: u32,
}

impl GlyphRow {
    pub fn new(role: GlyphRowRole) -> Self {
        Self {
            glyphs: [Vec::new(), Vec::new(), Vec::new()],
            hash: 0,
            enabled: true,
            role,
            cursor_col: None,
            cursor_type: None,
            truncated_left: false,
            continued: false,
            reversed_p: false,
            displays_text: false,
            ends_at_zv: false,
            mode_line: false,
            pixel_y: 0.0,
            height_px: 0.0,
            ascent_px: 0.0,
            start_charpos: 0,
            end_charpos: 0,
            left_fringe_bitmap: None,
            right_fringe_bitmap: None,
        }
    }

    /// Compute FNV-1a hash over all glyph areas.
    /// Returns 0 for empty rows (sentinel meaning "not computed").
    pub fn compute_hash(&self) -> u64 {
        let total: usize = self.glyphs.iter().map(|a| a.len()).sum();
        if total == 0 {
            return 0;
        }

        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for area in &self.glyphs {
            for glyph in area {
                let ch_val = match &glyph.glyph_type {
                    GlyphType::Char { ch } => *ch as u64,
                    GlyphType::Composite { text } => {
                        let mut h = 0u64;
                        for b in text.bytes() {
                            h = h.wrapping_mul(31).wrapping_add(b as u64);
                        }
                        h
                    }
                    GlyphType::Stretch { width_cols } => 0x8000_0000 | (*width_cols as u64),
                    GlyphType::Image { image_id } => 0x4000_0000 | (*image_id as u64),
                    GlyphType::Glyphless { ch } => 0x2000_0000 | (*ch as u64),
                };
                hash ^= ch_val;
                hash = hash.wrapping_mul(FNV_PRIME);
                hash ^= glyph.face_id as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
                hash ^= glyph.pixel_width.to_bits() as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
                hash ^= glyph.pixel_height.to_bits() as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
                hash ^= glyph.pixel_ascent.to_bits() as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
                hash ^= glyph.vertical_offset_px.to_bits() as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        // Right-to-left rows are aligned differently, so a direction flip on
        // otherwise-identical glyphs must still count as a change.
        hash ^= self.reversed_p as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash
    }

    pub fn row_equal(&self, other: &GlyphRow) -> bool {
        if self.hash != 0 && other.hash != 0 && self.hash != other.hash {
            return false;
        }
        if self.reversed_p != other.reversed_p {
            return false;
        }
        for i in 0..3 {
            if self.glyphs[i].len() != other.glyphs[i].len() {
                return false;
            }
            for (a, b) in self.glyphs[i].iter().zip(other.glyphs[i].iter()) {
                if a != b {
                    return false;
                }
            }
        }
        true
    }

    pub fn used(&self, area: GlyphArea) -> usize {
        self.glyphs[area.index()].len()
    }

    pub fn total_glyphs(&self) -> usize {
        self.glyphs[0].len() + self.glyphs[1].len() + self.glyphs[2].len()
    }

    pub fn clear(&mut self) {
        for area in &mut self.glyphs {
            area.clear();
        }
        self.hash = 0;
        self.cursor_col = None;
        self.cursor_type = None;
        self.truncated_left = false;
        self.continued = false;
        self.reversed_p = false;
        self.displays_text = false;
        self.ends_at_zv = false;
        self.pixel_y = 0.0;
        self.height_px = 0.0;
        self.ascent_px = 0.0;
        self.start_charpos = 0;
        self.end_charpos = 0;
    }
}

#[derive(Clone, Debug)]
pub struct GlyphMatrix {
    pub rows: Vec<GlyphRow>,
    pub nrows: usize,
    pub ncols: usize,
    pub matrix_x: usize,
    pub matrix_y: usize,
    pub header_line: bool,
    pub tab_line: bool,
}

impl GlyphMatrix {
    pub fn new(nrows: usize, ncols: usize) -> Self {
        // Matrix rows start disabled. `begin_row` and
        // `begin_status_line_row` flip `enabled = true` for rows
        // that are actually populated during a frame. Rows the
        // walker skips (below-the-text scratch rows, unused
        // slots) stay disabled so `overwrite_last_window_right_border`
        // and `TtyRif::rasterize` know not to touch them. Matches
        // GNU's `MATRIX_ROW_ENABLED_P` discipline where disabled
        // rows are inert until the walker marks them valid.
        let rows = (0..nrows)
            .map(|_| {
                let mut row = GlyphRow::new(GlyphRowRole::Text);
                row.enabled = false;
                row
            })
            .collect();
        Self {
            rows,
            nrows,
            ncols,
            matrix_x: 0,
            matrix_y: 0,
            header_line: false,
            tab_line: false,
        }
    }

    pub fn clear(&mut self) {
        for row in &mut self.rows {
            row.clear();
        }
    }

    pub fn resize(&mut self, nrows: usize, ncols: usize) {
        self.rows.resize_with(nrows, || {
            let mut row = GlyphRow::new(GlyphRowRole::Text);
            row.enabled = false;
            row
        });
        self.rows.truncate(nrows);
        self.nrows = nrows;
        self.ncols = ncols;
    }

    pub fn ensure_hashes(&mut self) {
        for row in &mut self.rows {
            if row.hash == 0 && row.total_glyphs() > 0 {
                row.hash = row.compute_hash();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct WindowMatrixEntry {
    pub window_id: u64,
    pub matrix: GlyphMatrix,
    /// Frame-relative bounds of the whole Emacs window area owned by
    /// this matrix, including margins/fringes and chrome rows.
    pub pixel_bounds: Rect,
    /// Frame-relative bounds of the GNU TEXT_AREA inside this window.
    ///
    /// Buffer text glyphs and the physical cursor are laid out in
    /// text-area-local coordinates; materialization applies this
    /// origin when converting them to frame pixels.  Header/mode-line
    /// rows remain window-wide and continue to use `pixel_bounds`.
    pub text_pixel_bounds: Rect,
    /// True when this window is the frame's selected window at the
    /// time the display state was built. The TTY rasterizer uses
    /// this to decide which window owns the physical terminal
    /// cursor: only the selected window contributes a
    /// `cursor_col` to the terminal cursor position, even though
    /// other windows may still draw a hollow cursor glyph via
    /// `cursor-in-non-selected-windows`. Mirrors GNU
    /// `src/xdisp.c::display_and_set_cursor`, which only resolves
    /// the frame cursor from the selected window's row.
    pub selected: bool,
}

/// Frame-level chrome row that is not owned by any leaf window.
///
/// GNU treats the tab bar specially:
/// - GUI builds a dedicated `tab_bar_window`
/// - TTY writes tab-bar rows directly into the frame matrix
///
/// Neomacs keeps immutable published display state, so we model the
/// frame-level tab bar explicitly here instead of smuggling it into the first
/// leaf window's matrix after layout.
#[derive(Clone, Debug)]
pub struct FrameChromeRow {
    /// Visual row number in frame matrix space.
    pub row_index: u32,
    /// Frame-relative clip rect / origin for this row.
    pub pixel_bounds: Rect,
    /// Row contents and row metrics.
    pub row: GlyphRow,
}

// ---------------------------------------------------------------------------
// Non-grid item structs — these mirror FrameGlyph variants for items that
// don't belong on the character grid (backgrounds, borders, cursors, etc.).
// ---------------------------------------------------------------------------

/// A window background rectangle.
#[derive(Clone, Debug)]
pub struct BackgroundItem {
    pub bounds: Rect,
    pub color: Color,
}

/// A window border/divider rectangle.
#[derive(Clone, Debug)]
pub struct BorderItem {
    pub window_id: DisplayWindowId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
}

/// A cursor entry.
#[derive(Clone, Debug)]
pub struct CursorItem {
    pub window_id: DisplayWindowId,
    pub slot_id: DisplaySlotId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub style: CursorStyle,
    pub color: Color,
}

/// An inline image.
#[derive(Clone, Debug)]
pub struct ImageItem {
    pub window_id: DisplayWindowId,
    pub row_role: GlyphRowRole,
    pub clip_rect: Option<Rect>,
    pub slot_id: Option<DisplaySlotId>,
    pub image_id: ImageId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// An inline video.
#[derive(Clone, Debug)]
pub struct VideoItem {
    pub window_id: DisplayWindowId,
    pub row_role: GlyphRowRole,
    pub clip_rect: Option<Rect>,
    pub slot_id: Option<DisplaySlotId>,
    pub video_id: VideoId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub loop_count: i32,
    pub autoplay: bool,
}

/// An inline xwidget.
#[derive(Clone, Debug)]
pub struct XwidgetItem {
    pub window_id: DisplayWindowId,
    pub row_role: GlyphRowRole,
    pub clip_rect: Option<Rect>,
    pub slot_id: Option<DisplaySlotId>,
    pub xwidget_id: XwidgetId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A scroll bar.
#[derive(Clone, Debug)]
pub struct ScrollBarItem {
    pub window_id: DisplayWindowId,
    pub row_role: GlyphRowRole,
    pub clip_rect: Option<Rect>,
    pub horizontal: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub position: i64,
    pub portion: i64,
    pub whole: i64,
    pub thumb_start: f32,
    pub thumb_size: f32,
    pub track_color: Color,
    pub thumb_color: Color,
}

#[derive(Clone, Debug)]
pub struct FrameDisplayState {
    pub window_matrices: Vec<WindowMatrixEntry>,
    /// Frame-level chrome rows that are not owned by any leaf window.
    pub frame_chrome_rows: Vec<FrameChromeRow>,
    pub frame_cols: usize,
    pub frame_rows: usize,
    pub frame_pixel_width: f32,
    pub frame_pixel_height: f32,
    pub char_width: f32,
    pub char_height: f32,
    pub font_pixel_size: f32,
    pub background: Color,
    pub faces: HashMap<u32, Face>,
    pub frame_id: DisplayFrameId,
    pub parent_id: DisplayFrameId,
    pub parent_x: f32,
    pub parent_y: f32,
    pub z_order: i32,
    pub undecorated: bool,
    pub border_width: f32,
    pub border_color: Color,
    pub background_alpha: f32,
    pub no_accept_focus: bool,
    pub window_infos: Vec<WindowInfo>,
    pub transition_hints: Vec<WindowTransitionHint>,
    /// Window background rectangles.
    pub backgrounds: Vec<BackgroundItem>,
    /// Window border/divider rectangles.
    pub borders: Vec<BorderItem>,
    /// Cursor entries.
    pub cursors: Vec<CursorItem>,
    /// Per-window cursor effect profiles.
    pub cursor_effects_by_window: HashMap<DisplayWindowId, EffectsConfig>,
    /// Inline images (non-grid, pixel-positioned).
    pub images: Vec<ImageItem>,
    /// Inline videos.
    pub videos: Vec<VideoItem>,
    /// Inline xwidgets.
    pub xwidgets: Vec<XwidgetItem>,
    /// Scroll bars.
    pub scroll_bars: Vec<ScrollBarItem>,
    /// Authoritative active cursor for the frame.
    pub phys_cursor: Option<PhysCursor>,
    /// Stipple patterns for background fills.
    pub stipple_patterns: HashMap<i32, StipplePattern>,
    /// Effect hints for the renderer.
    pub effect_hints: Vec<WindowEffectHint>,
    /// Frame-level menu bar (TTY only, GNU `display_menu_bar` analog).
    ///
    /// `Some` when `(frame-parameter nil 'menu-bar-lines)` > 0 on a TTY
    /// frame.  Carries the items collected by walking the active
    /// `[menu-bar]` keymaps (mirroring GNU `keyboard.c:menu_bar_items`).
    /// The TTY rasterizer paints these into row 0; the GUI runtime has
    /// its own menu-bar machinery and ignores this field.
    pub menu_bar: Option<TtyMenuBarState>,
    /// GUI menu-bar overlay state for the primary frame snapshot.
    pub gui_menu_bar: Option<GuiMenuBarState>,
    /// GUI tool-bar overlay state for the primary frame snapshot.
    pub gui_tool_bar: Option<GuiToolBarState>,
    /// GUI compact-bar overlay state for the primary frame snapshot.
    pub gui_compact_bar: Option<GuiCompactBarState>,
    /// Frame-level tab bar metadata for render-thread hit-testing.
    pub tab_bar: Option<FrameTabBarState>,
    /// Resolved fringe bitmaps for this frame, keyed by registry index. Each
    /// `GlyphRow::left_fringe_bitmap` references one of these by `bitmap_index`.
    pub fringe_bitmaps: HashMap<u16, FringeBitmapData>,
}

/// One label entry in the TTY menu bar (one top-level menu like "File").
///
/// Mirrors the per-item slot of GNU's `f->menu_bar_items` vector
/// (4 slots per item: key, string, def, hpos — see
/// `keyboard.c:menu_bar_items`). We only need the visible label and the
/// key (for future activation hit-testing); `def` is intentionally not
/// carried across the display-protocol boundary because the renderer
/// doesn't need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtyMenuBarItem {
    /// Display label, e.g. `"File"`.
    pub label: String,
    /// Key symbol the user invokes to open this menu, e.g. `file`.
    pub key: String,
    /// 0-based screen column where this item starts (filled by layout).
    pub hpos: u16,
}

/// Per-frame menu bar state passed from layout to the TTY rasterizer.
///
/// Carries the resolved attributes of the GNU `menu` face directly so
/// the rasterizer doesn't have to look anything up in the per-glyph
/// `faces` HashMap (which is populated dynamically as text glyphs are
/// emitted, and may not contain a `menu` entry at all when the menu
/// bar holds the only references to that face).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TtyMenuBarState {
    /// Items in display order, left-to-right.
    pub items: Vec<TtyMenuBarItem>,
    /// Number of menu-bar lines (today: always 1 on TTY, mirrors
    /// `FRAME_MENU_BAR_LINES` in GNU).
    pub lines: u16,
    /// Foreground RGB pixel (`0x00RRGGBB`) for the `menu` face.
    pub fg: u32,
    /// Background RGB pixel (`0x00RRGGBB`) for the `menu` face.
    pub bg: u32,
    /// Whether the terminal default foreground should be used.
    pub use_default_foreground: bool,
    /// Whether the terminal default background should be used.
    pub use_default_background: bool,
    /// Bold attribute from the `menu` face.
    pub bold: bool,
    /// Inverse-video attribute from the `menu` face.
    pub inverse: bool,
}

/// GUI menu-bar overlay state carried in a frame snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct GuiMenuBarState {
    pub items: Vec<MenuBarItem>,
    pub height: f32,
    pub fg: (f32, f32, f32),
    pub bg: (f32, f32, f32),
}

/// GUI tool-bar overlay state carried in a frame snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct GuiToolBarState {
    pub items: Vec<ToolBarItem>,
    pub height: f32,
    pub fg: (f32, f32, f32),
    pub bg: (f32, f32, f32),
}

/// GUI compact-bar overlay state carried in a frame snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct GuiCompactBarState {
    pub menu_items: Vec<MenuBarItem>,
    pub tool_items: Vec<ToolBarItem>,
    pub height: f32,
    pub menu_fg: (f32, f32, f32),
    pub menu_bg: (f32, f32, f32),
    pub tool_fg: (f32, f32, f32),
    pub tool_bg: (f32, f32, f32),
}

impl FrameDisplayState {
    pub fn new(frame_cols: usize, frame_rows: usize, char_width: f32, char_height: f32) -> Self {
        Self {
            window_matrices: Vec::new(),
            frame_chrome_rows: Vec::new(),
            frame_cols,
            frame_rows,
            frame_pixel_width: frame_cols as f32 * char_width,
            frame_pixel_height: frame_rows as f32 * char_height,
            char_width,
            char_height,
            font_pixel_size: char_height,
            background: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            faces: HashMap::new(),
            frame_id: DisplayFrameId::new(0),
            parent_id: DisplayFrameId::new(0),
            parent_x: 0.0,
            parent_y: 0.0,
            z_order: 0,
            undecorated: false,
            border_width: 0.0,
            border_color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            background_alpha: 1.0,
            no_accept_focus: false,
            window_infos: Vec::new(),
            transition_hints: Vec::new(),
            backgrounds: Vec::new(),
            borders: Vec::new(),
            cursors: Vec::new(),
            cursor_effects_by_window: HashMap::new(),
            images: Vec::new(),
            videos: Vec::new(),
            xwidgets: Vec::new(),
            scroll_bars: Vec::new(),
            phys_cursor: None,
            stipple_patterns: HashMap::new(),
            effect_hints: Vec::new(),
            menu_bar: None,
            gui_menu_bar: None,
            gui_tool_bar: None,
            gui_compact_bar: None,
            tab_bar: None,
            fringe_bitmaps: HashMap::new(),
        }
    }

    /// Create a `FrameDisplayState` from an existing `FrameGlyphBuffer`.
    ///
    /// Decomposes the flat glyph list into structured non-grid item
    /// vectors (backgrounds, borders, cursors, images, videos, xwidgets,
    /// scroll bars) and copies metadata (faces, window_infos, hints).
    pub fn from_frame_glyph_buffer(buf: &FrameGlyphBuffer) -> Self {
        let frame_cols = (buf.width / buf.char_width.max(1.0)) as usize;
        let frame_rows = (buf.height / buf.char_height.max(1.0)) as usize;
        let mut state = Self::new(frame_cols, frame_rows, buf.char_width, buf.char_height);
        state.frame_pixel_width = buf.width;
        state.frame_pixel_height = buf.height;
        state.font_pixel_size = buf.font_pixel_size;
        state.background = buf.background;
        state.frame_id = buf.frame_id;
        state.parent_id = buf.parent_id;
        state.parent_x = buf.parent_x;
        state.parent_y = buf.parent_y;
        state.z_order = buf.z_order;
        state.undecorated = buf.undecorated;
        state.border_width = buf.border_width;
        state.border_color = buf.border_color;
        state.background_alpha = buf.background_alpha;
        state.no_accept_focus = buf.no_accept_focus;
        state.faces = buf.faces.clone();
        state.window_infos = buf.window_infos.clone();
        // Reconstruct the layout-internal phys_cursor from the unified list's
        // active entry; charpos isn't carried on WindowCursor so default to 0.
        state.phys_cursor = buf.active_cursor().map(|c| PhysCursor {
            window_id: c.window_id,
            charpos: 0,
            row: c.slot_id.row as usize,
            col: c.slot_id.col,
            slot_id: c.slot_id,
            x: c.x,
            y: c.y,
            width: c.width,
            height: c.height,
            ascent: c.ascent,
            style: c.style,
            color: c.color,
            cursor_fg: c.cursor_fg,
        });
        state.cursor_effects_by_window = buf.cursor_effects_by_window.clone();
        state.stipple_patterns = buf.stipple_patterns.clone();
        state.fringe_bitmaps = buf.fringe_bitmaps.clone();
        state.transition_hints = buf.transition_hints.clone();
        state.effect_hints = buf.effect_hints.clone();
        state.tab_bar = buf.tab_bar.clone();
        // Only non-active (decorative) cursors round-trip into `cursors`; the
        // active entry is reconstructed into `phys_cursor` above.
        state.cursors.extend(
            buf.window_cursors
                .iter()
                .filter(|cursor| !cursor.active)
                .map(|cursor| CursorItem {
                    window_id: cursor.window_id,
                    slot_id: cursor.slot_id,
                    x: cursor.x,
                    y: cursor.y,
                    width: cursor.width,
                    height: cursor.height,
                    style: cursor.style,
                    color: cursor.color,
                }),
        );

        // Decompose glyphs into structured non-grid item vectors
        for glyph in &buf.glyphs {
            match glyph {
                FrameGlyph::Background { bounds, color } => {
                    state.backgrounds.push(BackgroundItem {
                        bounds: *bounds,
                        color: *color,
                    });
                }
                FrameGlyph::Border {
                    window_id,
                    x,
                    y,
                    width,
                    height,
                    color,
                    ..
                } => {
                    state.borders.push(BorderItem {
                        window_id: *window_id,
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                        color: *color,
                    });
                }
                FrameGlyph::Image {
                    window_id,
                    row_role,
                    clip_rect,
                    slot_id,
                    image_id,
                    x,
                    y,
                    width,
                    height,
                } => {
                    state.images.push(ImageItem {
                        window_id: *window_id,
                        row_role: *row_role,
                        clip_rect: *clip_rect,
                        slot_id: *slot_id,
                        image_id: *image_id,
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                    });
                }
                FrameGlyph::Video {
                    window_id,
                    row_role,
                    clip_rect,
                    slot_id,
                    video_id,
                    x,
                    y,
                    width,
                    height,
                    loop_count,
                    autoplay,
                } => {
                    state.videos.push(VideoItem {
                        window_id: *window_id,
                        row_role: *row_role,
                        clip_rect: *clip_rect,
                        slot_id: *slot_id,
                        video_id: *video_id,
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                        loop_count: *loop_count,
                        autoplay: *autoplay,
                    });
                }
                FrameGlyph::Xwidget {
                    window_id,
                    row_role,
                    clip_rect,
                    slot_id,
                    xwidget_id,
                    x,
                    y,
                    width,
                    height,
                } => {
                    state.xwidgets.push(XwidgetItem {
                        window_id: *window_id,
                        row_role: *row_role,
                        clip_rect: *clip_rect,
                        slot_id: *slot_id,
                        xwidget_id: *xwidget_id,
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                    });
                }
                FrameGlyph::ScrollBar {
                    window_id,
                    row_role,
                    clip_rect,
                    horizontal,
                    x,
                    y,
                    width,
                    height,
                    position,
                    portion,
                    whole,
                    thumb_start,
                    thumb_size,
                    track_color,
                    thumb_color,
                } => {
                    state.scroll_bars.push(ScrollBarItem {
                        window_id: *window_id,
                        row_role: *row_role,
                        clip_rect: *clip_rect,
                        horizontal: *horizontal,
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                        position: *position,
                        portion: *portion,
                        whole: *whole,
                        thumb_start: *thumb_start,
                        thumb_size: *thumb_size,
                        track_color: *track_color,
                        thumb_color: *thumb_color,
                    });
                }
                // Char, Stretch, Terminal — grid content, not decomposed here
                _ => {}
            }
        }

        state
    }

    /// Convert this `FrameDisplayState` into a `FrameGlyphBuffer`.
    ///
    /// Materializes the `GlyphMatrix` grid into pixel-positioned
    /// `FrameGlyph` entries and appends all non-grid items (backgrounds,
    /// borders, cursors, etc.).
    pub fn materialize(&self) -> FrameGlyphBuffer {
        let mut buf = FrameGlyphBuffer::with_size(self.frame_pixel_width, self.frame_pixel_height);
        buf.char_width = self.char_width;
        buf.char_height = self.char_height;
        buf.font_pixel_size = self.font_pixel_size;
        buf.background = self.background;
        buf.frame_id = self.frame_id;
        buf.parent_id = self.parent_id;
        buf.parent_x = self.parent_x;
        buf.parent_y = self.parent_y;
        buf.z_order = self.z_order;
        buf.undecorated = self.undecorated;
        buf.border_width = self.border_width;
        buf.border_color = self.border_color;
        buf.background_alpha = self.background_alpha;
        buf.no_accept_focus = self.no_accept_focus;

        // Copy faces
        for (id, face) in &self.faces {
            buf.faces.insert(*id, face.clone());
        }

        // Copy window_infos
        for info in &self.window_infos {
            buf.window_infos.push(info.clone());
        }

        // Copy stipple patterns
        buf.stipple_patterns = self.stipple_patterns.clone();

        // Copy fringe bitmaps (the bits referenced by each row's fringe info).
        buf.fringe_bitmaps = self.fringe_bitmaps.clone();

        // --- Grid conversion ---

        // Copy effect hints
        buf.effect_hints = self.effect_hints.clone();

        // Copy transition hints
        buf.transition_hints = self.transition_hints.clone();
        buf.tab_bar = self.tab_bar.clone();

        // --- Materialize all glyphs (backgrounds, grid, borders, images,
        // videos, xwidgets, scroll bars) in the canonical order ---
        self.for_each_glyph(|g| buf.glyphs.push(g));

        // --- Materialize cursors ---
        // These are non-selected (decorative) cursors; CursorItem has no
        // cursor_fg/ascent. The selected window's active cursor is pushed by
        // set_phys_cursor below. These write to `buf.window_cursors`, not
        // `buf.glyphs`, so the glyph order produced above is preserved.
        for cursor in &self.cursors {
            buf.window_cursors.push(WindowCursor {
                window_id: cursor.window_id,
                slot_id: cursor.slot_id,
                x: cursor.x,
                y: cursor.y,
                width: cursor.width,
                height: cursor.height,
                style: cursor.style,
                color: cursor.color,
                cursor_fg: Color::BLACK,
                ascent: 0.0,
                active: false,
            });
        }
        buf.cursor_effects_by_window = self.cursor_effects_by_window.clone();

        if let Some(cursor) = self.phys_cursor.clone() {
            buf.set_phys_cursor(cursor);
        }

        buf
    }

    /// Visit every `FrameGlyph` this state materializes, in the canonical
    /// `materialize()` order, calling `push` for each.
    ///
    /// This is the glyph-production half of [`Self::materialize`], factored out
    /// so callers can iterate the matrix directly without building the flat
    /// `Vec<FrameGlyph>`. It emits, in order: backgrounds, frame-chrome grid
    /// rows, window-matrix grid rows, borders, images, videos, xwidgets, and
    /// scroll bars. It does NOT emit cursors or write any `FrameGlyphBuffer`
    /// metadata.
    pub fn for_each_glyph(&self, mut push: impl FnMut(FrameGlyph)) {
        // --- Materialize backgrounds ---
        for bg in &self.backgrounds {
            push(FrameGlyph::Background {
                bounds: bg.bounds,
                color: bg.color,
            });
        }

        // --- Materialize grid content -> pixel-positioned Char/Stretch glyphs ---
        for frame_row in &self.frame_chrome_rows {
            self.for_each_grid_row_glyph(
                DisplayWindowId::new(0),
                frame_row.row_index,
                &frame_row.row,
                frame_row.pixel_bounds,
                self.char_width,
                self.char_height,
                &mut push,
            );
        }
        for entry in &self.window_matrices {
            for (row_idx, glyph_row) in entry.matrix.rows.iter().enumerate() {
                let row_bounds = entry.row_pixel_bounds(glyph_row.role);
                let char_w = if entry.matrix.ncols > 0 {
                    row_bounds.width / entry.matrix.ncols as f32
                } else {
                    self.char_width
                };
                self.for_each_grid_row_glyph(
                    DisplayWindowId::new(entry.window_id as i64),
                    row_idx as u32,
                    glyph_row,
                    row_bounds,
                    char_w,
                    self.char_height,
                    &mut push,
                );
            }
        }

        // --- Materialize left/right fringe bitmaps ---
        //
        // Only buffer-text rows carry fringe bitmaps (magit section headings).
        // The left fringe column spans from the window's left edge
        // (`pixel_bounds.x`) up to the text area (`text_pixel_bounds.x`); for the
        // magit-first scope (no left margin) this is exactly the fringe. The
        // right fringe path is parsed but not emitted yet.
        for entry in &self.window_matrices {
            let window_id = DisplayWindowId::new(entry.window_id as i64);
            for (row_idx, glyph_row) in entry.matrix.rows.iter().enumerate() {
                if !glyph_row.enabled {
                    continue;
                }
                if glyph_row.left_fringe_bitmap.is_none() && glyph_row.right_fringe_bitmap.is_none()
                {
                    continue;
                }
                let row_bounds = entry.row_pixel_bounds(glyph_row.role);
                let y = if glyph_row.height_px > 0.0 {
                    row_bounds.y + glyph_row.pixel_y
                } else {
                    row_bounds.y + row_idx as f32 * self.char_height
                };
                let height = if glyph_row.height_px > 0.0 {
                    glyph_row.height_px
                } else {
                    self.char_height
                };
                let clip_rect = Some(entry.pixel_bounds);

                if let Some(info) = glyph_row.left_fringe_bitmap {
                    let x = entry.pixel_bounds.x;
                    let width = (entry.text_pixel_bounds.x - entry.pixel_bounds.x).max(0.0);
                    push(FrameGlyph::FringeBitmap {
                        window_id,
                        row_role: glyph_row.role,
                        clip_rect,
                        x,
                        y,
                        width,
                        height,
                        bitmap_index: info.bitmap_index,
                        face_id: info.face_id,
                        side: FringeSide::Left,
                    });
                }
                if let Some(info) = glyph_row.right_fringe_bitmap {
                    let text_right = entry.text_pixel_bounds.x + entry.text_pixel_bounds.width;
                    let window_right = entry.pixel_bounds.x + entry.pixel_bounds.width;
                    let x = text_right;
                    let width = (window_right - text_right).max(0.0);
                    push(FrameGlyph::FringeBitmap {
                        window_id,
                        row_role: glyph_row.role,
                        clip_rect,
                        x,
                        y,
                        width,
                        height,
                        bitmap_index: info.bitmap_index,
                        face_id: info.face_id,
                        side: FringeSide::Right,
                    });
                }
            }
        }

        // --- Materialize borders ---
        for border in &self.borders {
            push(FrameGlyph::Border {
                window_id: border.window_id,
                row_role: GlyphRowRole::Text,
                clip_rect: None,
                x: border.x,
                y: border.y,
                width: border.width,
                height: border.height,
                color: border.color,
            });
        }

        // --- Materialize standalone images ---
        for img in &self.images {
            push(FrameGlyph::Image {
                window_id: img.window_id,
                row_role: img.row_role,
                clip_rect: img.clip_rect,
                slot_id: img.slot_id,
                image_id: img.image_id,
                x: img.x,
                y: img.y,
                width: img.width,
                height: img.height,
            });
        }

        // --- Materialize videos ---
        for vid in &self.videos {
            push(FrameGlyph::Video {
                window_id: vid.window_id,
                row_role: vid.row_role,
                clip_rect: vid.clip_rect,
                slot_id: vid.slot_id,
                video_id: vid.video_id,
                x: vid.x,
                y: vid.y,
                width: vid.width,
                height: vid.height,
                loop_count: vid.loop_count,
                autoplay: vid.autoplay,
            });
        }

        // --- Materialize xwidgets ---
        for xwidget in &self.xwidgets {
            push(FrameGlyph::Xwidget {
                window_id: xwidget.window_id,
                row_role: xwidget.row_role,
                clip_rect: xwidget.clip_rect,
                slot_id: xwidget.slot_id,
                xwidget_id: xwidget.xwidget_id,
                x: xwidget.x,
                y: xwidget.y,
                width: xwidget.width,
                height: xwidget.height,
            });
        }

        // --- Materialize scroll bars ---
        for sb in &self.scroll_bars {
            push(FrameGlyph::ScrollBar {
                window_id: sb.window_id,
                row_role: sb.row_role,
                clip_rect: sb.clip_rect,
                horizontal: sb.horizontal,
                x: sb.x,
                y: sb.y,
                width: sb.width,
                height: sb.height,
                position: sb.position,
                portion: sb.portion,
                whole: sb.whole,
                thumb_start: sb.thumb_start,
                thumb_size: sb.thumb_size,
                track_color: sb.track_color,
                thumb_color: sb.thumb_color,
            });
        }
    }

    /// Resolve face attributes for grid materialization.
    ///
    /// Returns a helper struct with the resolved colors, font properties, and
    /// decoration flags needed by `FrameGlyph::Char` and `FrameGlyph::Stretch`.
    fn resolve_face_for_materialize(&self, face_id: u32) -> MaterializedFaceData {
        if let Some(face) = self.faces.get(&face_id) {
            let underline = face.underline_style.gnu_code();
            MaterializedFaceData {
                fg: face.foreground,
                bg: face.background,
                font_ascent: face.font_ascent.max(0) as f32,
                font_weight: face.font_weight,
                italic: face.attributes.contains(FaceAttributes::ITALIC),
                font_size: face.font_size,
                underline,
                underline_color: face.underline_color,
                strike_through: if face.attributes.contains(FaceAttributes::STRIKE_THROUGH) {
                    1
                } else {
                    0
                },
                strike_through_color: face.strike_through_color,
                overline: if face.attributes.contains(FaceAttributes::OVERLINE) {
                    1
                } else {
                    0
                },
                overline_color: face.overline_color,
                overstrike: false,
            }
        } else {
            MaterializedFaceData {
                fg: Color::new(1.0, 1.0, 1.0, 1.0),
                bg: self.background,
                font_ascent: 0.0,
                font_weight: 400,
                italic: false,
                font_size: self.font_pixel_size,
                underline: 0,
                underline_color: None,
                strike_through: 0,
                strike_through_color: None,
                overline: 0,
                overline_color: None,
                overstrike: false,
            }
        }
    }

    fn for_each_grid_row_glyph(
        &self,
        window_id: DisplayWindowId,
        row_index: u32,
        glyph_row: &GlyphRow,
        pixel_bounds: Rect,
        char_w: f32,
        char_h: f32,
        push: &mut impl FnMut(FrameGlyph),
    ) {
        if !glyph_row.enabled {
            return;
        }

        let win_x = pixel_bounds.x;
        let win_y = pixel_bounds.y;
        let win_w = pixel_bounds.width;
        let y = if glyph_row.height_px > 0.0 {
            win_y + glyph_row.pixel_y
        } else {
            win_y + row_index as f32 * char_h
        };
        let row_height = if glyph_row.height_px > 0.0 {
            glyph_row.height_px
        } else {
            char_h
        };
        let row_role = glyph_row.role;
        let row_ascent = if glyph_row.ascent_px > 0.0 {
            glyph_row.ascent_px.min(row_height)
        } else {
            row_height
        };
        let clip_rect = Some(pixel_bounds);
        let mut col = 0usize;
        let mut x_cursor = win_x;

        // GNU `reversed_p` rows (right-to-left paragraphs) are flush to the
        // right margin: start the pen so the content ends at the right edge,
        // leaving the empty space on the left (drawn as background). The pen
        // then advances left-to-right as usual over the already visually
        // reordered glyphs.
        if glyph_row.reversed_p {
            let used: f32 = glyph_row.glyphs[GlyphArea::Text.index()]
                .iter()
                .filter(|glyph| !glyph.padding)
                .map(|glyph| {
                    if glyph.pixel_width > 0.0 {
                        glyph.pixel_width
                    } else {
                        match &glyph.glyph_type {
                            GlyphType::Stretch { width_cols } => *width_cols as f32 * char_w,
                            _ if glyph.wide => char_w * 2.0,
                            _ => char_w,
                        }
                    }
                })
                .sum();
            x_cursor = win_x + (win_w - used).max(0.0);
        }

        for area_idx in 0..3 {
            for glyph in &glyph_row.glyphs[area_idx] {
                if glyph.padding {
                    continue;
                }
                let fallback_width = match &glyph.glyph_type {
                    GlyphType::Stretch { width_cols } => *width_cols as f32 * char_w,
                    GlyphType::Image { .. } | GlyphType::Glyphless { .. } => char_w,
                    GlyphType::Char { .. } | GlyphType::Composite { .. } => {
                        if glyph.wide {
                            char_w * 2.0
                        } else {
                            char_w
                        }
                    }
                };
                let glyph_width = if glyph.pixel_width > 0.0 {
                    glyph.pixel_width
                } else {
                    fallback_width
                };
                let x = x_cursor;
                let right_edge = win_x + win_w;
                if x >= right_edge {
                    break;
                }
                let materialized_width = glyph_width.min(right_edge - x).max(0.0);
                if materialized_width <= 0.0 {
                    break;
                }
                let slot_id = DisplaySlotId {
                    window_id,
                    row: row_index,
                    col: col as u16,
                };

                match &glyph.glyph_type {
                    GlyphType::Char { ch } => {
                        let font_ascent =
                            self.resolve_face_for_materialize(glyph.face_id).font_ascent;
                        let row_ascent = if glyph_row.ascent_px > 0.0 {
                            glyph_row.ascent_px
                        } else if font_ascent > 0.0 {
                            font_ascent.min(row_height)
                        } else {
                            row_height
                        };
                        let baseline = y + row_ascent + glyph.vertical_offset_px;
                        push(FrameGlyph::Char {
                            window_id,
                            row_role,
                            clip_rect,
                            slot_id,
                            bidi_level: glyph.bidi_level,
                            char: *ch,
                            composed: None,
                            x,
                            y,
                            baseline,
                            width: materialized_width,
                            height: row_height,
                            ascent: if font_ascent > 0.0 {
                                font_ascent.min(row_height)
                            } else {
                                row_ascent
                            },
                            face_id: glyph.face_id,
                        });
                    }
                    GlyphType::Composite { text } => {
                        let font_ascent =
                            self.resolve_face_for_materialize(glyph.face_id).font_ascent;
                        let row_ascent = if glyph_row.ascent_px > 0.0 {
                            glyph_row.ascent_px
                        } else if font_ascent > 0.0 {
                            font_ascent.min(row_height)
                        } else {
                            row_height
                        };
                        let baseline = y + row_ascent + glyph.vertical_offset_px;
                        push(FrameGlyph::Char {
                            window_id,
                            row_role,
                            clip_rect,
                            slot_id,
                            bidi_level: glyph.bidi_level,
                            char: text.chars().next().unwrap_or(' '),
                            composed: Some(text.clone()),
                            x,
                            y,
                            baseline,
                            width: materialized_width,
                            height: row_height,
                            ascent: if font_ascent > 0.0 {
                                font_ascent.min(row_height)
                            } else {
                                row_ascent
                            },
                            face_id: glyph.face_id,
                        });
                    }
                    GlyphType::Stretch { .. } => {
                        let face_data = self.resolve_face_for_materialize(glyph.face_id);
                        let stretch_height = if glyph.pixel_height > 0.0 {
                            glyph.pixel_height
                        } else {
                            row_height
                        };
                        let stretch_ascent = if glyph.pixel_height > 0.0 {
                            glyph.pixel_ascent.min(stretch_height)
                        } else {
                            row_ascent.min(stretch_height)
                        };
                        let stretch_y = if glyph.pixel_height > 0.0 {
                            y + row_ascent - stretch_ascent
                        } else {
                            y
                        } + glyph.vertical_offset_px;
                        push(FrameGlyph::Stretch {
                            window_id,
                            row_role,
                            clip_rect,
                            slot_id,
                            bidi_level: glyph.bidi_level,
                            x,
                            y: stretch_y,
                            width: materialized_width,
                            height: stretch_height,
                            bg: face_data.bg,
                            face_id: glyph.face_id,
                            stipple_id: 0,
                            stipple_fg: None,
                        });
                    }
                    GlyphType::Image { image_id } => {
                        push(FrameGlyph::Image {
                            window_id,
                            row_role,
                            clip_rect,
                            slot_id: Some(slot_id),
                            image_id: ImageId::new(*image_id as u32),
                            x,
                            y: y + glyph.vertical_offset_px,
                            width: materialized_width,
                            height: row_height,
                        });
                    }
                    GlyphType::Glyphless { ch } => {
                        let font_ascent =
                            self.resolve_face_for_materialize(glyph.face_id).font_ascent;
                        let row_ascent = if glyph_row.ascent_px > 0.0 {
                            glyph_row.ascent_px
                        } else if font_ascent > 0.0 {
                            font_ascent.min(row_height)
                        } else {
                            row_height
                        };
                        let baseline = y + row_ascent + glyph.vertical_offset_px;
                        push(FrameGlyph::Char {
                            window_id,
                            row_role,
                            clip_rect,
                            slot_id,
                            bidi_level: glyph.bidi_level,
                            char: *ch,
                            composed: None,
                            x,
                            y,
                            baseline,
                            width: materialized_width,
                            height: row_height,
                            ascent: if font_ascent > 0.0 {
                                font_ascent.min(row_height)
                            } else {
                                row_ascent
                            },
                            face_id: glyph.face_id,
                        });
                    }
                }
                col += match &glyph.glyph_type {
                    GlyphType::Stretch { width_cols } => *width_cols as usize,
                    _ => {
                        if glyph.wide {
                            2
                        } else {
                            1
                        }
                    }
                };
                x_cursor += glyph_width;
            }
        }

        let final_x = x_cursor.min(win_x + win_w);
        let right_edge = win_x + win_w;
        if final_x < right_edge && col > 0 && row_role.is_chrome() {
            let last_face_id = glyph_row
                .glyphs
                .iter()
                .rev()
                .flat_map(|area| area.iter().rev())
                .find(|g| !g.padding)
                .map(|g| g.face_id)
                .unwrap_or(0);
            let face_data = self.resolve_face_for_materialize(last_face_id);
            push(FrameGlyph::Stretch {
                window_id,
                row_role,
                clip_rect,
                slot_id: DisplaySlotId {
                    window_id,
                    row: row_index,
                    col: col as u16,
                },
                bidi_level: 0,
                x: final_x,
                y,
                width: right_edge - final_x,
                height: row_height,
                bg: face_data.bg,
                face_id: last_face_id,
                stipple_id: 0,
                stipple_fg: None,
            });
        }
    }
}

impl WindowMatrixEntry {
    pub fn row_pixel_bounds(&self, role: GlyphRowRole) -> Rect {
        if role == GlyphRowRole::Text {
            self.text_pixel_bounds
        } else {
            self.pixel_bounds
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScrollRun {
    pub window_id: u64,
    pub first_row: usize,
    pub last_row: usize,
    pub distance: i32,
}

pub trait RedisplayInterface {
    fn update_window_begin(&mut self, window_id: u64);
    fn write_glyphs(&mut self, row: &GlyphRow, area: GlyphArea, start: usize, len: usize);
    fn clear_end_of_line(&mut self, row: &GlyphRow, area: GlyphArea);
    fn scroll_run(&mut self, run: &ScrollRun);
    fn update_window_end(&mut self, window_id: u64);
    fn set_cursor(&mut self, row: u16, col: u16, style: CursorStyle);
    fn flush(&mut self);
}

#[cfg(test)]
#[path = "glyph_matrix_test.rs"]
mod tests;
