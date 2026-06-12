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
    EchoArea,
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
        let _ = DisplayOrigin::EchoArea;
    }
}
