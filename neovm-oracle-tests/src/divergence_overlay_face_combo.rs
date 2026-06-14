//! Complex/combo divergence probes for overlays, text properties & faces.
//!
//! Each test combines several features at once (overlay + text-property face
//! precedence, before/after-string + editing, invisible/intangible/field +
//! navigation, modification-hooks + undo, evaporate under delete, display
//! property, propertized-string round-trips). These interactions surface
//! divergences that single-feature focused tests miss.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- overlay vs text-property face precedence --------------------------------

#[test]
fn div_combo_overlay_vs_textprop_face_precedence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello world")
  (put-text-property 3 7 'face 'bold)
  (let ((ov (make-overlay 4 6)))
    (overlay-put ov 'face 'italic)
    (list (get-text-property 4 'face)
          (get-char-property 4 'face)
          (get-char-property 5 'face)
          (get-char-property 7 'face))))
"##,
    );
}

#[test]
fn div_combo_overlapping_overlays_priority_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // get-char-property resolves the highest-priority overlay.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefgh")
  (let ((o1 (make-overlay 2 6)) (o2 (make-overlay 3 7)) (o3 (make-overlay 4 8)))
    (overlay-put o1 'face 'bold)
    (overlay-put o2 'face 'italic)
    (overlay-put o3 'face 'underline)
    (overlay-put o1 'priority 1)
    (overlay-put o2 'priority 3)
    (overlay-put o3 'priority 2)
    (list (get-char-property 4 'face)
          (get-char-property 5 'face)
          (get-char-property 6 'face)
          (get-char-property 7 'face))))
"##,
    );
}

#[test]
fn div_combo_face_and_font_lock_face_both() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello")
  (put-text-property 1 4 'face 'bold)
  (put-text-property 2 4 'font-lock-face 'italic)
  (list (get-text-property 2 'face)
        (get-text-property 2 'font-lock-face)
        (get-char-property 2 'face)
        (get-char-property 2 'font-lock-face)))
"##,
    );
}

// --- before/after-string + editing + multibyte ------------------------------

#[test]
fn div_combo_before_string_with_face_insert_near() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café世界")
  (let ((ov (make-overlay 3 5)))
    (overlay-put ov 'face 'bold)
    (overlay-put ov 'before-string (propertize ">>" 'face 'italic)))
  (goto-char 3)
  (insert "X")
  (list (length (overlays-at 4))
        (point-max)
        (buffer-substring-no-properties 1 (point-max))))
"##,
    );
}

#[test]
fn div_combo_before_string_does_not_change_point_max() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café")
  (let ((ov (make-overlay 2 3)))
    (overlay-put ov 'before-string "世界"))
  (list (point-max) (buffer-string) (length (overlays-at 2))))
"##,
    );
}

#[test]
fn div_combo_after_string_with_embedded_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abc")
  (let ((ov (make-overlay 2 3)))
    (overlay-put ov 'after-string (propertize "Z" 'face 'bold 'mouse-face 'highlight)))
  (let* ((ov (car (overlays-at 2)))
         (as (overlay-get ov 'after-string)))
    (list as (text-properties-at 0 as))))
"##,
    );
}

// --- propertized string preservation across ops -----------------------------

#[test]
fn div_combo_concat_preserves_text_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s1 (propertize "ab" 'face 'bold))
      (s2 "cd"))
  (let ((r (concat s1 s2)))
    (list (text-properties-at 0 r) (text-properties-at 2 r) (length r))))
"##,
    );
}

#[test]
fn div_combo_substring_preserves_offset_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s (copy-sequence "abcdef"))
       (_ (put-text-property 1 4 'face 'bold s))
       (sub (substring s 2 5)))
  (list (text-properties-at 0 sub) (text-properties-at 1 sub) (text-properties-at 2 sub)))
"##,
    );
}

#[test]
fn div_combo_propertized_string_prin1_read_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s (propertize "café" 'face 'bold 'mouse-face 'highlight))
       (p (prin1-to-string s))
       (back (car (read-from-string p))))
  (list (equal s back) (text-properties-at 0 back) (length p)))
"##,
    );
}

#[test]
fn div_combo_buffer_substring_with_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello")
  (put-text-property 1 4 'face 'bold)
  (put-text-property 2 5 'mouse-face 'highlight)
  (let ((full (buffer-substring 1 5))
        (bare (buffer-substring-no-properties 1 5)))
    (list (text-properties-at 2 full)
          (text-properties-at 2 bare)
          full bare)))
"##,
    );
}

// --- overlay lifecycle under editing / narrowing ----------------------------

#[test]
fn div_combo_overlay_moves_with_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (let ((ov (make-overlay 3 7)))
    (overlay-put ov 'face 'bold)
    (goto-char 4)
    (insert "X")
    (list (overlay-start ov) (overlay-end ov))))
"##,
    );
}

#[test]
fn div_combo_overlay_clipping_under_narrowing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (make-overlay 3 7)
  (narrow-to-region 4 6)
  (list (length (overlays-in (point-min) (point-max)))
        (length (overlays-at 5))
        (point-min) (point-max)))
"##,
    );
}

#[test]
fn div_combo_overlay_evaporate_under_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((ov (make-overlay 3 5)))
    (overlay-put ov 'evaporate t)
    (delete-region 3 5)
    (list (overlay-start ov) (overlay-end ov) (overlayp ov) (length (overlays-at 3)))))
"##,
    );
}

#[test]
fn div_combo_overlay_survives_undo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "abcdef")
  (let ((ov (make-overlay 2 5)))
    (overlay-put ov 'face 'bold)
    (goto-char 3)
    (insert "X")
    (undo)
    (list (overlay-start ov) (overlay-end ov) (buffer-string) (length (overlays-at 2)))))
"##,
    );
}

// --- invisible / intangible / field + navigation ----------------------------

#[test]
fn div_combo_intangible_point_motion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 2 4 'intangible t)
  (goto-char 1)
  (let ((p1 (progn (forward-char) (point)))
        (p2 (progn (forward-char) (point)))
        (p3 (progn (forward-char) (point))))
    (list p1 p2 p3)))
"##,
    );
}

#[test]
fn div_combo_invisible_count_lines_and_forward_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "a\nb\nc\nd\n")
  (put-text-property 3 5 'invisible t)
  (list (count-lines 1 (point-max))
        (progn (goto-char 1) (forward-line 2) (point))))
"##,
    );
}

#[test]
fn div_combo_field_property_line_beginning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "line1\nline2\n")
  (put-text-property 7 12 'field 'myfield)
  (goto-char 9)
  (let ((inhibit-field-text-motion nil))
    (list (line-beginning-position)
          (line-beginning-position t)
          (constrain-to-field 1 (point) t))))
"##,
    );
}

#[test]
fn div_combo_read_only_property_insert_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello")
  (put-text-property 1 4 'read-only t)
  (let ((inhibit-read-only nil))
    (goto-char 2)
    (condition-case err (progn (insert "X") 'inserted) (error (car err)))))
"##,
    );
}

// --- display property & modification hooks ----------------------------------

#[test]
fn div_combo_display_property_buffer_substring() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abc")
  (put-text-property 2 3 'display (propertize "XYZ" 'face 'bold))
  (list (buffer-substring 2 3)
        (get-text-property 2 'display)
        (buffer-substring-no-properties 2 3)))
"##,
    );
}

#[test]
fn div_combo_modification_hooks_on_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let (fired)
    (let ((hook (lambda (beg end &rest _) (push (list beg end) fired))))
      (put-text-property 2 4 'modification-hooks (list hook))
      (put-text-property 2 4 'insert-in-front-hooks (list hook))
      (goto-char 3)
      (insert "X"))
    (list (length fired) fired)))
"##,
    );
}

// --- multiple text properties + change search across mix --------------------

#[test]
fn div_combo_mixed_props_change_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (put-text-property 1 3 'face 'bold)
  (put-text-property 4 6 'mouse-face 'highlight)
  (put-text-property 7 9 'font-lock-face 'keyword)
  (list (next-single-property-change 1 'face)
        (next-single-property-change 1 'mouse-face)
        (next-property-change 1)
        (next-property-change 3)
        (text-property-any 1 10 'font-lock-face 'keyword)))
"##,
    );
}

// --- category text property + overlay priority combos -----------------------

#[test]
fn div_combo_category_text_property_with_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (set-text-properties 2 4 '(face bold mouse-face highlight))
  (let ((sub (buffer-substring 2 4)))
    (list sub (text-properties-at 0 sub) (text-properties-at 1 sub))))
"##,
    );
}

#[test]
fn div_combo_overlay_priority_negative_and_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello world")
  (let ((o1 (make-overlay 2 5)) (o2 (make-overlay 2 5)) (o3 (make-overlay 2 5)))
    (overlay-put o1 'priority -10)
    (overlay-put o2 'priority 0)
    (overlay-put o3 'priority 5)
    (mapcar (lambda (o) (overlay-get o 'priority)) (overlays-at 3))))
"##,
    );
}

// --- propertize + mapcar + multibyte ----------------------------------------

#[test]
fn div_combo_propertize_each_char_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((result (mapcar (lambda (c)
                         (text-properties-at 0 (propertize (char-to-string c) 'face 'bold)))
                       "café")))
  (list (length result) (nth 0 result) (nth 3 result)))
"##,
    );
}

// --- narrowing + overlay + buffer-substring across boundary -----------------

#[test]
fn div_combo_narrow_overlay_substring_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "ABCDEFGHIJ")
  (let ((ov (make-overlay 3 7)))
    (overlay-put ov 'face 'bold))
  (narrow-to-region 4 8)
  (list (buffer-string)
        (text-properties-at 0 (buffer-string))
        (text-properties-at 3 (buffer-string))
        (point-min) (point-max)))
"##,
    );
}

// --- overlay before-string spanning multibyte + position tracking -----------

#[test]
fn div_combo_overlay_before_string_position_tracking() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café世界x")
  (let ((ov (make-overlay 4 6)))
    (overlay-put ov 'before-string ">>")
    (overlay-put ov 'face 'bold))
  (goto-char 5)
  (insert "Y")
  (let ((ov (car (overlays-at 5))))
    (list (point-max)
          (overlay-start ov) (overlay-end ov)
          (buffer-substring-no-properties 1 (point-max)))))
"##,
    );
}

// --- face inheritance + text-property face combo ----------------------------

#[test]
fn div_combo_face_inherit_and_property_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (defface neo-combo-parent '((t :foreground "green" :weight bold)) "doc")
  (defface neo-combo-child '((t :inherit neo-combo-parent :slant italic)) "doc")
  (with-temp-buffer
    (insert "hello world")
    (put-text-property 3 7 'face 'neo-combo-child)
    (list (face-attribute 'neo-combo-child :foreground)
          (face-attribute 'neo-combo-child :weight)
          (get-text-property 3 'face))))
"##,
    );
}
