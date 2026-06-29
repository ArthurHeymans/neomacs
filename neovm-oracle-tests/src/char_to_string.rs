//! Oracle parity tests for `char-to-string` and `string-to-char`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

#[test]
fn oracle_prop_char_to_string_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(char-to-string ?A)",
        expect_test::expect![[r#""OK \"A\"""#]],
    );
    assert_ok_eq(r#""A""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(char-to-string ?z)",
        expect_test::expect![[r#""OK \"z\"""#]],
    );
    assert_ok_eq(r#""z""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(char-to-string ?0)",
        expect_test::expect![[r#""OK \"0\"""#]],
    );
    assert_ok_eq(r#""0""#, &o, &n);
}

#[test]
fn oracle_prop_char_to_string_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(char-to-string ?\\s)",
        expect_test::expect![[r#""OK \" \"""#]],
    );
    assert_ok_eq(r#"" ""#, &o, &n);
}

#[test]
fn oracle_prop_string_to_char_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-to-char "A")"#,
        expect_test::expect![[r#""OK 65""#]],
    );
    assert_ok_eq("65", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-to-char "hello")"#,
        expect_test::expect![[r#""OK 104""#]],
    );
    assert_ok_eq("104", &o, &n);
}

#[test]
fn oracle_prop_string_to_char_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-to-char "")"#,
        expect_test::expect![[r#""OK 0""#]],
    );
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_prop_char_to_string_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-to-char (char-to-string ?X))"#,
        expect_test::expect![[r#""OK 88""#]],
    );
    assert_ok_eq("88", &o, &n);
}

#[test]
fn oracle_prop_char_to_string_raw_byte_character_roundtrip_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU editfns.c:Fchar_to_string validates with CHECK_CHARACTER and then
    // uses CHAR_STRING/make_string_from_bytes, preserving raw-byte character
    // codes as multibyte characters rather than collapsing them to unibyte
    // byte values.
    let form = r#"
(let ((raw (char-to-string #x3fff80))
      (unicode (char-to-string 233)))
  (list
   (multibyte-string-p raw)
   (string-bytes raw)
   (string-to-char raw)
   (multibyte-string-p unicode)
   (string-bytes unicode)
   (string-to-char unicode)))
"#;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![r#""OK (t 2 4194176 t 2 233)""#],
    );
}

#[test]
fn oracle_prop_char_to_string_in_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r####"(concat (char-to-string ?H) (char-to-string ?i) (char-to-string ?!))"####;
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        form,
        expect_test::expect![[r#""OK \"Hi!\"""#]],
    );
    assert_ok_eq(r#""Hi!""#, &o, &n);
}
