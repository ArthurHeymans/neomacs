use super::*;
use crate::display_item::{
    DisplayImageItem, DisplayItemKind, DisplayLength, DisplayLengthExpr, DisplayLengthSymbol,
    DisplayStretch, DisplayStretchWidth, DisplayVideoItem, DisplayXwidgetItem,
};
use neovm_core::emacs_core::{Context, Value};

#[test]
fn classify_display_property_separates_replacements_from_text_modifiers() {
    let eval = Context::new();
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let xwidget = Value::make_xwidget(
        Value::symbol("webkit"),
        Value::string("Title"),
        Value::make_buffer(buffer_id),
        96,
        54,
        1234,
    );

    assert_eq!(
        classify_display_property(Value::string("replacement")).replacement,
        Some(DisplayReplacementProperty::String)
    );
    assert_eq!(
        classify_display_property(Value::list(vec![
            Value::symbol("space"),
            Value::keyword(":align-to"),
            Value::list(vec![
                Value::symbol("-"),
                Value::symbol("right"),
                Value::fixnum(2),
            ]),
        ]))
        .replacement,
        Some(DisplayReplacementProperty::Space(DisplayStretch {
            width: DisplayStretchWidth::AlignTo(DisplayLengthExpr::Sub(vec![
                DisplayLengthExpr::Symbol(DisplayLengthSymbol::Right),
                DisplayLengthExpr::Em(2.0),
            ])),
            height: None,
            ascent: None,
        }))
    );
    assert_eq!(
        classify_display_property(Value::list(vec![Value::symbol("image")])).replacement,
        Some(DisplayReplacementProperty::Image)
    );
    assert_eq!(
        classify_display_property(Value::list(vec![Value::symbol("video")])).replacement,
        Some(DisplayReplacementProperty::Video)
    );
    assert_eq!(
        classify_display_property(Value::list(vec![Value::symbol("webkit")])).replacement,
        Some(DisplayReplacementProperty::Webkit)
    );
    assert_eq!(
        classify_display_property(Value::list(vec![
            Value::symbol("xwidget"),
            Value::keyword("xwidget"),
            xwidget,
        ]))
        .replacement,
        Some(DisplayReplacementProperty::Xwidget(DisplayXwidgetItem {
            xwidget_id: 1234,
            width: 96.0,
            height: 54.0,
        }))
    );

    assert_eq!(
        classify_display_property(Value::list(vec![
            Value::symbol("raise"),
            Value::make_float(0.25),
        ]))
        .modifiers,
        DisplayTextPropertyModifiers {
            raise: Some(0.25),
            height: None,
        }
    );
    assert_eq!(
        classify_display_property(Value::list(vec![
            Value::keyword(":raise"),
            Value::make_float(0.2),
            Value::keyword(":height"),
            Value::make_float(1.4),
        ]))
        .modifiers,
        DisplayTextPropertyModifiers {
            raise: Some(0.2),
            height: Some(1.4),
        }
    );
}

#[test]
fn classify_display_property_parses_space_width_height_and_ascent() {
    let _eval = Context::new();

    assert_eq!(
        classify_display_property(Value::list(vec![
            Value::symbol("space"),
            Value::keyword(":width"),
            Value::fixnum(3),
            Value::keyword(":height"),
            Value::fixnum(2),
            Value::keyword(":ascent"),
            Value::fixnum(50),
        ]))
        .replacement,
        Some(DisplayReplacementProperty::Space(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Em(3.0)),
            height: Some(DisplayLength::Em(2.0)),
            ascent: Some(DisplayLength::Em(50.0)),
        }))
    );
}

#[test]
fn classify_display_property_keeps_space_replacement_without_explicit_width() {
    let _eval = Context::new();

    assert_eq!(
        classify_display_property(Value::list(vec![
            Value::symbol("space"),
            Value::keyword(":height"),
            Value::fixnum(2),
        ]))
        .replacement,
        Some(DisplayReplacementProperty::Space(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Em(1.0)),
            height: Some(DisplayLength::Em(2.0)),
            ascent: None,
        }))
    );
}

#[test]
fn display_replacement_property_accepts_only_matching_resolved_media_items() {
    let image_item = DisplayItemKind::Image(DisplayImageItem {
        image_id: 1,
        width: 10.0,
        height: 20.0,
    });
    let video_item = DisplayItemKind::Video(DisplayVideoItem {
        video_id: 2,
        width: 30.0,
        height: 40.0,
        loop_count: 0,
        autoplay: false,
    });
    let xwidget_item = DisplayItemKind::Xwidget(DisplayXwidgetItem {
        xwidget_id: 3,
        width: 50.0,
        height: 60.0,
    });

    assert!(DisplayReplacementProperty::Image.accepts_resolved_media_item(&image_item));
    assert!(!DisplayReplacementProperty::Image.accepts_resolved_media_item(&video_item));
    assert!(DisplayReplacementProperty::Video.accepts_resolved_media_item(&video_item));
    assert!(!DisplayReplacementProperty::Video.accepts_resolved_media_item(&image_item));
    assert!(DisplayReplacementProperty::Webkit.accepts_resolved_media_item(&xwidget_item));
    assert!(!DisplayReplacementProperty::Webkit.accepts_resolved_media_item(&image_item));
}
