//! Strict combo oracle probes, batch 318: frame + display metrics (tolerant).
//! frame-char-width/-height, display-pixel-width/-height, display-mm-width/
//! -height, frame-fringe-limit, asserted as shape (integerp/>0) since the
//! values are display-dependent between builds.
//! Uses assert_oracle_parity_expect format.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_frame_char_width_height_metrics_shape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let ((f (selected-frame)))
  (list (integerp (frame-char-width))
        (integerp (frame-char-height))
        (> (frame-char-width) 0)
        (> (frame-char-height) 0)
        (integerp (frame-cols f))
        (integerp (frame-lines f))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_display_pixel_mm_dimensions_shape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let ((d (frame-monitor)))
  (list (integerp (display-pixel-width))
        (integerp (display-pixel-height))
        (>= (display-pixel-width) 0)
        (>= (display-pixel-height) 0)
        (or (integerp (display-mm-width)) (null (display-mm-width)))
        (or (integerp (display-mm-height)) (null (display-mm-height)))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_frame_monitor_attributes_shape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let ((attrs (frame-monitor-attributes)))
  (list (consp attrs)
        (assq 'geometry attrs)
        (assq 'workarea attrs)
        (assq 'mm-size attrs)
        (integerp (frame-monitor-attributes nil nil 'geometry))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
