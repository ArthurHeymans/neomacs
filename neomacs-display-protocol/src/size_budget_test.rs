use std::mem::size_of;

use crate::effect_config::EffectsConfig;
use crate::face::Face;
use crate::frame_glyphs::FrameGlyph;
use crate::glyph_matrix::Glyph;

#[test]
fn display_protocol_hot_path_type_sizes_stay_within_budget() {
    assert!(
        size_of::<Glyph>() <= 56,
        "Glyph grew to {} bytes; update the display audit budget intentionally",
        size_of::<Glyph>()
    );
    assert!(
        size_of::<FrameGlyph>() <= 112,
        "FrameGlyph grew to {} bytes; update the display audit budget intentionally",
        size_of::<FrameGlyph>()
    );
    assert!(
        size_of::<Face>() <= 272,
        "Face grew to {} bytes; update the display audit budget intentionally",
        size_of::<Face>()
    );
    assert!(
        size_of::<EffectsConfig>() <= 3576,
        "EffectsConfig grew to {} bytes; update the display audit budget intentionally",
        size_of::<EffectsConfig>()
    );
}
