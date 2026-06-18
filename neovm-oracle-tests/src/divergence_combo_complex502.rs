/// Batch 502: set-buffer-multibyte characterization — various raw byte patterns.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx502_multibyte_raw_trailing_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200))
  (insert "ABC")
  (set-buffer-multibyte t)
  (list (buffer-string) (length (buffer-string)) (point-max)))"##,
    );
}

#[test]
fn div_cx502_multibyte_raw_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 202))
  (set-buffer-multibyte t)
  (list (buffer-string) (length (buffer-string)) (point-max)))"##,
    );
}

#[test]
fn div_cx502_multibyte_ascii_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert "ABCDE")
  (set-buffer-multibyte t)
  (list (buffer-string) (length (buffer-string)) (point-max)))"##,
    );
}

#[test]
fn div_cx502_multibyte_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (set-buffer-multibyte t)
  (list (buffer-string) (length (buffer-string)) (point-max)))"##,
    );
}

#[test]
fn div_cx502_multibyte_interleaved() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 65 201 66 202 67))
  (set-buffer-multibyte t)
  (list (buffer-string) (length (buffer-string)) (point-max)))"##,
    );
}

#[test]
fn div_cx502_multibyte_larger_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 202 203 204 65 66 67 68 69))
  (set-buffer-multibyte t)
  (list (buffer-string) (length (buffer-string)) (point-max)))"##,
    );
}

#[test]
fn div_cx502_multibyte_marker_preserved() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 65 66))
  (let ((m (set-marker (make-marker) 2)))
    (set-buffer-multibyte t)
    (list (marker-position m) (marker-buffer m))))"##,
    );
}

#[test]
fn div_cx502_multibyte_insert_after_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201))
  (set-buffer-multibyte t)
  (insert "EXTRA")
  (buffer-string))"##,
    );
}

#[test]
fn div_cx502_multibyte_narrow_then_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65 66 67))
  (narrow-to-region 2 4)
  (set-buffer-multibyte t)
  (list (buffer-string) (point-min) (point-max)))"##,
    );
}

#[test]
fn div_cx502_multibyte_overlay_then_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65 66))
  (let ((ov (make-overlay 1 3)))
    (overlay-put ov 'face 'bold))
  (set-buffer-multibyte t)
  (list (overlay-start ov) (overlay-end ov) (overlay-live-p ov)))"##,
    );
}

#[test]
fn div_cx502_multibyte_property_then_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65 66))
  (put-text-property 1 4 'face 'bold)
  (set-buffer-multibyte t)
  (get-text-property 1 'face))"##,
    );
}

#[test]
fn div_cx502_multibyte_delete_then_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65 66))
  (delete-region 1 2)
  (set-buffer-multibyte t)
  (buffer-string))"##,
    );
}

#[test]
fn div_cx502_multibyte_region_then_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65 66 67))
  (set-buffer-multibyte t)
  (set-buffer-multibyte nil)
  (set-buffer-multibyte t)
  (buffer-string))"##,
    );
}

#[test]
fn div_cx502_multibyte_save_excursion_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65 66))
  (save-excursion
    (set-buffer-multibyte t))
  (buffer-string))"##,
    );
}

#[test]
fn div_cx502_multibyte_two_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a (get-buffer-create " *cx502-a*"))
      (b (get-buffer-create " *cx502-b*")))
  (with-current-buffer a
    (set-buffer-multibyte nil)
    (insert (unibyte-string 200 65)))
  (with-current-buffer b
    (set-buffer-multibyte nil)
    (insert (unibyte-string 201 66)))
  (set-buffer-multibyte t)
  (with-current-buffer a (buffer-string)))"##,
    );
}
