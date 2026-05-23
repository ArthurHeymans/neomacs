//! Divergence tests: face, color, font specification deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_face_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'face-attribute)
  (fboundp 'face-all-attributes)
  (fboundp 'set-face-attribute)
  (fboundp 'face-foreground)
  (fboundp 'face-background)
  (fboundp 'face-font)) "#,
    );
}

#[test]
fn divergence_face_underline_box() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'set-face-underline)
  (fboundp 'face-underline-p)
  (fboundp 'set-face-box)
  (fboundp 'face-inverse-video-p)
  (fboundp 'set-face-inverse-video)
  (fboundp 'face-stipple)) "#,
    );
}

#[test]
fn divergence_face_realized() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'face-id)
  (fboundp 'face-name)
  (fboundp 'face-documentation)
  (fboundp 'list-faces-display)
  (fboundp 'describe-face)
  (fboundp 'face-list)) "#,
    );
}

#[test]
fn divergence_color_names() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'color-name-to-hex)
  (fboundp 'color-values)
  (fboundp 'color-values-from-color-spec)
  (fboundp 'defined-colors)
  (fboundp 'color-supported-p)
  (listp (defined-colors))) "#,
    );
}

#[test]
fn divergence_color_rgb_hsv() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'color-rgb-to-hsv)
  (fboundp 'color-hsv-to-rgb)
  (fboundp 'color-complement-hex)
  (fboundp 'color-gradient)
  (fboundp 'color-distance)) "#,
    );
}

#[test]
fn divergence_font_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'font-spec)
  (fboundp 'font-get)
  (fboundp 'font-put)
  (fboundp 'font-xlfd-name)
  (fboundp 'list-fonts)
  (fboundp 'list-families)) "#,
    );
}

#[test]
fn divergence_font_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'font-match)
  (fboundp 'font-open)
  (fboundp 'font-close)
  (fboundp 'font-info)
  (fboundp 'font-at)) "#,
    );
}

#[test]
fn divergence_cursor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'cursor-type)
  (member cursor-type '(t box hollow bar hbar nil))
  (boundp 'blink-cursor-blinks)
  (boundp 'blink-cursor-interval)
  (boundp 'blink-cursor-delay)) "#,
    );
}

#[test]
fn divergence_mouse_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'mouse-highlight)
  (boundp 'mouse-yank-at-point)
  (boundp 'focus-follows-mouse)
  (booleanp focus-follows-mouse)) "#,
    );
}

#[test]
fn divergence_theme_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'custom-theme-set-faces)
  (fboundp 'custom-theme-set-variables)
  (fboundp 'custom-declare-theme)
  (fboundp 'custom-check-theme)
  (fboundp 'custom-theme-p)) "#,
    );
}
