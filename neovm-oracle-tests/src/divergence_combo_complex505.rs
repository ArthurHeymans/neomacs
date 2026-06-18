/// Batch 505: case-fold search characterization — all Greek chars both directions.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx505_casefold_search_greek_alpha() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-match "α" "Α") (string-match "Α" "α") (string-match "α" "α")))
"##,
    );
}

#[test]
fn div_cx505_casefold_search_greek_pi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-match "π" "Π") (string-match "Π" "π")))
"##,
    );
}

#[test]
fn div_cx505_casefold_search_greek_omega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-match "ω" "Ω") (string-match "Ω" "ω")))
"##,
    );
}

#[test]
fn div_cx505_casefold_search_greek_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-match "[π-ω]+" "ΠΡΣΤΥΦΧΨΩ") (match-string 0 "ΠΡΣΤΥΦΧΨΩ")))
"##,
    );
}

#[test]
fn div_cx505_casefold_search_cyrillic_er() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-match "р" "Р") (string-match "Р" "р")))
"##,
    );
}

#[test]
fn div_cx505_casefold_search_cyrillic_es() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-match "с" "С") (string-match "С" "с")))
"##,
    );
}

#[test]
fn div_cx505_casefold_search_cyrillic_ya() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-match "я" "Я") (string-match "Я" "я")))
"##,
    );
}

#[test]
fn div_cx505_casefold_search_cyrillic_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-match "[р-я]+" "РСТУФХЦЧШЩЪЫЬЭЮЯ") (match-string 0 "РСТУФХЦЧШЩЪЫЬЭЮЯ")))
"##,
    );
}

#[test]
fn div_cx505_casefold_upcase_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (with-temp-buffer
    (insert "πρστυφχψω")
    (upcase-region (point-min) (point-max))
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx505_casefold_downcase_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (with-temp-buffer
    (insert "ΠΡΣΤΥΦΧΨΩ")
    (downcase-region (point-min) (point-max))
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx505_casefold_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (with-temp-buffer
    (insert "abcΠΡΣΤΥdef")
    (goto-char 1)
    (while (re-search-forward "[π-ω]+" nil t)
      (replace-match (upcase (match-string 0))))
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx505_casefold_char_equal_pi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (char-equal ?π ?Π) (char-equal ?Π ?π)))
"##,
    );
}

#[test]
fn div_cx505_casefold_char_equal_omega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (char-equal ?ω ?Ω) (char-equal ?Ω ?ω)))
"##,
    );
}

#[test]
fn div_cx505_casefold_string_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (string-equal-ignore-case "πρστυ" "ΠΡΣΤΥ")
        (string-equal-ignore-case "φχψω" "ΦΧΨΩ")))
"##,
    );
}

#[test]
fn div_cx505_casefold_search_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (with-temp-buffer
    (insert "ΠΡΣΤΥ")
    (goto-char (point-max))
    (search-backward "πρστυ" nil t)))
"##,
    );
}
