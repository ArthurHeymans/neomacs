//! Divergence tests: print circle, read circle, gensym deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_print_circle_shared() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((x (list 1 2)))
  (let ((print-circle t))
    (list (prin1-to-string (list x x))
          (prin1-to-string (vector x x))))) "#,
    );
}

#[test]
fn divergence_print_circle_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((x (list 1 2)))
  (let ((print-circle nil))
    (list (prin1-to-string (list x x))
          (stringp (prin1-to-string x))))) "#,
    );
}

#[test]
fn divergence_read_back() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((obj '(a (b c) d)))
  (let ((s (prin1-to-string obj)))
    (list s (read-from-string s)
          (equal obj (car (read-from-string s)))))) "#,
    );
}

#[test]
fn divergence_gensym() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((g (gensym)))
  (list (symbolp g)
        (not (null g))
        (null (intern-soft (symbol-name g)))
        (let ((print-gensym t))
          (string-match "gensym" (prin1-to-string g))))) "#,
    );
}

#[test]
fn divergence_gensym_prefix() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((g (gensym "test-prefix-")))
  (list (symbolp g)
        (string-match "test-prefix" (symbol-name g))
        (not (eq g (gensym "test-prefix-"))))) "#,
    );
}

#[test]
fn divergence_print_gensym_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'print-gensym)
  (booleanp print-gensym)
  (boundp 'print-circle)
  (booleanp print-circle)) "#,
    );
}

#[test]
fn divergence_print_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'print-escape-newlines)
  (boundp 'print-escape-control-characters)
  (boundp 'print-escape-multibyte)
  (boundp 'print-escape-nonascii)
  (booleanp print-escape-newlines)
  (booleanp print-escape-control-characters)) "#,
    );
}

#[test]
fn divergence_prin1_princ() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (stringp (prin1-to-string "hello\nworld"))
  (stringp (princ-to-string "hello\nworld"))
  (not (equal (prin1-to-string "hello\nworld")
              (princ-to-string "hello\nworld")))) "#,
    );
}

#[test]
fn divergence_print_array() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((v [1 2 3])
        (s (prin1-to-string v)))
  (list s
        (equal (read-from-string s) (cons v 7))
        (vectorp (car (read-from-string s))))) "#,
    );
}

#[test]
fn divergence_print_string_special() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (prin1-to-string "hello\tworld")
  (prin1-to-string "quote\"inside")
  (prin1-to-string "back\\slash")
  (length (prin1-to-string "a"))
  (> (length (prin1-to-string "\n")) 1)) "#,
    );
}
