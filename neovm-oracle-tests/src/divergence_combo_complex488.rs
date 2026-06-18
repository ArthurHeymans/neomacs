/// Batch 488: frame-config, frame-selected, frame-parameter deep, frame-geometry.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx488_frame_config_register() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (frame-configuration-to-register ?f)
  (jump-to-register ?f)
  (framep f))
"##,
    );
}

#[test]
fn div_cx488_frame_parameters_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (list (frame-parameter f 'name)
        (frame-parameter f 'width)
        (frame-parameter f 'height)))
"##,
    );
}

#[test]
fn div_cx488_frame_text_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (list (frame-text-width f) (frame-text-height f)))
"##,
    );
}

#[test]
fn div_cx488_frame_pixel_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (list (frame-pixel-width f) (frame-pixel-height f)))
"##,
    );
}

#[test]
fn div_cx488_frame_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (condition-case e
      (frame-position f)
    (error (car e))))
"##,
    );
}

#[test]
fn div_cx488_frame_iconified() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (list (frame-visible-p f) (frame-iconified-p f)))
"##,
    );
}

#[test]
fn div_cx488_frame_alpha() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (condition-case e
      (frame-parameter f 'alpha)
    (error (car e))))
"##,
    );
}

#[test]
fn div_cx488_frame_size_hints() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (condition-case e
      (frame-size-hints-pixelwise f)
    (error (car e))))
"##,
    );
}

#[test]
fn div_cx488_frame_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (list (frame-display f) (frame-terminal f)))
"##,
    );
}

#[test]
fn div_cx488_frame_live_child() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (list (frame-root-window f) (frame-first-window f)))
"##,
    );
}

#[test]
fn div_cx488_frame_parameter_names() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (selected-frame)))
  (list (frame-parameters-keys f) (frame-parameter-names f)))
"##,
    );
}

#[test]
fn div_cx488_frame_restack() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (frame-restack (selected-frame) (selected-frame) nil)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx488_frame_after_make() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (make-frame-invisible (selected-frame))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx488_frame_visible() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(frame-visible-p (selected-frame))
"##,
    );
}

#[test]
fn div_cx488_frame_raise_lower() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (raise-frame (selected-frame))
  (error (car e)))
"##,
    );
}
