//! Divergence tests: image manipulation, image-cache, image descriptors.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_image_descriptors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'image-type)
  (fboundp 'image-type-from-file-header)
  (fboundp 'image-type-from-file-name)
  (fboundp 'image-type-available-p))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_image_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'image-property)
  (fboundp 'image-size)
  (fboundp 'image-mask)
  (fboundp 'image-extension-data))"#,
        expect_test::expect![[r#""OK (t t nil nil)""#]],
    );
}

#[test]
fn divergence_image_cache_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'clear-image-cache)
  (fboundp 'image-flush)
  (boundp 'image-cache-eviction-delay)
  (boundp 'image-cache-size))"#,
        expect_test::expect![[r#""OK (t t t nil)""#]],
    );
}

#[test]
fn divergence_image_formatting() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'image-format)
  (fboundp 'image-animate)
  (fboundp 'image-animate-timer)
  (fboundp 'image-multi-frame-p))"#,
        expect_test::expect![[r#""OK (nil t t t)""#]],
    );
}

#[test]
fn divergence_xpm_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (member 'xpm image-types)
  (fboundp 'xpm-generate-message)
  (member 'pbm image-types))"#,
        expect_test::expect![[r#""OK ((xpm xbm pbm) nil (pbm))""#]],
    );
}

#[test]
fn divergence_image_magick() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'imagemagick-types)
  (fboundp 'imagemagick-enabled-p)
  (member 'imagemagick image-types))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
}

#[test]
fn divergence_image_scaling() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'image-scaling-factor)
  (boundp 'image-resize-margin)
  (fboundp 'image-compute-scaling-factor))"#,
        expect_test::expect![[r#""OK (t nil t)""#]],
    );
}

#[test]
fn divergence_image_size_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'image-size-in-characters)
  (fboundp 'image-get-display-property)
  (boundp 'max-image-size))"#,
        expect_test::expect![[r#""OK (nil nil t)""#]],
    );
}

#[test]
fn divergence_fringe_bitmaps() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'define-fringe-bitmap)
  (fboundp 'destroy-fringe-bitmap)
  (fboundp 'set-fringe-bitmap-face)
  (fboundp 'fringe-bitmaps-at-pos))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'window-divider-default-places)
  (boundp 'window-divider-default-bottom-width)
  (boundp 'window-divider-default-right-width)
  (fboundp 'window-divider-mode))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}
