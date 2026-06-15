use crate::display_face_policy::BaseFacePolicy;
use neomacs_display_protocol::face::BasicFaceId;
use neovm_core::buffer::CharPos0;
use neovm_core::emacs_core::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OverlayStringKind {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum DisplayPropertySource {
    TextProperty,
    Overlay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
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
    ModeLine,
    HeaderLine,
    TabLine,
    TabBar,
    EchoArea,
}

impl DisplayOrigin {
    pub(crate) fn default_base_face_policy(self) -> BaseFacePolicy {
        match self {
            Self::BufferText { .. } => BaseFacePolicy::BufferFaceIncludingOverlays,
            Self::OverlayString { .. } => BaseFacePolicy::OverlayStringAtAnchor,
            Self::DisplayPropertyString { .. } => BaseFacePolicy::DisplayPropertyUnderlyingFace,
            Self::LinePrefix { .. } | Self::WrapPrefix { .. } | Self::EchoArea => {
                BaseFacePolicy::DefaultFace
            }
            Self::ModeLine => BaseFacePolicy::FixedBasicFace(BasicFaceId::ModeLineActive),
            Self::HeaderLine => BaseFacePolicy::FixedBasicFace(BasicFaceId::HeaderLineActive),
            Self::TabLine => BaseFacePolicy::FixedBasicFace(BasicFaceId::TabLine),
            Self::TabBar => BaseFacePolicy::FixedBasicFace(BasicFaceId::TabBar),
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
        let _ = DisplayOrigin::ModeLine;
        let _ = DisplayOrigin::HeaderLine;
        let _ = DisplayOrigin::TabLine;
        let _ = DisplayOrigin::TabBar;
        let _ = DisplayOrigin::EchoArea;
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
            DisplayOrigin::EchoArea.default_base_face_policy(),
            BaseFacePolicy::DefaultFace
        );
        assert_eq!(
            DisplayOrigin::ModeLine.default_base_face_policy(),
            BaseFacePolicy::FixedBasicFace(BasicFaceId::ModeLineActive)
        );
        assert_eq!(
            DisplayOrigin::HeaderLine.default_base_face_policy(),
            BaseFacePolicy::FixedBasicFace(BasicFaceId::HeaderLineActive)
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
}
