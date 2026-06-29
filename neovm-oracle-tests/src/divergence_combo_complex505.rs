/// Batch 505: case-fold search characterization — all Greek chars both directions.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx505_casefold_search_greek_alpha() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-match "α" "Α") (string-match "Α" "α") (string-match "α" "α")))
"##,
        expect_test::expect![[r#""OK (0 0 0)""#]],
    );
}

#[test]
fn div_cx505_casefold_search_greek_pi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-match "π" "Π") (string-match "Π" "π")))
"##,
        expect_test::expect![[r#""OK (0 0)""#]],
    );
}

#[test]
fn div_cx505_casefold_search_greek_omega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-match "ω" "Ω") (string-match "Ω" "ω")))
"##,
        expect_test::expect![[r#""OK (0 0)""#]],
    );
}

#[test]
fn div_cx505_casefold_search_greek_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-match "[π-ω]+" "ΠΡΣΤΥΦΧΨΩ") (match-string 0 "ΠΡΣΤΥΦΧΨΩ")))
"##,
        expect_test::expect![[r#""OK (0 \"ΠΡΣΤΥΦΧΨΩ\")""#]],
    );
}

#[test]
fn div_cx505_casefold_search_cyrillic_er() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-match "р" "Р") (string-match "Р" "р")))
"##,
        expect_test::expect![[r#""OK (0 0)""#]],
    );
}

#[test]
fn div_cx505_casefold_search_cyrillic_es() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-match "с" "С") (string-match "С" "с")))
"##,
        expect_test::expect![[r#""OK (0 0)""#]],
    );
}

#[test]
fn div_cx505_casefold_search_cyrillic_ya() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-match "я" "Я") (string-match "Я" "я")))
"##,
        expect_test::expect![[r#""OK (0 0)""#]],
    );
}

#[test]
fn div_cx505_casefold_search_cyrillic_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-match "[р-я]+" "РСТУФХЦЧШЩЪЫЬЭЮЯ") (match-string 0 "РСТУФХЦЧШЩЪЫЬЭЮЯ")))
"##,
        expect_test::expect![[r#""OK (0 \"РСТУФХЦЧШЩЪЫЬЭЮЯ\")""#]],
    );
}

#[test]
fn div_cx505_casefold_upcase_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (with-temp-buffer
    (insert "πρστυφχψω")
    (upcase-region (point-min) (point-max))
    (buffer-string)))
"##,
        expect_test::expect![[r#""OK \"ΠΡΣΤΥΦΧΨΩ\"""#]],
    );
}

#[test]
fn div_cx505_casefold_downcase_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (with-temp-buffer
    (insert "ΠΡΣΤΥΦΧΨΩ")
    (downcase-region (point-min) (point-max))
    (buffer-string)))
"##,
        expect_test::expect![[r#""OK \"πρστυφχψω\"""#]],
    );
}

#[test]
fn div_cx505_casefold_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (with-temp-buffer
    (insert "abcΠΡΣΤΥdef")
    (goto-char 1)
    (while (re-search-forward "[π-ω]+" nil t)
      (replace-match (upcase (match-string 0))))
    (buffer-string)))
"##,
        expect_test::expect![[r#""OK \"abcΠΡΣΤΥdef\"""#]],
    );
}

#[test]
fn div_cx505_casefold_char_equal_pi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (char-equal ?π ?Π) (char-equal ?Π ?π)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx505_casefold_char_equal_omega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (char-equal ?ω ?Ω) (char-equal ?Ω ?ω)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx505_casefold_string_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (list (string-equal-ignore-case "πρστυ" "ΠΡΣΤΥ")
        (string-equal-ignore-case "φχψω" "ΦΧΨΩ")))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx505_casefold_search_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((case-fold-search t))
  (with-temp-buffer
    (insert "ΠΡΣΤΥ")
    (goto-char (point-max))
    (search-backward "πρστυ" nil t)))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}
