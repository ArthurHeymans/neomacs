use super::*;
use crate::neovm_bridge::FaceResolver;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;

fn base_face() -> crate::neovm_bridge::ResolvedFace {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    resolver.default_face().clone()
}

#[test]
fn display_row_request_accepts_window_chrome_roles() {
    let _eval = Context::new();
    for role in [
        GlyphRowRole::ModeLine,
        GlyphRowRole::HeaderLine,
        GlyphRowRole::TabLine,
    ] {
        let request = DisplayRowRequest {
            role,
            x: 1.0,
            y: 2.0,
            width: 80.0,
            height: 16.0,
            window_id: 7,
            matrix_row: Some(0),
            base_face: base_face(),
            source: DisplaySource::PropertizedString(Value::string("row")),
        };

        assert_eq!(request.role, role);
        assert_eq!(request.window_id, 7);
        assert_eq!(request.matrix_row, Some(0));
    }
}

#[test]
fn display_row_request_accepts_frame_and_minibuffer_roles() {
    for role in [GlyphRowRole::TabBar, GlyphRowRole::Minibuffer] {
        let request = DisplayRowRequest {
            role,
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 20.0,
            window_id: 0,
            matrix_row: None,
            base_face: base_face(),
            source: DisplaySource::PlainString("plain".to_string()),
        };

        assert_eq!(request.role, role);
        assert!(request.matrix_row.is_none());
    }
}

#[test]
fn display_source_keeps_plain_and_propertized_inputs_distinct() {
    let _eval = Context::new();
    let plain = DisplaySource::PlainString("plain".to_string());
    let propertized = DisplaySource::PropertizedString(Value::string("prop"));

    match plain {
        DisplaySource::PlainString(text) => assert_eq!(text, "plain"),
        DisplaySource::PropertizedString(_) => panic!("expected plain source"),
    }

    match propertized {
        DisplaySource::PropertizedString(value) => {
            assert_eq!(value.as_runtime_string_owned().as_deref(), Some("prop"));
        }
        DisplaySource::PlainString(_) => panic!("expected propertized source"),
    }
}
