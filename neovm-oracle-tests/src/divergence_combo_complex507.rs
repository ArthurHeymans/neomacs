/// Batch 507: string-collate-lessp characterization — various inputs.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx507_collate_basic_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "a" "b") (string-collate-lessp "b" "a"))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx507_collate_uppercase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "A" "a") (string-collate-lessp "a" "A"))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx507_collate_diacritics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "a" "ä") (string-collate-lessp "ä" "a"))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx507_collate_accent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "e" "é") (string-collate-lessp "é" "e"))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx507_collate_ss() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "ss" "ß") (string-collate-lessp "ß" "ss"))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx507_collate_case_insensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "a" "B" nil t) (string-collate-lessp "B" "a" nil t))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx507_collate_numeric() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "1" "2") (string-collate-lessp "10" "2"))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx507_collate_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "" "a") (string-collate-lessp "a" ""))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx507_collate_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "a" "a") (string-collate-lessp "abc" "abc"))
"##,
        expect_test::expect![[r#""OK (nil nil)""#]],
    );
}

#[test]
fn div_cx507_collate_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-collate-lessp "cafe" "café") (string-collate-lessp "café" "cafe"))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx507_collate_sort_mixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(sort '("a" "B" "c" "A" "b" "C") #'string-collate-lessp)
"##,
        expect_test::expect![[r#""OK (\"A\" \"B\" \"C\" \"a\" \"b\" \"c\")""#]],
    );
}

#[test]
fn div_cx507_collate_sort_diacritic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(sort '("a" "ä" "o" "ö" "u" "ü") #'string-collate-lessp)
"##,
        expect_test::expect![[r#""OK (\"a\" \"o\" \"u\" \"ä\" \"ö\" \"ü\")""#]],
    );
}

#[test]
fn div_cx507_collate_sort_mixed_case_diacritic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(sort '("A" "ä" "O" "ö" "U" "ü" "a" "o" "u") #'string-collate-lessp)
"##,
        expect_test::expect![[r#""OK (\"A\" \"O\" \"U\" \"a\" \"o\" \"u\" \"ä\" \"ö\" \"ü\")""#]],
    );
}

#[test]
fn div_cx507_collate_locale_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (let ((locale-coding-system 'utf-8))
      (string-collate-lessp "a" "ä"))
  (error (car e)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx507_collate_ignore_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(sort '("abc" "ABC" "Abc" "aBc") (lambda (a b) (string-collate-lessp a b nil t)))
"##,
        expect_test::expect![[r#""OK (\"abc\" \"ABC\" \"Abc\" \"aBc\")""#]],
    );
}
