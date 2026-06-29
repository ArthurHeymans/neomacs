/// Batch 510: character operation edge cases — zero-width, combining, SMP, surrogates.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx510_char_width_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (char-width #x0300) (char-width #x200B) (char-width #x200D))
"##,
        expect_test::expect![[r#""OK (0 0 0)""#]],
    );
}

#[test]
fn div_cx510_char_width_wide() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (char-width #x11000) (char-width #x1F600) (char-width #x2A600))
"##,
        expect_test::expect![[r#""OK (1 2 2)""#]],
    );
}

#[test]
fn div_cx510_char_bytes_extreme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s "a cafe \U0001F600 end"))
  (list (length s) (string-bytes s) (string-width s)))
"##,
        expect_test::expect![[r#""OK (12 15 13)""#]],
    );
}

#[test]
fn div_cx510_string_width_smp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s "abc\U0001F600\U0001F601\U0001F602"))
  (list (length s) (string-width s) (string-bytes s)))
"##,
        expect_test::expect![[r#""OK (6 9 15)""#]],
    );
}

#[test]
fn div_cx510_string_width_zero_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s "a\u0301e\u0300o\u0302"))
  (list (length s) (string-width s) (string-bytes s)))
"##,
        expect_test::expect![[r#""OK (6 3 9)""#]],
    );
}

#[test]
fn div_cx510_char_direction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (get-char-code-property ?c 'bidi-class)
      (get-char-code-property ?\u05D0 'bidi-class)
      (get-char-code-property ?\u0600 'bidi-class))
"##,
        expect_test::expect![[r#""OK (L R AN)""#]],
    );
}

#[test]
fn div_cx510_char_mirrored() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (get-char-code-property ?\( 'mirror)
      (get-char-code-property ?\) 'mirror)
      (get-char-code-property ?< 'mirror))
"##,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
}

#[test]
fn div_cx510_char_numeric() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (get-char-code-property ?5 'numeric-value)
      (get-char-code-property ?0 'numeric-value)
      (get-char-code-property #x2150 'numeric-value))
"##,
        expect_test::expect![[r#""OK (5 0 0.14285714285714285)""#]],
    );
}

#[test]
fn div_cx510_char_combining_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (get-char-code-property ?a 'canonical-combining-class)
      (get-char-code-property #x0300 'canonical-combining-class))
"##,
        expect_test::expect![[r#""OK (0 230)""#]],
    );
}

#[test]
fn div_cx510_char_decomposition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (get-char-code-property #xC0 'decomposition)
      (get-char-code-property #xE9 'decomposition))
"##,
        expect_test::expect![[r#""OK ((65 768) (101 769))""#]],
    );
}

#[test]
fn div_cx510_char_uppercase_lowercase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (get-char-code-property ?a 'uppercase)
      (get-char-code-property ?A 'lowercase)
      (get-char-code-property ?a 'lowercase))
"##,
        expect_test::expect![[r#""OK (65 97 nil)""#]],
    );
}

#[test]
fn div_cx510_char_titlecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (get-char-code-property ?a 'titlecase)
      (get-char-code-property #x01C5 'titlecase))
"##,
        expect_test::expect![[r#""OK (65 453)""#]],
    );
}

#[test]
fn div_cx510_string_to_unibyte_surrogate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (string-to-unibyte "abc")
  (error (car e)))
"##,
        expect_test::expect![[r#""OK \"abc\"""#]],
    );
}

#[test]
fn div_cx510_string_as_multibyte_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s (string-as-multibyte (string-as-unibyte "cafe"))))
  (list (string-bytes s) (length s) (string= s "cafe")))
"##,
        expect_test::expect![[r#""OK (4 4 t)""#]],
    );
}

#[test]
fn div_cx510_make_string_zero_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (make-string 0 ?x) (string ?h ?e ?l ?l ?o) (length ""))
"##,
        expect_test::expect![[r#""OK (\"\" \"hello\" 0)""#]],
    );
}
