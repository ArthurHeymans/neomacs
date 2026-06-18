//! Complex combo batch 405 — 20 probes targeting fresh divergence
//! surfaces: detect-coding-string, time-convert with float/nil,
//! char-equal with case-fold across Greek/Cyrillic, string-greaterp
//! with case-fold, reverse/nreverse on strings with multibyte,
//! sort with string-lessp vs string-greaterp, encode-coding-string
//! to latin-1, current-time-zone, float-time with t, seconds-to-time,
//! color-distance, display-color-p, font-family-list in batch,
//! getenv with multibyte, featurep after provide, load/autoload
//! path resolution, prog1/prog2 edge, time-to-seconds, and
//! format-with-line-numbers.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// detect-coding-string with various byte sequences:
/// Neomacs returns (undecided) for everything (stub).
#[test]
fn div_cx405_detect_coding_string_stub() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (detect-coding-string "hello")
      (detect-coding-string (unibyte-string 228 184 173 230 151 182))
      (detect-coding-string "café")
      (detect-coding-string (string #xff #xfe #x00)))
"##,
    );
}

/// time-convert with float and nil: Neomacs may not support
/// the newer time-convert API correctly.
#[test]
fn div_cx405_time_convert_float_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((t1 (encode-time 0 0 0 1 1 2024 nil)))
  (list (condition-case e (time-convert t1 'float) (error (car e)))
        (condition-case e (time-convert t1 'integer) (error (car e)))
        (condition-case e (time-convert t1 nil) (error (car e)))
        (condition-case e (time-convert t1 'list) (error (car e)))))
"##,
    );
}

/// char-equal with case-fold across Greek characters:
/// should match lower↔upper in both directions.
#[test]
fn div_cx405_char_equal_greek_casefold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (list (char-equal ?α ?Α)
        (char-equal ?Α ?α)
        (char-equal ?π ?Π)
        (char-equal ?Π ?π)
        (char-equal ?ω ?Ω)
        (char-equal ?Ω ?ω)))
"##,
    );
}

/// string-greaterp with mixed case multibyte strings.
#[test]
fn div_cx405_string_greaterp_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (list (string-lessp "α" "β")
        (string-greaterp "β" "α")
        (string-lessp "abc" "α")
        (string-lessp "α" "abc")))
"##,
    );
}

/// reverse / nreverse on strings with multibyte characters.
#[test]
fn div_cx405_reverse_string_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (reverse "café")
      (reverse "世界")
      (let ((s "abc")) (nreverse s) s)
      (reverse "αβγδε"))
"##,
    );
}

/// sort with string-lessp vs string-greaterp on multibyte list.
#[test]
fn div_cx405_sort_multibyte_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((words '("café" "alpha" "βeta" "世界" "über" "世界abc")))
  (list (sort (copy-sequence words) #'string-lessp)
        (sort (copy-sequence words) #'string-greaterp)))
"##,
    );
}

/// encode-coding-string to latin-1: should encode multibyte string
/// to latin-1 bytes, may differ for non-encodeable chars.
#[test]
fn div_cx405_encode_coding_latin1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (encode-coding-string "café" 'latin-1)
      (encode-coding-string "abc" 'latin-1)
      (condition-case e (encode-coding-string "世界" 'latin-1) (error (car e))))
"##,
    );
}

/// current-time-zone: timezone offset/name may differ between
/// Neomacs and GNU for the same timestamp.
#[test]
fn div_cx405_current_time_zone() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((t1 (encode-time 0 0 12 1 1 2024 nil)))
  (list (current-time-zone t1)
        (car (current-time-zone t1))
        (cdr (current-time-zone t1))))
"##,
    );
}

/// float-time with encode-time value and t (error type differs).
#[test]
fn div_cx405_float_time_fixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((t1 (encode-time 0 0 0 1 1 2024 nil)))
  (list (float-time t1)
        (condition-case e (float-time t) (error (car e)))))
"##,
    );
}

/// seconds-to-time and time-to-seconds roundtrip.
#[test]
fn div_cx405_seconds_to_time_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (seconds-to-time 3600)
      (time-to-seconds (seconds-to-time 86400))
      (time-to-seconds (seconds-to-time 0.5)))
"##,
    );
}

/// color-distance between two named colors.
#[test]
fn div_cx405_color_distance() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (color-distance "red" "blue")
      (color-distance "red" "#ff0000")
      (color-distance "black" "white"))
"##,
    );
}

/// display-color-p and display-color-cells in batch.
#[test]
fn div_cx405_display_color_capabilities() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (display-color-p)
      (display-color-cells)
      (display-graphic-p)
      (display-visual-class))
"##,
    );
}

/// font-family-list in batch mode — may return different
/// font families or nil.
#[test]
fn div_cx405_font_family_list_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((families (font-family-list)))
  (list (listp families)
        (> (length families) 0)
        (member "monospace" families)
        (member "Monospace" families)))
"##,
    );
}

/// getenv with multibyte environment variable names/values.
#[test]
fn div_cx405_getenv_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((process-environment (cons "NEO_CX405_TEST=café世界" process-environment)))
  (list (getenv "NEO_CX405_TEST")
        (getenv "HOME")
        (getenv "NONEXISTENT_ENV_VAR_12345")))
"##,
    );
}

/// featurep after provide: feature registration differences.
#[test]
fn div_cx405_featurep_after_provide() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((feature (make-symbol "neo-cx405-feat")))
  (list (featurep feature)
        (provide feature)
        (featurep feature)))
"##,
    );
}

/// load / require with autoload path resolution:
/// Neomacs may resolve paths differently.
#[test]
fn div_cx405_require_path_resolution() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (require 'neomacs-nonexistent) (error (car e)))
      (condition-case e (load "neomacs-nonexistent" t t) (error (car e)))
      (featurep 'neomacs-nonexistent))
"##,
    );
}

/// prog1 / prog2 edge cases with multiple values.
#[test]
fn div_cx405_prog1_prog2_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (prog1 1 (setq a 2) (setq b 3))
      (prog1 (+ 1 2) (+ 3 4) (+ 5 6))
      (prog2 1 2 3)
      (prog2 (+ 10 20) (+ 30 40) (+ 50 60)))
"##,
    );
}

/// format-with-line-numbers: line number formatting.
#[test]
fn div_cx405_format_with_line_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "line %04d: %s" 1 "hello")
      (format "line %4d: %s" 42 "world")
      (format "%010d" 12345))
"##,
    );
}

/// string-pad / string-limit with multibyte strings.
#[test]
fn div_cx405_string_pad_limit_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "café世界"))
  (list (string-pad s 12)
        (string-pad s 12 nil t)
        (string-limit s 4)
        (string-limit s 6)))
"##,
    );
}
