use crate::display_face_policy::BaseFacePolicy;
use crate::display_origin::{DisplayOrigin, DisplayPropertySource, OverlayStringKind};
use neomacs_display_protocol::face::BasicFaceId;
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

    pub(crate) fn buffer_text(start: CharPos0, end: CharPos0) -> Self {
        Self::buffer_span(
            start,
            end,
            DisplayOrigin::BufferText { charpos: start },
            BaseFacePolicy::BufferFaceIncludingOverlays,
        )
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

    pub(crate) fn display_property_string(
        value: Value,
        anchor_charpos: CharPos0,
        source: DisplayPropertySource,
    ) -> Self {
        Self::lisp_string(
            value,
            DisplayOrigin::DisplayPropertyString {
                anchor_charpos,
                source,
            },
            BaseFacePolicy::DisplayPropertyUnderlyingFace,
        )
    }

    pub(crate) fn line_prefix(value: Value, anchor_charpos: CharPos0) -> Self {
        Self::lisp_string(
            value,
            DisplayOrigin::LinePrefix { anchor_charpos },
            BaseFacePolicy::DefaultFace,
        )
    }

    pub(crate) fn wrap_prefix(value: Value, anchor_charpos: CharPos0) -> Self {
        Self::lisp_string(
            value,
            DisplayOrigin::WrapPrefix { anchor_charpos },
            BaseFacePolicy::DefaultFace,
        )
    }

    pub(crate) fn mode_line(value: Value, selected_window: bool) -> Self {
        let face = if selected_window {
            BasicFaceId::ModeLineActive
        } else {
            BasicFaceId::ModeLineInactive
        };
        Self::lisp_string(
            value,
            DisplayOrigin::ModeLine,
            BaseFacePolicy::FixedBasicFace(face),
        )
    }

    pub(crate) fn header_line(value: Value, selected_window: bool) -> Self {
        let face = if selected_window {
            BasicFaceId::HeaderLineActive
        } else {
            BasicFaceId::HeaderLineInactive
        };
        Self::lisp_string(
            value,
            DisplayOrigin::HeaderLine,
            BaseFacePolicy::FixedBasicFace(face),
        )
    }

    pub(crate) fn tab_line(value: Value) -> Self {
        Self::lisp_string(
            value,
            DisplayOrigin::TabLine,
            BaseFacePolicy::FixedBasicFace(BasicFaceId::TabLine),
        )
    }

    pub(crate) fn tab_bar(value: Value) -> Self {
        Self::lisp_string(
            value,
            DisplayOrigin::TabBar,
            BaseFacePolicy::FixedBasicFace(BasicFaceId::TabBar),
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

    #[test]
    fn display_text_fragment_builds_display_property_string_fragment() {
        let _ctx = Context::new();
        let value = Value::string("replacement");
        let fragment = DisplayTextFragment::display_property_string(
            value,
            CharPos0::new(2),
            DisplayPropertySource::TextProperty,
        );

        assert_eq!(fragment.storage, DisplayTextStorage::LispString(value));
        assert_eq!(
            fragment.origin,
            DisplayOrigin::DisplayPropertyString {
                anchor_charpos: CharPos0::new(2),
                source: DisplayPropertySource::TextProperty,
            }
        );
        assert_eq!(
            fragment.base_face_policy,
            BaseFacePolicy::DisplayPropertyUnderlyingFace
        );
    }

    #[test]
    fn display_text_fragment_builds_prefix_fragments() {
        let _ctx = Context::new();
        let line_value = Value::string("line");
        let line = DisplayTextFragment::line_prefix(line_value, CharPos0::new(3));
        assert_eq!(line.storage, DisplayTextStorage::LispString(line_value));
        assert_eq!(
            line.origin,
            DisplayOrigin::LinePrefix {
                anchor_charpos: CharPos0::new(3)
            }
        );
        assert_eq!(line.base_face_policy, BaseFacePolicy::DefaultFace);

        let wrap_value = Value::string("wrap");
        let wrap = DisplayTextFragment::wrap_prefix(wrap_value, CharPos0::new(5));
        assert_eq!(wrap.storage, DisplayTextStorage::LispString(wrap_value));
        assert_eq!(
            wrap.origin,
            DisplayOrigin::WrapPrefix {
                anchor_charpos: CharPos0::new(5)
            }
        );
        assert_eq!(wrap.base_face_policy, BaseFacePolicy::DefaultFace);
    }

    #[test]
    fn display_text_fragment_builds_buffer_text_fragment() {
        let fragment = DisplayTextFragment::buffer_text(CharPos0::new(4), CharPos0::new(5));
        assert_eq!(
            fragment.storage,
            DisplayTextStorage::BufferSpan {
                start: CharPos0::new(4),
                end: CharPos0::new(5)
            }
        );
        assert_eq!(
            fragment.origin,
            DisplayOrigin::BufferText {
                charpos: CharPos0::new(4)
            }
        );
        assert_eq!(
            fragment.base_face_policy,
            BaseFacePolicy::BufferFaceIncludingOverlays
        );
    }

    #[test]
    fn display_text_fragment_builds_chrome_fragments() {
        use neomacs_display_protocol::face::BasicFaceId;

        let _ctx = Context::new();
        let mode_value = Value::string("mode");
        let mode = DisplayTextFragment::mode_line(mode_value, true);
        assert_eq!(mode.storage, DisplayTextStorage::LispString(mode_value));
        assert_eq!(mode.origin, DisplayOrigin::ModeLine);
        assert_eq!(
            mode.base_face_policy,
            BaseFacePolicy::FixedBasicFace(BasicFaceId::ModeLineActive)
        );

        let inactive_mode_value = Value::string("inactive");
        let inactive_mode = DisplayTextFragment::mode_line(inactive_mode_value, false);
        assert_eq!(inactive_mode.origin, DisplayOrigin::ModeLine);
        assert_eq!(
            inactive_mode.base_face_policy,
            BaseFacePolicy::FixedBasicFace(BasicFaceId::ModeLineInactive)
        );

        let header_value = Value::string("header");
        let header = DisplayTextFragment::header_line(header_value, true);
        assert_eq!(header.storage, DisplayTextStorage::LispString(header_value));
        assert_eq!(header.origin, DisplayOrigin::HeaderLine);
        assert_eq!(
            header.base_face_policy,
            BaseFacePolicy::FixedBasicFace(BasicFaceId::HeaderLineActive)
        );

        let tab_line_value = Value::string("tabs");
        let tab_line = DisplayTextFragment::tab_line(tab_line_value);
        assert_eq!(
            tab_line.storage,
            DisplayTextStorage::LispString(tab_line_value)
        );
        assert_eq!(tab_line.origin, DisplayOrigin::TabLine);
        assert_eq!(
            tab_line.base_face_policy,
            BaseFacePolicy::FixedBasicFace(BasicFaceId::TabLine)
        );

        let tab_bar_value = Value::string("frame tabs");
        let tab_bar = DisplayTextFragment::tab_bar(tab_bar_value);
        assert_eq!(
            tab_bar.storage,
            DisplayTextStorage::LispString(tab_bar_value)
        );
        assert_eq!(tab_bar.origin, DisplayOrigin::TabBar);
        assert_eq!(
            tab_bar.base_face_policy,
            BaseFacePolicy::FixedBasicFace(BasicFaceId::TabBar)
        );
    }
}
