use crate::display_face_policy::BaseFacePolicy;
use crate::display_origin::{DisplayOrigin, OverlayStringKind};
use neovm_core::buffer::CharPos0;
use neovm_core::emacs_core::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum DisplayTextStorage {
    BufferSpan { start: CharPos0, end: CharPos0 },
    LispString(Value),
    Static(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) struct DisplayTextFragment {
    pub(crate) storage: DisplayTextStorage,
    pub(crate) origin: DisplayOrigin,
    pub(crate) base_face_policy: BaseFacePolicy,
}

#[allow(dead_code)]
impl DisplayTextFragment {
    pub(crate) fn buffer_span(
        start: CharPos0,
        end: CharPos0,
        origin: DisplayOrigin,
        base_face_policy: BaseFacePolicy,
    ) -> Self {
        Self {
            storage: DisplayTextStorage::BufferSpan { start, end },
            origin,
            base_face_policy,
        }
    }

    pub(crate) fn lisp_string(
        value: Value,
        origin: DisplayOrigin,
        base_face_policy: BaseFacePolicy,
    ) -> Self {
        Self {
            storage: DisplayTextStorage::LispString(value),
            origin,
            base_face_policy,
        }
    }

    pub(crate) fn static_text(
        value: &'static str,
        origin: DisplayOrigin,
        base_face_policy: BaseFacePolicy,
    ) -> Self {
        Self {
            storage: DisplayTextStorage::Static(value),
            origin,
            base_face_policy,
        }
    }

    pub(crate) fn overlay_string(
        value: Value,
        overlay_id: Value,
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    ) -> Self {
        Self::lisp_string(
            value,
            DisplayOrigin::OverlayString {
                overlay_id,
                anchor_charpos,
                kind,
            },
            BaseFacePolicy::OverlayStringAtAnchor,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display_face_policy::BaseFacePolicy;
    use crate::display_origin::{DisplayOrigin, DisplayPropertySource, OverlayStringKind};
    use neovm_core::buffer::CharPos0;
    use neovm_core::emacs_core::{Context, Value};

    #[test]
    fn display_text_fragment_models_buffer_lisp_static_and_display_property_text() {
        let buffer = DisplayTextFragment::buffer_span(
            CharPos0::new(1),
            CharPos0::new(4),
            DisplayOrigin::BufferText {
                charpos: CharPos0::new(1),
            },
            BaseFacePolicy::BufferFaceIncludingOverlays,
        );
        assert_eq!(
            buffer.storage,
            DisplayTextStorage::BufferSpan {
                start: CharPos0::new(1),
                end: CharPos0::new(4)
            }
        );

        let _ctx = Context::new();
        let string_value = Value::string("candidate");
        let lisp = DisplayTextFragment::lisp_string(
            string_value,
            DisplayOrigin::OverlayString {
                overlay_id: Value::fixnum(1),
                anchor_charpos: CharPos0::new(4),
                kind: OverlayStringKind::Before,
            },
            BaseFacePolicy::OverlayStringAtAnchor,
        );
        assert_eq!(lisp.storage, DisplayTextStorage::LispString(string_value));

        let static_text = DisplayTextFragment::static_text(
            "mode",
            DisplayOrigin::ModeLine,
            BaseFacePolicy::FixedBasicFace(
                neomacs_display_protocol::face::BasicFaceId::ModeLineActive,
            ),
        );
        assert_eq!(static_text.storage, DisplayTextStorage::Static("mode"));

        let display_property = DisplayTextFragment::lisp_string(
            Value::string("replacement"),
            DisplayOrigin::DisplayPropertyString {
                anchor_charpos: CharPos0::new(2),
                source: DisplayPropertySource::TextProperty,
            },
            BaseFacePolicy::DisplayPropertyUnderlyingFace,
        );
        assert_eq!(
            display_property.base_face_policy,
            BaseFacePolicy::DisplayPropertyUnderlyingFace
        );
    }

    #[test]
    fn display_text_fragment_builds_overlay_string_fragment() {
        let _ctx = Context::new();
        let value = Value::string("candidate");
        let fragment = DisplayTextFragment::overlay_string(
            value,
            Value::fixnum(7),
            CharPos0::new(4),
            OverlayStringKind::After,
        );

        assert_eq!(fragment.storage, DisplayTextStorage::LispString(value));
        assert_eq!(
            fragment.origin,
            DisplayOrigin::OverlayString {
                overlay_id: Value::fixnum(7),
                anchor_charpos: CharPos0::new(4),
                kind: OverlayStringKind::After,
            }
        );
        assert_eq!(
            fragment.base_face_policy,
            BaseFacePolicy::OverlayStringAtAnchor
        );
    }
}
