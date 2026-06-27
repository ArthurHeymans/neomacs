//! Strict combo oracle probes, batch 18: string-make-unibyte of non-Latin-1,
//! tab-width motion math, invisible text + buffer-substring/invisibility-spec,
//! char-table extra-slot fill/get, window hscroll vs body-width, overlay
//! before/after-string vs buffer-string, and decode-char/encode-char on
//! out-of-charset codepoints.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_f3_string_make_unibyte_nonlatin1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (string-make-unibyte "日本") (error (car err)))
      (condition-case err (string-make-unibyte "café") (error (car err)))
      (length (string-make-unibyte "café"))
      (string-make-unibyte "abc")
      (multibyte-string-p (string-make-unibyte "abc")))
"##,
    );
}

#[test]
fn div_f3_tab_motion_widths() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (with-temp-buffer
        (setq-local tab-width 4)
        (insert "a\tb\tc")
        (goto-char (point-max))
        (current-column))
      (with-temp-buffer
        (setq-local tab-width 8)
        (insert "a\tb\tc")
        (goto-char (point-max))
        (current-column))
      (with-temp-buffer
        (setq-local tab-width 4)
        (insert "a\tb")
        (goto-char 1)
        (move-to-column 4)
        (list (current-column) (point))))
"##,
    );
}

#[test]
fn div_f3_invisible_buffer_substring() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "aaaa")
  (add-text-properties 2 4 '(invisible t))
  (list (buffer-substring 1 5)
        (buffer-substring-no-properties 1 5)
        (buffer-invisibility-spec)
        (progn (add-to-invisibility-spec 'probe-inv) (buffer-invisibility-spec))))
"##,
    );
}

#[test]
fn div_f3_char_table_extra_slots_filled() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'display-table 0)))
  (set-char-table-extra-slot ct 0 'slot0)
  (set-char-table-extra-slot ct 1 'slot1)
  (list (char-table-extra-slot ct 0)
        (char-table-extra-slot ct 1)
        (char-table-extra-slots ct)
        (char-table-extra-slot ct 5)))
"##,
    );
}

#[test]
fn div_f3_window_hscroll_body_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b (get-buffer-create " *probe-hsbw*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b)
        (set-window-hscroll nil 10)
        (list (window-hscroll)
              (window-body-width)
              (window-total-width)))
    (when (buffer-live-p b) (kill-buffer b))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_f3_overlay_string_vs_buffer_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((o (make-overlay 2 4)))
    (overlay-put o 'before-string "<B>")
    (overlay-put o 'after-string "<A>")
    (list (buffer-string)
          (overlay-get o 'before-string)
          (overlay-get o 'after-string)
          (length (overlays-in 1 5))
          (overlay-start o)
          (overlay-end o))))
"##,
    );
}

#[test]
fn div_f3_decode_char_out_of_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (decode-char 'ascii 128)
      (decode-char 'ascii 65)
      (encode-char ?日 'ascii)
      (encode-char 65 'ascii)
      (condition-case err (decode-char 'nonexistent-probe-cs 65)
        (error (car err))))
"##,
    );
}
