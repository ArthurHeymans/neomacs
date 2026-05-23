//! Divergence tests: string manipulation, substring, concat, case conversion.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_substring_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((s "Hello, World!"))
  (list (substring s 0 5)
        (substring s 7)
        (substring s -6)
        (substring s -6 -1)))"#,
    );
}

#[test]
fn divergence_substring_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((s (propertize "abcdef" 'face 'bold))
         (sub (substring s 2 4)))
  (list (get-text-property 0 'face sub)
        (get-text-property 0 'face s)
        (length sub)))"#,
    );
}

#[test]
fn divergence_string_bytes_vs_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((s "Héllo 世界"))
  (list (length s)
        (string-bytes s)
        (string-equal s "Héllo 世界")
        (string< "abc" "abd")
        (string> "abd" "abc")))"#,
    );
}

#[test]
fn divergence_case_conversion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (upcase "Hello World")
  (downcase "Hello World")
  (capitalize "hello world foo")
  (upcase-initials "hello world foo"))"#,
    );
}

#[test]
fn divergence_case_conversion_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list (upcase "Straße")
              (downcase "İSTANBUL")
              (capitalize "foo-bar baz"))"#,
    );
}

#[test]
fn divergence_string_multibyte_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((s (concat "abc" "中文" "def")))
  (list s
        (length s)
        (string-bytes s)
        (multibyte-string-p s)
        (string= s "abc中文def")))"#,
    );
}

#[test]
fn divergence_string_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((s (string ?a ?b 0xc0 ?d)))
  (list s
        (length s)
        (multibyte-string-p s)
        (string-to-multibyte s)
        (multibyte-string-p (string-to-multibyte s))))"#,
    );
}

#[test]
fn divergence_string_make_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((ms "Héllo")
         (us (string-make-unibyte ms)))
  (list (multibyte-string-p ms)
        (multibyte-string-p us)
        (length us)))"#,
    );
}

#[test]
fn divergence_string_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list\n  (string-replace \"world\" \"Emacs\" \"hello world\")\n  (string-replace \"o\" \"0\" \"foo boo moo\")\n  (replace-regexp-in-string \"[0-9]\" \"#\" \"a1b2c3\"))",
    );
}

#[test]
fn divergence_split_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (split-string "  foo bar  baz  " " +" t)
  (split-string "a,b,,c" ",")
  (split-string "foo-bar-baz" "-"))"#,
    );
}

#[test]
fn divergence_string_pad() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (string-pad "hi" 10)
  (string-pad "hi" 10 ?-)
  (string-pad "hello" 3))"#,
    );
}
