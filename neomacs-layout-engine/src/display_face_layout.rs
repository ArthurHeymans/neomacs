use crate::neovm_bridge::ResolvedFace;

pub(crate) struct DisplayHeightFaceBasis<'a> {
    pub(crate) canonical_face: &'a ResolvedFace,
    pub(crate) base_face: &'a ResolvedFace,
    pub(crate) fallback_char_width: f32,
    pub(crate) fallback_ascent: f32,
    pub(crate) fallback_row_height: f32,
}

pub(crate) fn height_adjusted_face(
    source: &ResolvedFace,
    basis: DisplayHeightFaceBasis<'_>,
    factor: f32,
) -> Option<ResolvedFace> {
    if !factor.is_finite() || factor <= 0.0 {
        return None;
    }
    if basis.fallback_char_width <= 1.0 && basis.fallback_row_height <= 1.0 {
        return None;
    }

    let canonical = basis.canonical_face;
    let canonical_font_size = positive_f32(canonical.font_size)
        .or_else(|| positive_f32(basis.base_face.font_size))
        .or_else(|| positive_f32(source.font_size))
        .unwrap_or_else(|| basis.fallback_row_height.max(1.0));
    let canonical_line_height = positive_f32(canonical.font_line_height)
        .or_else(|| positive_f32(basis.fallback_row_height))
        .unwrap_or(canonical_font_size);
    let canonical_ascent = positive_f32(canonical.font_ascent)
        .or_else(|| positive_f32(basis.fallback_ascent))
        .unwrap_or(canonical_line_height * 0.8)
        .min(canonical_line_height);
    let canonical_char_width = positive_f32(canonical.font_char_width)
        .or_else(|| positive_f32(basis.fallback_char_width))
        .unwrap_or(canonical_font_size * 0.5);

    let mut resolved = source.clone();
    resolved.font_size = (canonical_font_size * factor).max(1.0);
    resolved.font_line_height = (canonical_line_height * factor).max(1.0);
    resolved.font_ascent = (canonical_ascent * factor)
        .max(1.0)
        .min(resolved.font_line_height);
    resolved.font_char_width = (canonical_char_width * factor).max(1.0);
    Some(resolved)
}

fn positive_f32(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0).then_some(value)
}
