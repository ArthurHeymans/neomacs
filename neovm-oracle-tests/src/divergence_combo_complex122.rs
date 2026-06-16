//! Complex combo batch 122 — `width` / `string-width` / `char-width` /
//! `truncate-string-to-width` / `window-text-pixel-width` consistency
//! with multibyte, double-width CJK, and combining marks.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx122_string_width_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-width "hello")
      (string-width "")
      (string-width "hello world"))
"##,
    );
}

#[test]
fn div_cx122_string_width_multibyte_2_byte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-width "café")
      (string-width "α β γ")
      (string-width "naïve"))
"##,
    );
}

#[test]
fn div_cx122_string_width_cjk_double_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-width "世界")
      (string-width "你好")
      (string-width "日本語")
      (string-width "hello 世界"))
"##,
    );
}

#[test]
fn div_cx122_char_width_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (char-width c)))
        '(?a ?A ?0 ?\s ?- ?_
          ?α ?Ω ?à ?é ?ü ?ñ ?ß
          ?世 ?界 ?日 ?本 ?語
          ?😀 ?\n ?\t))
"##,
    );
}

#[test]
fn div_cx122_truncate_string_to_width_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (truncate-string-to-width "hello world" 5)
      (truncate-string-to-width "hello world" 20)
      (truncate-string-to-width "hello world" 8 nil t "...")
      (truncate-string-to-width "hello world" 5 2 nil))
"##,
    );
}

#[test]
fn div_cx122_truncate_string_with_multibyte_widths() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (truncate-string-to-width "hello世界" 5)
      (truncate-string-to-width "hello世界" 6)
      (truncate-string-to-width "hello世界" 7)
      (truncate-string-to-width "café 世界" 8))
"##,
    );
}

#[test]
fn div_cx122_window_text_pixel_width_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((win (selected-window)))
  (list (integerp (window-text-pixel-width win))
        (integerp (window-text-pixel-height win))
        (integerp (window-body-width win))
        (integerp (window-body-height win))))
"##,
    );
}

#[test]
fn div_cx122_current_column_with_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café 世界 hello")
  (goto-char 1)
  (forward-char 4)
  (let ((c1 (current-column)))
    (forward-char 1)
    (let ((c2 (current-column)))
      (forward-char 2)
      (let ((c3 (current-column)))
        (list c1 c2 c3)))))
"##,
    );
}

#[test]
fn div_cx122_move_to_column_with_cjk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "你好abc世界def")
  (goto-char 1)
  (move-to-column 4)
  (let ((p1 (point)))
    (move-to-column 8)
    (let ((p2 (point)))
      (move-to-column 12)
      (list p1 p2 (point) (current-column)))))
"##,
    );
}

#[test]
fn div_cx122_indent_to_with_tabs_and_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((indent-tabs-mode nil))
    (insert "x")
    (indent-to 10)
    (let ((s1 (buffer-string)))
      (erase-buffer)
      (let ((indent-tabs-mode t)
            (tab-width 4))
        (insert "x")
        (indent-to 10))
      (let ((s2 (buffer-string)))
        (list s1 s2 (current-column))))))
"##,
    );
}

#[test]
fn div_cx122_string_pixel_width_via_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%20s|" "hello")
      (format "%-20s|" "hello")
      (format "%20s|" "世界")
      (format "%-20s|" "café 世界"))
"##,
    );
}

#[test]
fn div_cx122_window_max_chars_per_line_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((win (selected-window)))
  (list (integerp (window-max-chars-per-line))
        (> (window-max-chars-per-line) 0)))
"##,
    );
}

#[test]
fn div_cx122_string_width_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Width test buffer café 世界 hello")
  (put-text-property 1 5 'face 'bold)
  (let ((m (set-marker (make-marker) 12))
        (ov (make-overlay 4 18)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 25)
    (let ((state (list (string-width (buffer-string))
                       (string-width "café 世界")
                       (char-width ?世)
                       (buffer-string)
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (text-properties-at 1))))
      (undo)
      (widen)
      (list state (buffer-string) (marker-position m)
            (overlay-start ov) (overlay-end ov)
            (text-properties-at 1)))))
"##,
    );
}
