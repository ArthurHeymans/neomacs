/// Batch 536: rx, pcase, string-case, char-fold-to-regexp deep probes.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx536_rx_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match (rx "hello") "hello world")
"##,
    );
}

#[test]
fn div_cx536_rx_or() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match (rx (or "cat" "dog")) "doghouse")
"##,
    );
}

#[test]
fn div_cx536_rx_and() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match (rx (and "abc" "def")) "abcdef")
"##,
    );
}

#[test]
fn div_cx536_rx_char_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match (rx (any "a-z")) "5")
"##,
    );
}

#[test]
fn div_cx536_rx_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match (rx (+ (in "0-9"))) "abc123def")
"##,
    );
}

#[test]
fn div_cx536_rx_opt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match (rx "ab" (? "c") "d") "abd")
"##,
    );
}

#[test]
fn div_cx536_rx_minimal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match (rx (minimal-match (one-or-more any)) "b") "aabb")
"##,
    );
}

#[test]
fn div_cx536_rx_maximal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match (rx (maximal-match (one-or-more any)) "b") "aabb")
"##,
    );
}

#[test]
fn div_cx536_char_fold_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (char-fold-to-regexp "a") (char-fold-to-regexp "e"))
"##,
    );
}

#[test]
fn div_cx536_char_fold_accent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (string-match (char-fold-to-regexp "cafe") "café"))
"##,
    );
}

#[test]
fn div_cx536_char_fold_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (string-match (char-fold-to-regexp "αβγ") "αβγδε"))
"##,
    );
}

#[test]
fn div_cx536_pcase_let_star() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase-let* ((`(,a ,b) '(1 2))
                (c (+ a b)))
  c)
"##,
    );
}

#[test]
fn div_cx536_pcase_let() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase-let ((`(,a . ,b) '(1 2 3 4)))
  (list a b))
"##,
    );
}

#[test]
fn div_cx536_pcase_exhaustive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase-exhaustive '(1 2 3)
  (`(,a ,b ,c) (+ a b c)))
"##,
    );
}

#[test]
fn div_cx536_pcase_dolist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let (result)
  (pcase-dolist (`(,a ,b) '((1 2) (3 4) (5 6)))
    (push (+ a b) result))
  (nreverse result))
"##,
    );
}
