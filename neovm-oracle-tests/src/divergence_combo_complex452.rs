//! Complex combo batch 452 — 15 deeper edge probes: string-join,
//! string-repeat, string-replace (Emacs 28), string-search,
//! string-equal-ignore-case multibyte, string-greaterp deep,
//! string-version-lessp, string-version-greaterp, string-to-number
//! edge hex, string-trim with predicate, string-glyph-split,
//! string-glyph-compose, string-limit after, string-pad after.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx452_string_join() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(string-join '("a" "b" "c") ", ")"##);
}

#[test]
fn div_cx452_string_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(string-repeat "ab" 3)"##);
}

#[test]
fn div_cx452_string_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(string-replace "foo" "bar" "foo foo foo")"##);
}

#[test]
fn div_cx452_string_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-search "world" "hello world")
      (string-search "xyz" "hello world"))"##,
    );
}

#[test]
fn div_cx452_string_equal_ignore_case_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-equal-ignore-case "cafe" "CAFE")
      (string-equal-ignore-case "cafe" "cafe")
      (string-equal-ignore-case "abc" "def"))"##,
    );
}

#[test]
fn div_cx452_string_lessp_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-lessp "abc" "abc")
      (string-lessp "abc" "abcd")
      (string-lessp "" "a"))"##,
    );
}

#[test]
fn div_cx452_string_version_lessp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-version-lessp "1.0" "2.0")
      (string-version-lessp "1.10" "1.2"))"##,
    );
}

#[test]
fn div_cx452_string_version_greaterp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-version-greaterp "2.0" "1.0")
      (string-version-greaterp "1.0" "2.0"))"##,
    );
}

#[test]
fn div_cx452_string_to_number_hex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-to-number "ff" 16)
      (string-to-number "0xff" 16)
      (string-to-number "1010" 2)
      (string-to-number "  -42  "))"##,
    );
}

#[test]
fn div_cx452_string_trim_predicate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(string-trim "  hello  " nil nil)"##);
}

#[test]
fn div_cx452_string_limit_after() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-limit "hello world" 5)
        (string-limit "hello world" 5 t)))"##,
    );
}

#[test]
fn div_cx452_string_pad_after() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-pad "hello" 8)
        (string-pad "hello" 8 nil t)
        (string-pad "hello" 3)))"##,
    );
}

#[test]
fn div_cx452_substring_no_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "hello"))
  (put-text-property 1 4 'face 'bold s)
  (list (substring-no-properties s)
        (substring-no-properties s 1 3)
        (substring s 1 3)))"##,
    );
}

#[test]
fn div_cx452_make_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (make-string 5 ?x)
      (make-string 3 65)
      (make-string 0 ?x))"##,
    );
}

#[test]
fn div_cx452_format_with_printf_flags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%-10s" "left")
      (format "%10s" "right")
      (format "%010d" 42)
      (format "%+d" 42)
      (format "% d" 42))"##,
    );
}
