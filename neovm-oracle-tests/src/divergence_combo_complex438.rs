//! Complex combo batch 438 — 16 deep characterization probes targeting
//! known divergence areas: display column + overlay + invisible,
//! eight-bit + multibyte + coding + comparison, overlay-lists + delete
//! + create, set-buffer-multibyte + save-restriction + marker,
//! case-fold + char-equal + string-equal-ignore-case Greek,
//! string-collate + locale-coding-system, encode-time + time-add + format,
//! make-network-process + make-pipe-process + process-id.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// display column + invisible overlay + display prop combo.
#[test]
fn div_cx438_display_column_invisible_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abcd efgh ijkl")
  (put-text-property 5 6 'display "XXX")
  (let ((ov (make-overlay 10 14)))
    (overlay-put ov 'invisible t))
  (list (current-column)
        (progn (goto-char 7) (current-column))
        (progn (goto-char 11) (current-column))))"##,
    );
}

/// eight-bit recovered + string-bytes + md5 + format.
#[test]
fn div_cx438_eightbit_coding_md5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let* ((raw (unibyte-string #x80 #xff #xe9))
        (dec (decode-coding-string raw 'utf-8))
        (con (string-make-multibyte raw)))
  (list (string-bytes dec)
        (string-bytes con)
        (md5 dec)
        (md5 con)
        (format "%S" dec)
        (format "%S" con)))"##,
    );
}

/// overlay-lists: create + delete + create cycle.
#[test]
fn div_cx438_overlay_lists_create_delete_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abcdef")
  (let ((o (make-overlay 2 4)))
    (overlay-put o 'face 'bold)
    (delete-overlay o)
    (let ((o2 (make-overlay 2 4)))
      (overlay-put o2 'face 'italic)
      (list (length (car (overlay-lists)))
            (length (overlays-in 1 10))))))"##,
    );
}

/// set-buffer-multibyte + save-restriction + marker preservation.
#[test]
fn div_cx438_set_buf_multibyte_save_restriction_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65 66))
  (let ((m (set-marker (make-marker) 2)))
    (save-restriction
      (narrow-to-region 1 3)
      (set-buffer-multibyte t)
      (list (marker-position m)
            (point-min) (point-max)
            (buffer-string)))
    (list (marker-position m)
          (buffer-string)
          (length (buffer-string)))))"##,
    );
}

/// case-fold: char-equal across full CF/D1 ranges.
#[test]
fn div_cx438_char_equal_casefold_full() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (char-equal ?π ?Π) (char-equal ?Π ?π)
        (char-equal ?ρ ?Ρ) (char-equal ?Ρ ?ρ)
        (char-equal ?σ ?Σ) (char-equal ?Σ ?σ)
        (char-equal ?τ ?Τ) (char-equal ?Τ ?τ)
        (char-equal ?υ ?Υ) (char-equal ?Υ ?υ)
        (char-equal ?φ ?Φ) (char-equal ?Φ ?φ)
        (char-equal ?χ ?Χ) (char-equal ?Χ ?χ)
        (char-equal ?ψ ?Ψ) (char-equal ?Ψ ?ψ)
        (char-equal ?ω ?Ω) (char-equal ?Ω ?ω)))"##,
    );
}

/// string-equal-ignore-case with Greek.
#[test]
fn div_cx438_string_equal_ignore_case_greek() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-equal-ignore-case "πρστυ" "ΠΡΣΤΥ")
      (string-equal-ignore-case "φχψω" "ΦΧΨΩ"))"##,
    );
}

/// string-collate with locale-coding-system interaction.
#[test]
fn div_cx438_string_collate_locale_interact() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-collate-lessp "ä" "z")
      (string-collate-lessp "ö" "o"))
"##,
    );
}

/// encode-time + time-add + format-time-string timezone edge.
#[test]
fn div_cx438_time_add_format_tz() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((t1 (encode-time 0 0 12 16 6 2024 nil)))
      (format-time-string "%Y-%m-%d %H:%M:%S %z" t1))
  (error (car e)))
"##,
    );
}

/// process: make-network-process + make-pipe-process stubs.
#[test]
fn div_cx438_process_stubs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (condition-case e (make-network-process :name "n" :server t :service 0) (error (car e)))
      (condition-case e (make-pipe-process :name "p") (error (car e))))"##,
    );
}

/// detect-coding-string + find-coding-system + coding-system-p.
#[test]
fn div_cx438_coding_detect_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (detect-coding-string "hello")
      (find-coding-system 'utf-8)
      (coding-system-p 'utf-8)
      (coding-system-p 'nonexistent))"##,
    );
}

/// buffer-local-variables with local and permanent locals.
#[test]
fn div_cx438_buffer_local_vars_locals() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (let ((a (make-local-variable 'neo-cx438-a))
        (b (make-local-variable 'neo-cx438-b)))
    (setq neo-cx438-a 1)
    (let ((locals (buffer-local-variables)))
      (length locals))))"##,
    );
}

/// make-frame in batch: error type differs.
#[test]
fn div_cx438_make_frame_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (make-frame '((name . "test")))
  (error (car e)))"##,
    );
}

/// split-char + charset-after + char-charset combo.
#[test]
fn div_cx438_split_char_charset_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (condition-case e (char-charset ?A) (error (car e)))
      (condition-case e (split-char ?A) (error (car e)))
      (condition-case e (char-charset ?世) (error (car e)))
      (condition-case e (split-char ?世) (error (car e))))"##,
    );
}

/// apropos with missing metadata test.
#[test]
fn div_cx438_apropos_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'apropos)
  (let ((buf (get-buffer-create "*Apropos*")))
    (apropos "forward-word")
    (prog1 (with-current-buffer buf
             (count-lines (point-min) (point-max)))
      (kill-buffer buf))))"##,
    );
}

/// time-convert + float-time + seconds-to-time combo.
#[test]
fn div_cx438_time_convert_float_seconds() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((t1 (encode-time 0 0 0 1 1 2024 nil)))
  (list (time-convert t1 'integer)
        (condition-case e (time-convert t1 'float) (error (car e)))
        (seconds-to-time (float-time t1))))"##,
    );
}
