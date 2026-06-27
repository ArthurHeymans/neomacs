//! Strict combo oracle probes, batch 53: register system (set/get with
//! number/string/default and register-alist shape), char-validity edges
//! (char-valid-p / characterp / max-char / min-char), and window/frame-
//! configuration object predicates and comparison.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_k0_register_various_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((register-alist nil))
  (set-register ?a 42)
  (set-register ?b "hello")
  (list (get-register ?a)
        (get-register ?b)
        (get-register ?z 'missing)
        (length register-alist)
        (car (assq ?a register-alist))))
"##,
    );
}

#[test]
fn div_k0_register_point_and_increment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((register-alist nil))
  (with-temp-buffer
    (insert "abcdef")
    (goto-char 3)
    (point-to-register ?p)
    (increment-register 5 ?a)
    (list (markerp (get-register ?p))
          (marker-position (get-register ?p))
          (get-register ?a))))
"##,
    );
}

#[test]
fn div_k0_char_validity_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (char-valid-p ?a)
      (char-valid-p 0)
      (char-valid-p max-char)
      (char-valid-p (1+ max-char))
      (characterp ?a)
      (characterp (1+ max-char))
      (characterp -1)
      max-char
      min-char)
"##,
    );
}

#[test]
fn div_k0_window_frame_configuration_objects() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((wc (current-window-configuration))
      (fc (current-frame-configuration)))
  (list (window-configuration-p wc)
        (frame-configuration-p fc)
        (compare-window-configurations wc wc)
        (compare-window-configurations wc (current-window-configuration))))
"##,
    );
}

#[test]
fn div_k0_decode_char_charset_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (decode-char 'eight-bit 128)
      (decode-char 'eight-bit 255)
      (encode-char 128 'eight-bit)
      (char-charset 128)
      (char-charset 255)
      (char-charset 256)
      (decode-char 'unicode #x1F600)
      (encode-char #x1F600 'unicode))
"##,
    );
}
