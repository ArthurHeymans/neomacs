//! Complex combo batch 188 — `text-property` interval engine deep:
//! `next-property-change`, `previous-property-change`, `text-property-any`,
//! `text-property-not-all`, `text-property-search-forward/backward` with
//! overlays at boundaries.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx188_next_property_change_text_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghijklmnopqrstuvwxyz")
  (put-text-property 3 8 'face 'bold)
  (put-text-property 12 18 'face 'italic)
  (list (next-property-change 1)
        (next-property-change 3)
        (next-property-change 8)
        (next-property-change 12)
        (next-property-change 18)))
"##,
    );
}

#[test]
fn div_cx188_previous_property_change_text_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghijklmnopqrstuvwxyz")
  (put-text-property 3 8 'face 'bold)
  (put-text-property 12 18 'face 'italic)
  (list (previous-property-change 26)
        (previous-property-change 20)
        (previous-property-change 15)
        (previous-property-change 10)
        (previous-property-change 5)))
"##,
    );
}

#[test]
fn div_cx188_next_single_property_change_with_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789ABCDEF")
  (put-text-property 3 7 'face 'bold)
  (let ((ov (make-overlay 9 13)))
    (overlay-put ov 'face 'italic)
    (list (next-single-char-property-change 1 'face)
          (next-single-char-property-change 3 'face)
          (next-single-char-property-change 7 'face)
          (next-single-char-property-change 9 'face)
          (next-single-char-property-change 13 'face))))
"##,
    );
}

#[test]
fn div_cx188_text_property_any_with_predicate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghijklmnopqrstuvwxyz")
  (put-text-property 5 10 'p :one)
  (put-text-property 15 20 'p :two)
  (list (text-property-any 1 26 'p :one)
        (text-property-any 1 26 'p :two)
        (text-property-any 11 26 'p :one)
        (text-property-any 1 26 'p :missing)))
"##,
    );
}

#[test]
fn div_cx188_text_property_not_all_with_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghijklmnopqrstuvwxyz")
  (put-text-property 5 10 'p :val)
  (list (text-property-not-all 1 26 'p nil)
        (text-property-not-all 5 10 'p nil)
        (text-property-not-all 11 15 'p nil)
        (text-property-not-all 1 26 'p :val)))
"##,
    );
}

#[test]
fn div_cx188_text_property_search_forward_with_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "alpha beta gamma alpha beta gamma")
      (put-text-property 1 5 'cat :x)
      (put-text-property 7 10 'cat :x)
      (put-text-property 13 17 'cat :y)
      (let ((m1 (text-property-search-forward 'cat :x t)))
        (goto-char (point-max))
        (let ((m2 (text-property-search-backward 'cat :y t)))
          (list (and m1 (prop-match-beginning m1))
                (and m1 (prop-match-end m1))
                (and m2 (prop-match-beginning m2))
                (and m2 (prop-match-end m2))))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx188_get_char_property_with_overlay_priority_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (put-text-property 1 10 'face 'bold)
  (let ((ov1 (make-overlay 3 7)))
    (overlay-put ov1 'face 'italic)
    (overlay-put ov1 'priority 5)
    (let ((ov2 (make-overlay 4 6)))
      (overlay-put ov2 'face 'underline)
      (overlay-put ov2 'priority 10)
      (list (get-char-property 1 'face)
            (get-char-property 3 'face)
            (get-char-property 4 'face)
            (get-char-property 5 'face)
            (get-char-property 6 'face)
            (get-char-property 7 'face)
            (get-char-property 8 'face)))))
"##,
    );
}

#[test]
fn div_cx188_set_text_properties_clear_then_re_add() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (add-text-properties 1 6 '(face bold weight heavy))
  (set-text-properties 1 6 nil)
  (add-text-properties 1 6 '(face italic))
  (list (text-properties-at 1)
        (text-properties-at 3)
        (text-properties-at 5)
        (text-properties-at 6)
        (next-single-property-change 1 'face)))
"##,
    );
}

#[test]
fn div_cx188_sticky_props_after_insertion_at_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghij")
  (put-text-property 3 7 'front-sticky nil)
  (put-text-property 3 7 'rear-nonsticky t)
  (put-text-property 3 7 'p :core)
  (goto-char 5)
  (insert "X")
  (goto-char 8)
  (insert "Y")
  (list (buffer-string)
        (get-text-property 3 'p)
        (get-text-property 5 'p)
        (get-text-property 6 'p)
        (get-text-property 8 'p)
        (get-text-property 9 'p)))
"##,
    );
}

#[test]
fn div_cx188_text_prop_interval_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Text property interval mega test buffer content here")
  (put-text-property 1 10 'face 'bold)
  (put-text-property 12 20 'face 'italic)
  (put-text-property 25 35 'face 'underline)
  (let ((m (set-marker (make-marker) 15))
        (ov (make-overlay 5 30)))
    (overlay-put ov 'face 'region)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 3 38)
    (let ((state (list (next-property-change 1)
                       (next-single-char-property-change 1 'face)
                       (previous-property-change 35)
                       (text-property-any 1 35 'face 'bold)
                       (text-property-not-all 1 35 'face nil)
                       (buffer-string)
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (text-properties-at 1)
                       (get-char-property 14 'face))))
      (undo)
      (widen)
      (list state (buffer-string) (marker-position m)
            (overlay-start ov) (overlay-end ov)
            (text-properties-at 1)))))
"##,
    );
}
