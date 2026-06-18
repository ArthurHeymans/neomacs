//! Complex combo batch 448 — 15 random cross-feature interaction tests
//! combining unrelated features to surface emergent divergences:
//! calc+process+display, url+case-fold+time, info+overlay+string-collate,
//! woman+font+face, diff+marker+undo, vc+coding+detect, hanoi+eight-bit,
//! life+column+display, rot13+multibyte+encode, zone+buffer-local,
//! doctor+case-table+char-fold, copyright+bidi+regex, calc+network+stub,
//! url+overlay-lists+split-char, info+features+provide.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// calc + display column + process exit status.
#[test]
fn div_cx448_calc_display_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'calc)
  (list (calc-eval "2+3")
        (condition-case e (make-process :name "cx448" :command '("echo" "hi") :connection-type 'pipe :buffer nil) (error (car e)))))"##,
    );
}

/// url + case-fold + time.
#[test]
fn div_cx448_url_casefold_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'url-parse)
  (let ((case-fold-search t))
    (list (url-type (url-generic-parse-url "HTTP://EXAMPLE.COM"))
          (condition-case e (encode-time 30.5 30 14 16 6 2026 nil) (error (car e))))))"##,
    );
}

/// info-lookup + overlay + string-collate.
#[test]
fn div_cx448_info_overlay_collate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'info-look)
  (with-temp-buffer
    (insert "info text")
    (let ((ov (make-overlay 1 5)))
      (overlay-put ov 'face 'bold)
      (list (length (overlays-in 1 10))
            (string-collate-lessp "a" "B")))))"##,
    );
}

/// woman-browse + font-lock + face.
#[test]
fn div_cx448_woman_font_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'woman)
  (list (fboundp 'woman)
        (face-attribute 'bold :weight nil 'default)))"##,
    );
}

/// diff-mode + marker + undo.
#[test]
fn div_cx448_diff_marker_undo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'diff)
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "a\nb\nc\n")
    (let ((m (set-marker (make-marker) 2)))
      (delete-region 1 3)
      (undo)
      (marker-position m))))"##,
    );
}

/// vc + coding + detect-coding.
#[test]
fn div_cx448_vc_coding_detect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'vc)
  (list (boundp 'vc-handled-backends)
        (detect-coding-string "hello")))"##,
    );
}

/// hanoi + eight-bit + multibyte.
#[test]
fn div_cx448_hanoi_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'hanoi)
  (let ((raw (unibyte-string 200 201 65 66)))
    (list (fboundp 'hanoi)
          (string-bytes raw)
          (length raw))))"##,
    );
}

/// life + column + display.
#[test]
fn div_cx448_life_column_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'life)
  (with-temp-buffer
    (insert "abc")
    (put-text-property 2 3 'display "XX")
    (list (fboundp 'life) (current-column))))"##,
    );
}

/// rot13 + multibyte + encode.
#[test]
fn div_cx448_rot13_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (rot13 "hello")
      (encode-coding-string "café" 'utf-8))"##,
    );
}

/// zone + buffer-local + setq-local.
#[test]
fn div_cx448_zone_buffer_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'zone)
  (with-temp-buffer
    (setq-local neo-cx448-z 'zone-val)
    (list (fboundp 'zone) neo-cx448-z)))"##,
    );
}

/// copyright + case-fold + char-equal.
#[test]
fn div_cx448_copyright_casefold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'copyright)
  (let ((case-fold-search t))
    (list (fboundp 'copyright-update)
          (char-equal ?π ?Π))))"##,
    );
}

/// doctor + buffer-local-variables + string-collate.
#[test]
fn div_cx448_doctor_local_collate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'doctor)
  (with-temp-buffer
    (let ((locals (buffer-local-variables)))
      (list (boundp 'doctor-doctors)
            (string-collate-lessp "a" "B")
            (length locals)))))"##,
    );
}

/// calc + detect-coding + charset-priority.
#[test]
fn div_cx448_calc_coding_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'calc)
  (list (calc-eval "2*3")
        (detect-coding-string "abc")
        (length (charset-priority-list))))"##,
    );
}

/// url-parse + overlay-lists + split-char.
#[test]
fn div_cx448_url_overlay_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'url-parse)
  (with-temp-buffer
    (insert "abc")
    (let ((o (make-overlay 1 3)))
      (overlay-put o 'face 'bold)
      (list (url-type (url-generic-parse-url "https://test.com"))
            (length (car (overlay-lists)))
            (condition-case e (split-char ?A) (error (car e)))))))"##,
    );
}

/// info + features + provide.
#[test]
fn div_cx448_info_features_provide() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'info)
  (let ((sym (make-symbol "neo-cx448-f")))
    (provide sym)
    (list (fboundp 'info)
          (featurep sym)
          (listp features))))"##,
    );
}
