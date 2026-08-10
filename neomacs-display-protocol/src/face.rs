//! Face (text styling) types.

use crate::types::{Color, FaceId};
use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_int;
use strum::{EnumString, IntoStaticStr};

bitflags! {
    /// Face attributes flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FaceAttributes: u32 {
        const BOLD = 1 << 0;
        const ITALIC = 1 << 1;
        const UNDERLINE = 1 << 2;
        const OVERLINE = 1 << 3;
        const STRIKE_THROUGH = 1 << 4;
        const INVERSE = 1 << 5;
        const BOX = 1 << 6;
    }
}

// Serialize as the raw u32 bit pattern: stable, lossless, and a plain
// integer in snapshot JSON (bitflags' own serde feature would emit the
// "BOLD | ITALIC" string form, which is noisier for machine consumers).
impl serde::Serialize for FaceAttributes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> serde::Deserialize<'de> for FaceAttributes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(FaceAttributes::from_bits_retain(u32::deserialize(
            deserializer,
        )?))
    }
}

/// Underline style.
///
/// The numeric values match GNU Emacs `enum face_underline_type` and the
/// `Smulx` terminfo underline style parameter.
#[repr(u8)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    IntoPrimitive,
    TryFromPrimitive,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum UnderlineStyle {
    #[default]
    None = 0,
    Line = 1,
    Double = 2,
    Wave = 3,
    Dotted = 4,
    Dashed = 5,
}

impl UnderlineStyle {
    pub fn from_gnu_code(code: u8) -> Option<Self> {
        Self::try_from(code).ok()
    }

    pub fn gnu_code(self) -> u8 {
        self.into()
    }
}

/// Box type for face
#[repr(u8)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    IntoPrimitive,
    TryFromPrimitive,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum BoxType {
    #[default]
    None = 0,
    Line = 1,
    Raised3D = 2,
    Sunken3D = 3,
}

impl BoxType {
    pub fn from_gnu_code(code: u8) -> Option<Self> {
        Self::try_from(code).ok()
    }

    pub fn gnu_code(self) -> u8 {
        self.into()
    }
}

/// GNU Emacs scalar `:box :line-width` semantics.
///
/// The magnitude is the painted border thickness. A positive value expands
/// the glyph row vertically; a negative value paints the top and bottom edges
/// inside the existing row. GNU uses the absolute magnitude for the left and
/// right edges in both cases (`struct face::box_{vertical,horizontal}_line_width`).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BoxLineWidth(i32);

impl BoxLineWidth {
    pub const fn from_gnu(value: i32) -> Self {
        Self(value)
    }

    pub const fn gnu_value(self) -> i32 {
        self.0
    }

    pub const fn is_visible(self) -> bool {
        self.0 != 0
    }

    pub const fn paint_thickness(self) -> u32 {
        self.0.unsigned_abs()
    }

    pub const fn expands_row_height(self) -> bool {
        self.0 > 0
    }

    pub const fn row_expansion_per_edge(self) -> u32 {
        if self.expands_row_height() {
            self.paint_thickness()
        } else {
            0
        }
    }
}

impl From<i32> for BoxLineWidth {
    fn from(value: i32) -> Self {
        Self::from_gnu(value)
    }
}

/// Fancy box border style — a Neomacs extension to GNU's box faces.
///
/// The numeric codes are the wire values used by the C-side
/// `fill_face_data()` (`FaceDataFFI::box_border_style`) and by the
/// GPU border shader's style id.
#[repr(u32)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    IntoPrimitive,
    TryFromPrimitive,
    serde::Serialize,
    serde::Deserialize,
)]
#[non_exhaustive]
pub enum BoxBorderStyle {
    #[default]
    Solid = 0,
    Rainbow = 1,
    AnimatedRainbow = 2,
    Gradient = 3,
    Glow = 4,
    Neon = 5,
    Dashed = 6,
    Comet = 7,
    Iridescent = 8,
    Fire = 9,
    Heartbeat = 10,
}

impl BoxBorderStyle {
    pub fn from_gnu_code(code: u32) -> Option<Self> {
        Self::try_from(code).ok()
    }

    pub fn gnu_code(self) -> u32 {
        self.into()
    }

    /// True for any animated/fancy style (everything except plain solid).
    pub fn is_fancy(self) -> bool {
        self != Self::Solid
    }
}

/// Basic face IDs — fixed cache slots matching GNU's `enum face_id`.
///
/// Realized at frame creation via `realize_basic_faces()` in GNU
/// (`src/xfaces.c`).  NeoMacs mirrors this: basic faces always occupy
/// IDs 0–19 in every frame's face cache; dynamic faces start at
/// [`BasicFaceId::SENTINEL`].
#[repr(u32)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    IntoPrimitive,
    IntoStaticStr,
    TryFromPrimitive,
)]
#[strum(serialize_all = "kebab-case")]
pub enum BasicFaceId {
    Default = 0,
    ModeLineActive = 1,
    ModeLineInactive = 2,
    ToolBar = 3,
    Fringe = 4,
    HeaderLineActive = 5,
    HeaderLineInactive = 6,
    ScrollBar = 7,
    Border = 8,
    Cursor = 9,
    Mouse = 10,
    Menu = 11,
    VerticalBorder = 12,
    WindowDivider = 13,
    WindowDividerFirstPixel = 14,
    WindowDividerLastPixel = 15,
    InternalBorder = 16,
    ChildFrameBorder = 17,
    TabBar = 18,
    TabLine = 19,
}

impl BasicFaceId {
    /// One past the last basic face.  Dynamic face IDs start here.
    pub const SENTINEL: u32 = 20;

    /// Look up a basic face by its canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// Look up a basic face by GNU's fixed `enum face_id` value.
    pub fn from_gnu_code(code: u32) -> Option<Self> {
        Self::try_from(code).ok()
    }

    /// Return the canonical basic face name.
    pub fn name(self) -> &'static str {
        self.into()
    }

    /// Return the GNU Lisp face realized into this cache slot.
    ///
    /// Cache-role names and Lisp face names are deliberately distinct for the
    /// active mode/header slots: GNU's `MODE_LINE_FACE_ID` realizes
    /// `mode-line`, while only the inactive slot realizes
    /// `mode-line-inactive` (likewise for `header-line`).  Keep this mapping
    /// exhaustive so adding a cache role cannot silently select a made-up Lisp
    /// face name.
    pub const fn lisp_face_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ModeLineActive => "mode-line",
            Self::ModeLineInactive => "mode-line-inactive",
            Self::ToolBar => "tool-bar",
            Self::Fringe => "fringe",
            Self::HeaderLineActive => "header-line",
            Self::HeaderLineInactive => "header-line-inactive",
            Self::ScrollBar => "scroll-bar",
            Self::Border => "border",
            Self::Cursor => "cursor",
            Self::Mouse => "mouse",
            Self::Menu => "menu",
            Self::VerticalBorder => "vertical-border",
            Self::WindowDivider => "window-divider",
            Self::WindowDividerFirstPixel => "window-divider-first-pixel",
            Self::WindowDividerLastPixel => "window-divider-last-pixel",
            Self::InternalBorder => "internal-border",
            Self::ChildFrameBorder => "child-frame-border",
            Self::TabBar => "tab-bar",
            Self::TabLine => "tab-line",
        }
    }

    /// Return GNU's fixed `enum face_id` value.
    pub fn gnu_code(self) -> u32 {
        self.into()
    }
}

/// Basic faces occupy their fixed cache slot in every frame's face table.
impl From<BasicFaceId> for FaceId {
    fn from(basic: BasicFaceId) -> Self {
        FaceId::new(basic.gnu_code())
    }
}

/// A face defines text styling (colors, font, decorations)
#[repr(C)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Face {
    /// Face ID
    pub id: FaceId,

    /// Foreground color
    pub foreground: Color,

    /// Background color
    pub background: Color,

    /// Use the terminal's default foreground instead of `foreground`.
    pub use_default_foreground: bool,

    /// Use the terminal's default background instead of `background`.
    pub use_default_background: bool,

    /// Underline color (if different from foreground)
    pub underline_color: Option<Color>,

    /// Overline color
    pub overline_color: Option<Color>,

    /// Strike-through color
    pub strike_through_color: Option<Color>,

    /// Box color
    pub box_color: Option<Color>,

    /// Font family name
    pub font_family: String,

    /// Font size in points (1/72 inch)
    pub font_size: f32,

    /// Font weight (400 = normal, 700 = bold)
    pub font_weight: u16,

    /// Attribute flags
    pub attributes: FaceAttributes,

    /// Underline style
    pub underline_style: UnderlineStyle,

    /// Box type
    pub box_type: BoxType,

    /// GNU scalar box line width, including inside/outside sign semantics.
    pub box_line_width: BoxLineWidth,

    /// Box corner radius (0 = sharp corners)
    pub box_corner_radius: i32,

    /// Fancy border style; see [`BoxBorderStyle`].
    pub box_border_style: BoxBorderStyle,

    /// Animation speed multiplier for fancy border effects (default 1.0)
    pub box_border_speed: f32,

    /// Secondary box color (for gradient, neon, etc.)
    pub box_color2: Option<Color>,

    /// Absolute path to the resolved font file (from Fontconfig), if available.
    /// Used to pre-load the exact font file into cosmic-text's fontdb.
    pub font_file_path: Option<String>,

    /// Font metrics from Emacs's realized font
    /// Font ascent (FONT_BASE) in pixels
    pub font_ascent: i32,
    /// Font descent (FONT_DESCENT) in pixels
    pub font_descent: i32,
    /// Underline position below baseline (font->underline_position)
    pub underline_position: i32,
    /// Underline thickness (font->underline_thickness)
    pub underline_thickness: i32,

    /// Optional GPU-rendered gradient background.
    /// If present, this overrides the solid `background` color during rendering.
    /// GPU fragment shader evaluates the gradient per-pixel with no CPU overhead.
    pub background_gradient: Option<Box<crate::gradient::Gradient>>,

    /// Lisp face name this realized face came from (e.g.
    /// "font-lock-keyword-face"), when known. `None` for anonymous faces
    /// realized from raw attribute plists. Basic faces (id 0-19) fall back
    /// to their canonical [`BasicFaceId`] name at snapshot time.
    ///
    /// NOTE: new fields append AFTER this one so the `#[repr(C)]` prefix
    /// layout stays stable for existing readers.
    pub lisp_name: Option<String>,

    /// The face's primary resolved font, referencing the frame state's
    /// resolved font table (`FrameDisplayState::fonts`). When present, the
    /// renderer must rasterize with exactly this font instead of re-running
    /// semantic selection from `font_family`/`font_weight`/attributes.
    pub default_resolved_font_id: Option<crate::font::ResolvedFontId>,

    /// Stipple bitmap painted as the glyph background (GNU `face->stipple`).
    /// When present the renderer tiles this XBM pattern over the glyph's
    /// background rect, painting 1-bits in the face foreground over the solid
    /// background (`highlight-indent-guides`/`indent-bars` rely on this).
    /// Boxed to keep `Face` small, matching `background_gradient`.
    #[serde(default)]
    pub stipple: Option<Box<crate::frame_glyphs::StipplePattern>>,
}

impl Default for Face {
    fn default() -> Self {
        Self {
            id: FaceId::new(0),
            foreground: Color::WHITE,
            background: Color::BLACK,
            use_default_foreground: false,
            use_default_background: false,
            underline_color: None,
            overline_color: None,
            strike_through_color: None,
            box_color: None,
            font_family: "monospace".to_string(),
            font_size: 10.0,
            font_weight: 400,
            attributes: FaceAttributes::empty(),
            underline_style: UnderlineStyle::None,
            box_type: BoxType::None,
            box_line_width: BoxLineWidth::default(),
            box_corner_radius: 0,
            box_border_style: BoxBorderStyle::Solid,
            box_border_speed: 1.0,
            box_color2: None,
            font_file_path: None,
            font_ascent: 0,
            font_descent: 0,
            underline_position: 1,
            underline_thickness: 1,
            background_gradient: None,
            lisp_name: None,
            default_resolved_font_id: None,
            stipple: None,
        }
    }
}

impl Face {
    /// Create a new face with default values
    pub fn new(id: FaceId) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    /// Check if face is bold
    pub fn is_bold(&self) -> bool {
        self.attributes.contains(FaceAttributes::BOLD) || self.font_weight >= 700
    }

    /// Check if face is italic
    pub fn is_italic(&self) -> bool {
        self.attributes.contains(FaceAttributes::ITALIC)
    }

    /// Check if face has underline
    pub fn has_underline(&self) -> bool {
        self.underline_style != UnderlineStyle::None
    }

    /// Get the underline color (foreground if not explicitly set)
    pub fn get_underline_color(&self) -> Color {
        self.underline_color.unwrap_or(self.foreground)
    }

    /// Create a Pango font description string
    pub fn to_pango_font_description(&self) -> String {
        let mut desc = self.font_family.clone();

        if self.is_italic() {
            desc.push_str(" Italic");
        }

        if self.is_bold() {
            desc.push_str(" Bold");
        }

        desc.push_str(&format!(" {}", self.font_size as i32));
        desc
    }
}

/// FFI-safe face data struct, populated by C's `fill_face_data()`.
///
/// This is the canonical bridge type between C (Emacs face system) and Rust.
/// Layout must match the C `struct FaceDataFFI` in `neomacsterm.c`.
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct FaceDataFFI {
    /// Face ID
    pub face_id: u32,
    /// Foreground color (sRGB pixel: 0x00RRGGBB)
    pub fg: u32,
    /// Background color (sRGB pixel: 0x00RRGGBB)
    pub bg: u32,
    /// Font family name (null-terminated C string, valid for duration of layout)
    pub font_family: *const c_char,
    /// Font weight (CSS scale: 400=normal, 700=bold)
    pub font_weight: c_int,
    /// Italic flag
    pub italic: c_int,
    /// Font pixel size
    pub font_size: c_int,
    /// Underline style (0=none, 1=single, 2=double, 3=wave, 4=dotted, 5=dashed)
    pub underline_style: c_int,
    /// Underline color (sRGB pixel)
    pub underline_color: u32,
    /// Strike-through (0=none, 1=enabled)
    pub strike_through: c_int,
    /// Strike-through color
    pub strike_through_color: u32,
    /// Overline (0=none, 1=enabled)
    pub overline: c_int,
    /// Overline color
    pub overline_color: u32,
    /// Box type (0=none, 1=line, 2=raised, 3=sunken)
    pub box_type: c_int,
    /// Box color
    pub box_color: u32,
    /// Box line width
    pub box_line_width: c_int,
    /// Box corner radius (0 = sharp corners)
    pub box_corner_radius: c_int,
    /// Fancy border style (0=solid, 1=rainbow, 2=animated-rainbow, 3=gradient,
    /// 4=glow, 5=neon, 6=dashed, 7=comet, 8=iridescent, 9=fire, 10=heartbeat)
    pub box_border_style: c_int,
    /// Animation speed multiplier (100 = 1.0x)
    pub box_border_speed: c_int,
    /// Secondary box color (sRGB pixel: 0x00RRGGBB)
    pub box_color2: u32,
    /// Signed box horizontal (top/bottom) line width.
    /// `>0`: box adds height (borders drawn outside text area).
    /// `<0`: box drawn within text area (no extra height).
    pub box_h_line_width: c_int,
    /// Extend: face bg extends to end of visual line (0=no, 1=yes)
    pub extend: c_int,
    /// Per-face font character width (0.0 = use window default)
    pub font_char_width: f32,
    /// Per-face font ascent (0.0 = use window default)
    pub font_ascent: f32,
    /// Per-face space width (for tab stop calculations with proportional fonts)
    pub font_space_width: f32,
    /// Whether the face's font is monospace (1=monospace, 0=proportional)
    pub font_is_monospace: c_int,
    /// Stipple bitmap ID (0 = none, positive = 1-based bitmap index)
    pub stipple: c_int,
    /// Overstrike flag (1 = simulate bold by drawing twice at x and x+1)
    pub overstrike: c_int,
    /// Font descent in pixels (FONT_DESCENT)
    pub font_descent: c_int,
    /// Underline position below baseline (font->underline_position, >=1)
    pub underline_position: c_int,
    /// Underline thickness in pixels (font->underline_thickness, >=1)
    pub underline_thickness: c_int,
    /// Absolute path to resolved font file (from Fontconfig), or NULL.
    pub font_file_path: *const c_char,
}

// Safety: FaceDataFFI contains raw pointers that are only valid during
// the FFI call. The to_face() method copies all data into owned types.
unsafe impl Send for FaceDataFFI {}
unsafe impl Sync for FaceDataFFI {}

impl FaceDataFFI {
    /// Convert FFI face data to the Rust `Face` type.
    ///
    /// # Safety
    /// Caller must ensure `font_family` and `font_file_path` pointers
    /// (if non-null) point to valid, null-terminated C strings.
    pub unsafe fn to_face(&self) -> Face {
        let font_family = if !self.font_family.is_null() {
            unsafe { CStr::from_ptr(self.font_family) }
                .to_str()
                .unwrap_or("monospace")
                .to_string()
        } else {
            "monospace".to_string()
        };

        let font_file_path = if !self.font_file_path.is_null() {
            unsafe { CStr::from_ptr(self.font_file_path) }
                .to_str()
                .ok()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        } else {
            None
        };

        let underline_style_code = u8::try_from(self.underline_style).unwrap_or(0);
        let underline_style =
            UnderlineStyle::from_gnu_code(underline_style_code).unwrap_or_default();
        let has_underline = underline_style != UnderlineStyle::None;
        let strike_through = self.strike_through > 0;
        let overline = self.overline > 0;
        let font_weight = self.font_weight.max(0) as u16;

        let mut attrs = FaceAttributes::empty();
        if font_weight >= 700 {
            attrs |= FaceAttributes::BOLD;
        }
        if self.italic != 0 {
            attrs |= FaceAttributes::ITALIC;
        }
        if has_underline {
            attrs |= FaceAttributes::UNDERLINE;
        }
        if strike_through {
            attrs |= FaceAttributes::STRIKE_THROUGH;
        }
        if overline {
            attrs |= FaceAttributes::OVERLINE;
        }
        let box_type =
            BoxType::from_gnu_code(u8::try_from(self.box_type).unwrap_or(0)).unwrap_or_default();
        if !matches!(box_type, BoxType::None) {
            attrs |= FaceAttributes::BOX;
        }

        Face {
            // FFI edge: wrap the raw C-side face id into the typed FaceId here.
            id: FaceId::new(self.face_id),
            foreground: Color::from_pixel(self.fg),
            background: Color::from_pixel(self.bg),
            use_default_foreground: false,
            use_default_background: false,
            underline_color: has_underline.then(|| Color::from_pixel(self.underline_color)),
            overline_color: overline.then(|| Color::from_pixel(self.overline_color)),
            strike_through_color: strike_through
                .then(|| Color::from_pixel(self.strike_through_color)),
            box_color: (box_type != BoxType::None).then(|| Color::from_pixel(self.box_color)),
            font_family,
            font_size: self.font_size.max(0) as f32,
            font_weight,
            attributes: attrs,
            underline_style,
            box_type,
            box_line_width: BoxLineWidth::from_gnu(self.box_line_width),
            box_corner_radius: self.box_corner_radius,
            // FFI edge: decode the raw C style code; unknown codes fall back
            // to the solid border rather than propagating garbage.
            box_border_style: BoxBorderStyle::from_gnu_code(self.box_border_style.max(0) as u32)
                .unwrap_or_default(),
            box_border_speed: self.box_border_speed as f32 / 100.0,
            box_color2: (self.box_color2 != 0).then(|| Color::from_pixel(self.box_color2)),
            font_file_path,
            font_ascent: self.font_ascent as i32,
            font_descent: self.font_descent,
            underline_position: self.underline_position.max(1),
            underline_thickness: self.underline_thickness.max(1),
            background_gradient: None,
            lisp_name: None,
            default_resolved_font_id: None,
            // The legacy FFI face path does not carry a realized stipple; the
            // typed layout-engine path (`render_face`) is the only producer.
            stipple: None,
        }
    }
}

/// Face cache for efficient lookup
#[derive(Debug, Default)]
pub struct FaceCache {
    faces: Vec<Face>,
}

impl FaceCache {
    pub fn new() -> Self {
        Self { faces: Vec::new() }
    }

    /// Get face by ID
    pub fn get(&self, id: FaceId) -> Option<&Face> {
        self.faces.iter().find(|f| f.id == id)
    }

    /// Get or create a face by ID
    pub fn get_or_create(&mut self, id: FaceId) -> &Face {
        // Check if exists
        if self.get(id).is_some() {
            return self.get(id).unwrap();
        }
        // Create new
        let face = Face::new(id);
        self.faces.push(face);
        self.faces.last().unwrap()
    }

    /// Add or update a face, returns the face ID
    pub fn insert(&mut self, face: Face) -> FaceId {
        let id = face.id;
        if let Some(existing) = self.faces.iter_mut().find(|f| f.id == face.id) {
            *existing = face;
        } else {
            self.faces.push(face);
        }
        id
    }

    /// Get default face (ID 0)
    pub fn default_face(&self) -> Option<&Face> {
        self.get(FaceId::new(0))
    }
}

#[cfg(test)]
#[path = "face_test.rs"]
mod tests;
