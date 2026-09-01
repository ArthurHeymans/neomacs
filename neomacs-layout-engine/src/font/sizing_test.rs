use super::{FontSizing, LogicalFontScale, points_to_layout_pixels};

#[test]
fn cocoa_points_are_cocoa_logical_units() {
    let sizing = FontSizing::new(LogicalFontScale::GnuCocoaPoint);

    assert_eq!(sizing.face_height_to_layout_pixels(100), 10.0);
    assert_eq!(sizing.face_height_to_layout_pixels(120), 12.0);
    assert_eq!(sizing.layout_dpi(), 72.27);
}

#[test]
fn platform_point_rules_are_explicit_and_distinct() {
    let cocoa = FontSizing::new(LogicalFontScale::GnuCocoaPoint);
    let windows = FontSizing::new(LogicalFontScale::WindowsDip);
    let x11 = FontSizing::new(LogicalFontScale::X11 {
        effective_dpi: 100.0,
    });

    assert_eq!(cocoa.face_height_to_layout_pixels(100), 10.0);
    assert_eq!(windows.face_height_to_layout_pixels(100), 13.0);
    assert_eq!(x11.face_height_to_layout_pixels(100), 14.0);
}

#[test]
fn point_conversion_uses_gnu_printer_points() {
    assert_eq!(points_to_layout_pixels(22.0, 100.0), 30.0);
}
