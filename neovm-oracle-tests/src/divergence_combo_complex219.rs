//! Complex combo batch 219 — `window` display / `mode-line-format` /
//! `header-line-format` / `tab-line-format` / `face-remapping-alist`
//! interaction with buffer-local state.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx219_mode_line_format_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ml (default-value 'mode-line-format)))
  (list (consp ml)
        (> (length ml) 0)
        (format-mode-line mode-line-format)
        (format-mode-line "%b %p")))
"##,
    );
}

#[test]
fn div_cx219_header_line_format_buffer_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((before header-line-format))
    (setq header-line-format "Custom Header")
    (list before
          (buffer-local-value 'header-line-format (current-buffer))
          (format-mode-line header-line-format))))
"##,
    );
}

#[test]
fn div_cx219_face_remapping_alist_buffer_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((before face-remapping-alist))
    (setq-local face-remapping-alist '((default :height 2.0)
                                       (bold :foreground "red")))
    (list before
          (buffer-local-value 'face-remapping-alist (current-buffer))
          (assq 'default face-remapping-alist)
          (assq 'bold face-remapping-alist)
          (assq 'italic face-remapping-alist))))
"##,
    );
}

#[test]
fn div_cx219_tab_line_format_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'tab-line-format)
      (boundp 'global-tab-line-mode)
      (fboundp 'tab-line-mode))
"##,
    );
}

#[test]
fn div_cx219_mode_line_modified_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "content")
  (let ((ml-modified (format-mode-line "%*"))
        (ml-position (format-mode-line "%p"))
        (ml-buffer (format-mode-line "%b")))
    (list ml-modified ml-position ml-buffer)))
"##,
    );
}

#[test]
fn div_cx219_mode_line_format_with_multiple_specs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "test")
  (let ((result (format-mode-line
                 '(:eval (format "[%s:%d]" (buffer-name) (point))))))
    (list (stringp result)
          (> (length result) 0))))
"##,
    );
}

#[test]
fn div_cx219_face_remapping_inherited_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (setq-local face-remapping-alist '((mode-line (:inherit default :background "blue"))))
  (list (assq 'mode-line face-remapping-alist)
        (buffer-local-value 'face-remapping-alist (current-buffer))))
"##,
    );
}

#[test]
fn div_cx219_mode_line_format_with_props_preserved() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((ml-str (format-mode-line mode-line-format))
       (props-at-0 (text-properties-at 0 ml-str))
       (len (length ml-str)))
  (list (stringp ml-str)
        (> len 0)
        props-at-0
        len))
"##,
    );
}

#[test]
fn div_cx219_window_mode_line_height_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((win (selected-window)))
  (list (integerp (window-mode-line-height win))
        (integerp (window-header-line-height win))))
"##,
    );
}

#[test]
fn div_cx219_mode_line_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Mode-line mega test buffer content")
  (put-text-property 1 6 'face 'bold)
  (setq-local face-remapping-alist '((default :height 2.0)))
  (let ((m (set-marker (make-marker) 8))
        (ov (make-overlay 4 14)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 18)
    (let ((state (list (format-mode-line "%b %p")
                       (assq 'default face-remapping-alist)
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
