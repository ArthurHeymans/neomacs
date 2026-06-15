use crate::display_face_policy::BaseFacePolicy;
use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::buffer::CharPos0;
use neovm_core::emacs_core::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OverlayStringKind {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayPropertySource {
    TextProperty,
    #[allow(dead_code)]
    Overlay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayOrigin {
    BufferText {
        charpos: CharPos0,
    },
    OverlayString {
        overlay_id: Value,
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    },
    DisplayPropertyString {
        anchor_charpos: CharPos0,
        source: DisplayPropertySource,
    },
    LinePrefix {
        anchor_charpos: CharPos0,
    },
    WrapPrefix {
        anchor_charpos: CharPos0,
    },
    ModeLine {
        selected: bool,
    },
    HeaderLine {
        selected: bool,
    },
    TabLine,
    TabBar,
    Minibuffer,
    EchoArea,
    #[allow(dead_code)]
    Posframe,
}

impl DisplayOrigin {
    pub(crate) fn default_base_face_policy(self) -> BaseFacePolicy {
        match self {
            Self::BufferText { .. } => BaseFacePolicy::BufferFaceIncludingOverlays,
            Self::OverlayString { .. } => BaseFacePolicy::OverlayStringAtAnchor,
            Self::DisplayPropertyString { .. } => BaseFacePolicy::DisplayPropertyUnderlyingFace,
            Self::LinePrefix { .. }
            | Self::WrapPrefix { .. }
            | Self::Minibuffer
            | Self::EchoArea
            | Self::Posframe => BaseFacePolicy::DefaultFace,
            Self::ModeLine { selected } => BaseFacePolicy::FixedBasicFace(if selected {
                BasicFaceId::ModeLineActive
            } else {
                BasicFaceId::ModeLineInactive
            }),
            Self::HeaderLine { selected } => BaseFacePolicy::FixedBasicFace(if selected {
                BasicFaceId::HeaderLineActive
            } else {
                BasicFaceId::HeaderLineInactive
            }),
            Self::TabLine => BaseFacePolicy::FixedBasicFace(BasicFaceId::TabLine),
            Self::TabBar => BaseFacePolicy::FixedBasicFace(BasicFaceId::TabBar),
        }
    }

    pub(crate) fn glyph_row_role(self) -> Option<GlyphRowRole> {
        match self {
            Self::ModeLine { .. } => Some(GlyphRowRole::ModeLine),
            Self::HeaderLine { .. } => Some(GlyphRowRole::HeaderLine),
            Self::TabLine => Some(GlyphRowRole::TabLine),
            Self::TabBar => Some(GlyphRowRole::TabBar),
            Self::Minibuffer | Self::EchoArea => Some(GlyphRowRole::Minibuffer),
            Self::Posframe => Some(GlyphRowRole::Text),
            Self::BufferText { .. }
            | Self::OverlayString { .. }
            | Self::DisplayPropertyString { .. }
            | Self::LinePrefix { .. }
            | Self::WrapPrefix { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_origin_models_all_display_text_sources() {
        let _ = DisplayOrigin::BufferText {
            charpos: CharPos0::new(0),
        };
        let _ = DisplayOrigin::OverlayString {
            overlay_id: Value::fixnum(1),
            anchor_charpos: CharPos0::new(0),
            kind: OverlayStringKind::Before,
        };
        let _ = DisplayOrigin::DisplayPropertyString {
            anchor_charpos: CharPos0::new(0),
            source: DisplayPropertySource::TextProperty,
        };
        let _ = DisplayOrigin::LinePrefix {
            anchor_charpos: CharPos0::new(0),
        };
        let _ = DisplayOrigin::WrapPrefix {
            anchor_charpos: CharPos0::new(0),
        };
        let _ = DisplayOrigin::ModeLine { selected: true };
        let _ = DisplayOrigin::HeaderLine { selected: true };
        let _ = DisplayOrigin::TabLine;
        let _ = DisplayOrigin::TabBar;
        let _ = DisplayOrigin::Minibuffer;
        let _ = DisplayOrigin::EchoArea;
        let _ = DisplayOrigin::Posframe;
    }

    #[test]
    fn display_origin_derives_default_base_face_policy() {
        assert_eq!(
            DisplayOrigin::BufferText {
                charpos: CharPos0::new(3),
            }
            .default_base_face_policy(),
            BaseFacePolicy::BufferFaceIncludingOverlays
        );
        assert_eq!(
            DisplayOrigin::OverlayString {
                overlay_id: Value::fixnum(1),
                anchor_charpos: CharPos0::new(4),
                kind: OverlayStringKind::Before,
            }
            .default_base_face_policy(),
            BaseFacePolicy::OverlayStringAtAnchor
        );
        assert_eq!(
            DisplayOrigin::DisplayPropertyString {
                anchor_charpos: CharPos0::new(5),
                source: DisplayPropertySource::TextProperty,
            }
            .default_base_face_policy(),
            BaseFacePolicy::DisplayPropertyUnderlyingFace
        );
        assert_eq!(
            DisplayOrigin::LinePrefix {
                anchor_charpos: CharPos0::new(6),
            }
            .default_base_face_policy(),
            BaseFacePolicy::DefaultFace
        );
        assert_eq!(
            DisplayOrigin::WrapPrefix {
                anchor_charpos: CharPos0::new(7),
            }
            .default_base_face_policy(),
            BaseFacePolicy::DefaultFace
        );
        assert_eq!(
            DisplayOrigin::Minibuffer.default_base_face_policy(),
            BaseFacePolicy::DefaultFace
        );
        assert_eq!(
            DisplayOrigin::EchoArea.default_base_face_policy(),
            BaseFacePolicy::DefaultFace
        );
        assert_eq!(
            DisplayOrigin::Posframe.default_base_face_policy(),
            BaseFacePolicy::DefaultFace
        );
        assert_eq!(
            DisplayOrigin::ModeLine { selected: true }.default_base_face_policy(),
            BaseFacePolicy::FixedBasicFace(BasicFaceId::ModeLineActive)
        );
        assert_eq!(
            DisplayOrigin::ModeLine { selected: false }.default_base_face_policy(),
            BaseFacePolicy::FixedBasicFace(BasicFaceId::ModeLineInactive)
        );
        assert_eq!(
            DisplayOrigin::HeaderLine { selected: true }.default_base_face_policy(),
            BaseFacePolicy::FixedBasicFace(BasicFaceId::HeaderLineActive)
        );
        assert_eq!(
            DisplayOrigin::HeaderLine { selected: false }.default_base_face_policy(),
            BaseFacePolicy::FixedBasicFace(BasicFaceId::HeaderLineInactive)
        );
        assert_eq!(
            DisplayOrigin::TabLine.default_base_face_policy(),
            BaseFacePolicy::FixedBasicFace(BasicFaceId::TabLine)
        );
        assert_eq!(
            DisplayOrigin::TabBar.default_base_face_policy(),
            BaseFacePolicy::FixedBasicFace(BasicFaceId::TabBar)
        );
    }

    #[test]
    fn display_origin_derives_chrome_row_roles() {
        assert_eq!(
            DisplayOrigin::ModeLine { selected: true }.glyph_row_role(),
            Some(GlyphRowRole::ModeLine)
        );
        assert_eq!(
            DisplayOrigin::HeaderLine { selected: true }.glyph_row_role(),
            Some(GlyphRowRole::HeaderLine)
        );
        assert_eq!(
            DisplayOrigin::TabLine.glyph_row_role(),
            Some(GlyphRowRole::TabLine)
        );
        assert_eq!(
            DisplayOrigin::TabBar.glyph_row_role(),
            Some(GlyphRowRole::TabBar)
        );
        assert_eq!(
            DisplayOrigin::Minibuffer.glyph_row_role(),
            Some(GlyphRowRole::Minibuffer)
        );
        assert_eq!(
            DisplayOrigin::EchoArea.glyph_row_role(),
            Some(GlyphRowRole::Minibuffer)
        );
        assert_eq!(
            DisplayOrigin::Posframe.glyph_row_role(),
            Some(GlyphRowRole::Text)
        );
        assert_eq!(
            DisplayOrigin::BufferText {
                charpos: CharPos0::new(0),
            }
            .glyph_row_role(),
            None
        );
    }
}
