use crate::neovm_bridge::ResolvedFace;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::emacs_core::Value;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) enum DisplaySource {
    PropertizedString(Value),
    PlainString(String),
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct DisplayRowRequest {
    pub role: GlyphRowRole,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub window_id: i64,
    pub matrix_row: Option<usize>,
    pub base_face: ResolvedFace,
    pub source: DisplaySource,
}

#[cfg(test)]
#[path = "display_row_test.rs"]
mod tests;
