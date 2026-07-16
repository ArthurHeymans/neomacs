//! Parse tests for the `(surface :id N :width W :height H)` display spec.

use super::*;

#[test]
fn parse_surface_layout_full_spec() {
    let _eval = neovm_core::emacs_core::Context::new();
    let layout = parse_display_surface_layout(&Value::list(vec![
        Value::symbol("surface"),
        Value::symbol(":id"),
        Value::fixnum(7),
        Value::symbol(":width"),
        Value::fixnum(320),
        Value::symbol(":height"),
        Value::fixnum(120),
    ]))
    .expect("surface layout");
    assert_eq!(
        layout,
        DisplaySurfaceLayout {
            surface_id: 7,
            width: 320.0,
            height: 120.0,
        }
    );
}

#[test]
fn parse_surface_layout_requires_id() {
    let _eval = neovm_core::emacs_core::Context::new();
    assert!(
        parse_display_surface_layout(&Value::list(vec![
            Value::symbol("surface"),
            Value::symbol(":width"),
            Value::fixnum(320),
        ]))
        .is_none()
    );
}

#[test]
fn parse_surface_layout_defaults_missing_dimensions() {
    let _eval = neovm_core::emacs_core::Context::new();
    let layout = parse_display_surface_layout(&Value::list(vec![
        Value::symbol("surface"),
        Value::symbol(":id"),
        Value::fixnum(1),
    ]))
    .expect("surface layout");
    assert_eq!(layout.width, 64.0);
    assert_eq!(layout.height, 64.0);
}

#[test]
fn parse_surface_layout_rejects_other_heads() {
    let _eval = neovm_core::emacs_core::Context::new();
    assert!(
        parse_display_surface_layout(&Value::list(vec![
            Value::symbol("video"),
            Value::symbol(":id"),
            Value::fixnum(1),
        ]))
        .is_none()
    );
}
