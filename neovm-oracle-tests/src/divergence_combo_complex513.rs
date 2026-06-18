/// Batch 513: string-lessp vs string-collate-lessp deeper, string version ordering.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx513_string_lessp_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-lessp "a" "b") (string-lessp "b" "a") (string-lessp "a" "a"))
"##,
    );
}

#[test]
fn div_cx513_string_lessp_mixed_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-lessp "A" "a") (string-lessp "a" "A"))
"##,
    );
}

#[test]
fn div_cx513_string_lessp_accent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-lessp "cafe" "café") (string-lessp "café" "cafe")))
"##,
    );
}

#[test]
fn div_cx513_string_version_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(sort '("1.10" "1.2" "1.1" "2.0" "1.20") #'string-version-lessp)
"##,
    );
}

#[test]
fn div_cx513_string_version_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-version-lessp "1.0" "2.0")
      (string-version-lessp "1.10" "1.2")
      (string-version-lessp "1.0a" "1.0b"))
"##,
    );
}

#[test]
fn div_cx513_string_version_greater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-version-greaterp "2.0" "1.0")
      (string-version-greaterp "1.0" "2.0"))
"##,
    );
}

#[test]
fn div_cx513_compare_strings_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (compare-strings "abc" nil nil "ABC" nil nil t)
      (compare-strings "ABC" nil nil "abc" nil nil t))
"##,
    );
}

#[test]
fn div_cx513_compare_strings_accent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (compare-strings "cafe" nil nil "CAFE" nil nil t)))
"##,
    );
}

#[test]
fn div_cx513_string_prefix_suffix_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-prefix-p "" "abc")
      (string-prefix-p "abc" "")
      (string-suffix-p "" "abc"))
"##,
    );
}

#[test]
fn div_cx513_string_remove_prefix_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-remove-prefix "abc" "abcdef")
      (string-remove-prefix "xyz" "abcdef")
      (string-remove-suffix "def" "abcdef"))
"##,
    );
}

#[test]
fn div_cx513_string_replace_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-replace "a" "x" "aaa aaa")
      (string-replace "" "x" "abc")
      (string-replace "a" "" "abcabc"))
"##,
    );
}

#[test]
fn div_cx513_string_pad_trim_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-pad "abc" 10)
        (string-trim "  abc  ")
        (string-trim-left "  abc")
        (string-trim-right "abc  ")))
"##,
    );
}

#[test]
fn div_cx513_string_chop_newline_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-chop-newline "hello\n")
        (string-chop-newline "hello\r\n")
        (string-chop-newline "hello")))
"##,
    );
}

#[test]
fn div_cx513_string_limit_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-limit "hello world" 5)
        (string-limit "hello world" 5 t)
        (string-limit "abc" 10)))
"##,
    );
}

#[test]
fn div_cx513_string_lines_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-lines "hello\nworld\nthird")
        (string-lines "")
        (string-lines "single")))
"##,
    );
}
