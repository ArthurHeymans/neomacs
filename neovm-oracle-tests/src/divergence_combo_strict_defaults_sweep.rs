//! Strict combo oracle probes, batch 20: sweeps of frame, window, buffer-local,
//! face, and window-resize default values — the class where divergences have
//! clustered (line-spacing, frame-title-format, color-gray-p).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_f5_frame_parameter_sweep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (frame-parameters)))
  (list (assq 'vertical-scroll-bars p)
        (assq 'horizontal-scroll-bars p)
        (assq 'scroll-bar-width p)
        (assq 'scroll-bar-height p)
        (assq 'right-divider-width p)
        (assq 'bottom-divider-width p)
        (assq 'tool-bar-lines p)
        (assq 'z-group p)
        (assq 'inhibit-double-buffering p)))
"##,
    );
}

#[test]
fn div_f5_explicit_name_frame_param() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK nil
    // Neomacs:   OK (explicit-name)
    // The batch frame's explicit-name parameter is absent (nil) in GNU Emacs
    // but present in Neomacs.
    assert_oracle_parity(
        r##"
(assq 'explicit-name (frame-parameters))
"##,
    );
}

#[test]
fn div_f5_window_parameter_obscure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b (get-buffer-create " *probe-wpo*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b)
        (list (window-parameter nil 'clone-of)
              (window-parameter nil 'window-side)
              (window-parameter nil 'triggered-edge)
              (window-parameter nil 'combination-limit)
              (window-parameter nil 'preserved-size)
              (window-parameter nil 'no-delete-other-windows)))
    (when (buffer-live-p b) (kill-buffer b))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_f5_buffer_local_var_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (default-value 'bidi-display-reordering)
      (default-value 'bidi-paragraph-direction)
      (default-value 'cursor-type)
      (default-value 'cursor-in-non-selected-windows)
      (default-value 'left-margin)
      (default-value 'right-margin)
      (default-value 'tab-width)
      (default-value 'ctl-arrow)
      (default-value 'bidi-paragraph-separate-re)
      (default-value 'enable-multibyte-characters))
"##,
    );
}

#[test]
fn div_f5_face_attribute_sweep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (face-attribute 'region :background nil 'default)
      (face-attribute 'highlight :background nil 'default)
      (face-attribute 'secondary-selection :background nil 'default)
      (face-attribute 'link :foreground nil 'default)
      (face-attribute 'error :foreground nil 'default)
      (face-attribute 'warning :foreground nil 'default)
      (face-attribute 'success :foreground nil 'default)
      (face-attribute 'shadow :foreground nil 'default))
"##,
    );
}

#[test]
fn div_f5_window_resize_config_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (default-value 'window-resize-pixelwise)
      (default-value 'window-combination-resize)
      (default-value 'window-combination-limit)
      window-min-width
      window-min-height
      (default-value 'recenter-redisplay)
      (default-value 'auto-window-vscroll))
"##,
    );
}

#[test]
fn div_f5_even_window_heights_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK t
    // Neomacs:   OK width-only
    // (default-value 'even-window-heights) is t in GNU Emacs but width-only
    // in Neomacs.
    assert_oracle_parity(
        r##"
(default-value 'even-window-heights)
"##,
    );
}

#[test]
fn div_f5_line_number_face_and_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (facep 'line-number)
      (facep 'line-number-current-line)
      (face-attribute 'line-number :foreground nil 'default)
      (face-attribute 'line-number-current-line :foreground nil 'default))
"##,
    );
}

#[test]
fn div_f5_obarray_global_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((n 0))
  (mapatoms (lambda (_) (setq n (1+ n))))
  (list (> n 1000)
        (intern-soft "car")
        (intern-soft "defun")
        (intern-soft "nonexistent-probe-sym-xyz")))
"##,
    );
}
