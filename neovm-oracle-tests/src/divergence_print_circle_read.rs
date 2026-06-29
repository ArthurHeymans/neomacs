//! Divergence tests: print-circle, read-circle, circular structure deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_print_circle_shared() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-circle t)
        (shared (list 1 2 3)))
  (prin1-to-string (list shared shared)))"#,
        expect_test::expect![[r#""OK \"(#1=(1 2 3) #1#)\"""#]],
    );
}

#[test]
fn divergence_print_circle_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-circle nil)
        (shared (list 1 2 3)))
  (prin1-to-string (list shared shared)))"#,
        expect_test::expect![[r#""OK \"((1 2 3) (1 2 3))\"""#]],
    );
}

#[test]
fn divergence_read_circle_shared() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-circle t)
        (x (list 1 2))
        (y (vector 'a 'b)))
  (aset y 0 x)
  (aset y 1 x)
  (let* ((s (prin1-to-string y))
         (r (read s)))
    (list (eq (aref r 0) (aref r 1))
          s)))"#,
        expect_test::expect![[r#""OK (t \"[#1=(1 2) #1#]\")""#]],
    );
}

#[test]
fn divergence_print_length_ellipsis() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-length 3))
  (prin1-to-string '(1 2 3 4 5)))"#,
        expect_test::expect![[r#""OK \"(1 2 3 ...)\"""#]],
    );
}

#[test]
fn divergence_print_level_ellipsis() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-level 2))
  (prin1-to-string '(a (b (c (d))) e)))"#,
        expect_test::expect![[r#""OK \"(a (b ...) e)\"""#]],
    );
}

#[test]
fn divergence_print_escape_newlines() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-escape-newlines t))
  (prin1-to-string "line1\nline2"))"#,
        expect_test::expect![[r#""OK \"\\\"line1\\\\nline2\\\"\"""#]],
    );
}

#[test]
fn divergence_print_escape_nonascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-escape-nonascii t))
  (prin1-to-string "café"))"#,
        expect_test::expect![[r#""OK \"\\\"café\\\"\"""#]],
    );
}

#[test]
fn divergence_print_quoted() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-quoted t))
  (list (prin1-to-string ''foo)
        (prin1-to-string '(lambda (x) x))))"#,
        expect_test::expect![[r#""OK (\"'foo\" \"(lambda (x) x)\")""#]],
    );
}

#[test]
fn divergence_print_gensym_uninterned() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((print-gensym t)
        (sym (make-symbol "test")))
  (list (intern-soft "test")
        (symbol-name sym)
        (prin1-to-string sym)))"#,
        expect_test::expect![[r##""OK (test \"test\" \"#:test\")""##]],
    );
}

#[test]
fn divergence_print_float_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((float-output-format "%.2f"))
  (list (prin1-to-string 3.14159)
        (prin1-to-string 1.0)
        (prin1-to-string 0.5)))"#,
        expect_test::expect![[r#""OK (\"3.14\" \"1.00\" \"0.50\")""#]],
    );
}
