//! Complex combo batch 109 — `set-text-properties` interaction with
//! overlays, `add-text-properties` merging, `remove-text-properties`
//! selectivity, sticky rear-nonsticky across insertions.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx109_set_text_properties_replaces_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (add-text-properties 1 6 '(face bold weight light))
  (set-text-properties 1 6 '(color red))
  (list (text-properties-at 1)
        (text-properties-at 5)
        (text-properties-at 6)))
"##,
    );
}

#[test]
fn div_cx109_add_text_properties_appends_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (add-text-properties 1 4 '(face bold))
  (add-text-properties 1 4 '(weight heavy))
  (add-text-properties 1 4 '(color blue))
  (list (text-properties-at 1)
        (text-properties-at 2)
        (text-properties-at 4)
        (text-properties-at 5)))
"##,
    );
}

#[test]
fn div_cx109_remove_text_properties_specific_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (add-text-properties 1 6 '(face bold weight heavy color blue))
  (remove-text-properties 1 6 '(weight nil))
  (list (text-properties-at 1)
        (text-properties-at 3)
        (text-properties-at 6)))
"##,
    );
}

#[test]
fn div_cx109_remove_text_properties_all_with_nil_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (add-text-properties 1 6 '(face bold weight heavy color blue))
  (remove-text-properties 1 6 nil)
  (list (text-properties-at 1)
        (text-properties-at 3)
        (text-properties-at 6)))
"##,
    );
}

#[test]
fn div_cx109_rear_nonsticky_blocks_property_at_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (put-text-property 1 6 'face 'bold)
  (put-text-property 1 6 'rear-nonsticky t)
  (list (get-text-property 1 'face)
        (get-text-property 5 'face)
        (get-text-property 6 'face)
        (get-text-property 7 'face)))
"##,
    );
}

#[test]
fn div_cx109_text_property_stickiness_on_insertion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (put-text-property 1 5 'face 'bold)
  (let ((props-before (get-text-property 1 'face)))
    (goto-char 3)
    (insert "X")
    (let ((props-at-2 (get-text-property 2 'face))
          (props-at-3 (get-text-property 3 'face))
          (props-at-4 (get-text-property 4 'face))
          (props-at-5 (get-text-property 5 'face))))
    (goto-char 3)
    (insert "Y")
    (list props-before
          (get-text-property 1 'face)
          (get-text-property 3 'face)
          (get-text-property 4 'face)
          (get-text-property 5 'face)
          (get-text-property 6 'face))))
"##,
    );
}

#[test]
fn div_cx109_text_property_at_point_and_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 6 8 'face 'italic)
  (goto-char 3)
  (let ((at-3 (text-properties-at (point)))
        (beg (next-single-property-change (point-min) 'face))
        (end (next-single-property-change beg 'face)))
    (list at-3 beg end)))
"##,
    );
}

#[test]
fn div_cx109_text_property_search_with_predicate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "aaa bbb aaa ccc aaa")
      (put-text-property 1 3 'cat :x)
      (put-text-property 9 11 'cat :x)
      (put-text-property 17 19 'cat :x)
      (goto-char 1)
      (let* ((matches nil))
        (while (text-property-search-forward 'cat :x t)
          (push (list (prop-match-beginning (point))
                      (prop-match-end (point)))
                matches))
        (nreverse matches)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx109_set_text_properties_with_overlap_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (set-text-properties 1 6 '(face bold))
  (let ((ov (make-overlay 3 8)))
    (overlay-put ov 'face 'italic)
    (let ((at-1 (get-char-property 1 'face))
          (at-3 (get-char-property 3 'face))
          (at-5 (get-char-property 5 'face))
          (at-7 (get-char-property 7 'face))
          (at-8 (get-char-property 8 'face)))
      (list at-1 at-3 at-5 at-7 at-8
            (get-text-property 1 'face)
            (get-text-property 5 'face)))))
"##,
    );
}

#[test]
fn div_cx109_text_property_buffer_substring_with_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (put-text-property 1 4 '(face bold))
  (put-text-property 5 8 '(face italic))
  (let ((sub (buffer-substring 1 10))
        (no-props (buffer-substring-no-properties 1 10)))
    (list sub
          no-props
          (text-properties-at 0 sub)
          (text-properties-at 3 sub)
          (text-properties-at 4 sub)
          (text-properties-at 7 sub))))
"##,
    );
}

#[test]
fn div_cx109_set_text_properties_full_buffer_no_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (put-text-property 1 4 '(face bold))
  (set-text-properties (point-min) (point-max) nil)
  (list (text-properties-at 1)
        (text-properties-at 3)
        (text-properties-at 5)
        (text-properties-at 7)
        (next-single-property-change 1 'face)))
"##,
    );
}

#[test]
fn div_cx109_text_property_undo_redo_with_marker_overlay_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Text property mega test buffer")
  (add-text-properties 1 6 '(face bold weight heavy))
  (let ((m (set-marker (make-marker) 8))
        (ov (make-overlay 4 14)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 18)
    (set-text-properties 1 20 '(face underline))
    (let ((state (list (buffer-string)
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (text-properties-at 1)
                       (text-properties-at 5)
                       (text-properties-at 10))))
      (undo)
      (widen)
      (list state
            (buffer-string)
            (marker-position m)
            (overlay-start ov) (overlay-end ov)
            (text-properties-at 1)
            (text-properties-at 5)))))
"##,
    );
}
