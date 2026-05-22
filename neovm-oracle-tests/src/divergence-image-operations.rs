//! Divergence tests: image manipulation, image-cache, image descriptors.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_image_descriptors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'image-type)
  (fboundp 'image-type-from-file-header)
  (fboundp 'image-type-from-file-name)
  (fboundp 'image-type-available-p))"#,
    );
}

#[test]
fn divergence_image_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'image-property)
  (fboundp 'image-size)
  (fboundp 'image-mask)
  (fboundp 'image-extension-data))"#,
    );
}

#[test]
fn divergence_image_cache_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'clear-image-cache)
  (fboundp 'image-flush)
  (boundp 'image-cache-eviction-delay)
  (boundp 'image-cache-size))"#,
    );
}

#[test]
fn divergence_image_formatting() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'image-format)
  (fboundp 'image-animate)
  (fboundp 'image-animate-timer)
  (fboundp 'image-multi-frame-p))"#,
    );
}

#[test]
fn divergence_xpm_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (member 'xpm image-types)
  (fboundp 'xpm-generate-message)
  (member 'pbm image-types))"#,
    );
}

#[test]
fn divergence_image_magick() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'imagemagick-types)
  (fboundp 'imagemagick-enabled-p)
  (member 'imagemagick image-types))"#,
    );
}

#[test]
fn divergence_image_scaling() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'image-scaling-factor)
  (boundp 'image-resize-margin)
  (fboundp 'image-compute-scaling-factor))"#,
    );
}

#[test]
fn divergence_image_size_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'image-size-in-characters)
  (fboundp 'image-get-display-property)
  (boundp 'max-image-size))"#,
    );
}

#[test]
fn divergence_fringe_bitmaps() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'define-fringe-bitmap)
  (fboundp 'destroy-fringe-bitmap)
  (fboundp 'set-fringe-bitmap-face)
  (fboundp 'fringe-bitmaps-at-pos))"#,
    );
}

#[test]
fn divergence_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'window-divider-default-places)
  (boundp 'window-divider-default-bottom-width)
  (boundp 'window-divider-default-right-width)
  (fboundp 'window-divider-mode))"#,
    );
}
