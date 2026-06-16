//! Complex combo batch 232 — `posn` / `pos-at-point` / `pos-at-x-y` /
//! pixel-position mapping / `frame-child-frame` / `display-fill-column` /
//! `display-line-numbers` pixel-width queries.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx232_posn_at_point_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "first line\nsecond line\nthird line\n")
  (goto-char 15)
  (let ((posn (posn-at-point (point))))
    (list (posn-point posn)
          (posn-col-row posn)
          (posn-actual-col-row posn)
          (posn-window posn)
          (posn-area posn)
          (posn-object posn))))
"##,
    );
}

#[test]
fn div_cx232_posn_at_x_y_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'posn-at-x-y)
      (fboundp 'pos-at-point)
      (fboundp 'window-absolute-pixel-edges))
"##,
    );
}

#[test]
fn div_cx232_posn_string_object_decomposition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "text with properties")
  (goto-char 5)
  (let ((posn (posn-at-point (point))))
    (list (posn-point posn)
          (posn-string posn)
          (posn-image posn)
          (posn-object posn))))
"##,
    );
}

#[test]
fn div_cx232_child_frame_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'make-frame)
          (fboundp 'make-child-frame)
          (boundp 'child-frame-parameters))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx232_display_fill_column_indicator_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'display-fill-column-indicator-mode)
      (boundp 'display-fill-column-indicator)
      (boundp 'display-fill-column-indicator-character))
"##,
    );
}

#[test]
fn div_cx232_display_line_numbers_pixel_width_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'display-line-numbers-mode)
      (boundp 'display-line-numbers-width)
      (boundp 'display-line-numbers-grow-limit)
      (boundp 'display-line-numbers-offset))
"##,
    );
}

#[test]
fn div_cx232_posn_col_row_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert (mapconcat #'identity (make-list 20 "text content here") "\n"))
  (goto-char 50)
  (let* ((posn (posn-at-point (point)))
         (col-row (posn-col-row posn))
         (actual-col-row (posn-actual-col-row posn)))
    (list (consp col-row)
          (consp actual-col-row)
          (>= (car col-row) 0)
          (>= (cdr col-row) 0))))
"##,
    );
}

#[test]
fn div_cx232_frame_parameters_pixel_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((frame (selected-frame)))
  (list (integerp (frame-pixel-width frame))
        (integerp (frame-pixel-height frame))
        (integerp (frame-char-width frame))
        (integerp (frame-char-height frame))
        (consp (frame-edges frame))))
"##,
    );
}

#[test]
fn div_cx232_window_text_pixel_dimensions_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((win (selected-window)))
  (list (integerp (window-text-pixel-width win))
        (integerp (window-text-pixel-height win))
        (integerp (window-body-width win 'pixels))
        (integerp (window-body-height win 'pixels))
        (integerp (window-max-chars-per-line))))
"##,
    );
}

#[test]
fn div_cx232_posn_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Posn mega test buffer content here for testing")
  (put-text-property 1 5 'face 'bold)
  (goto-char 15)
  (let ((m (set-marker (make-marker) 15))
        (ov (make-overlay 4 20)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (let ((posn (posn-at-point (point))))
      (narrow-to-region 2 35)
      (let ((state (list (posn-point posn)
                         (posn-col-row posn)
                         (fboundp 'display-fill-column-indicator-mode)
                         (boundp 'display-line-numbers-width)
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    );
}
