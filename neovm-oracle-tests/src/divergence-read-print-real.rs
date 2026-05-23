//! Divergence tests: real read/print behavioral differences.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_read_basic_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (read-from-string \"42\")
  (read-from-string \"3.14\")
  (read-from-string \"\\\"hello\\\"\")
  (read-from-string \"nil\")
  (read-from-string \"t\")
  (read-from-string \"(1 2 3)\")
  (read-from-string \"[1 2 3]\")
  (read-from-string \"?A\")) ",
    );
}

#[test]
fn divergence_print_circle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((print-circle t)
        (obj (list 'a 'b)))
  (nconc obj obj)
  (list (prin1-to-string obj)
        print-circle)) ",
    );
}

#[test]
fn divergence_print_gensym_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((sym (make-symbol \"temp\")))
  (list (symbolp sym)
        (symbol-name sym)
        (interned-p sym)
        (let ((print-gensym t))
          (prin1-to-string sym))
        (let ((print-gensym nil))
          (prin1-to-string sym)))) ",
    );
}

#[test]
fn divergence_print_escape_newlines() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((s \"hello\\nworld\"))
  (list s
        (let ((print-escape-newlines t))
          (prin1-to-string s))
        (let ((print-escape-newlines nil))
          (prin1-to-string s)))) ",
    );
}

#[test]
fn divergence_format_specifiers_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (format \"hello %s\" \"world\")
  (format \"count: %d\" 42)
  (format \"pi: %.2f\" 3.14159)
  (format \"char: %c\" 65)
  (format \"%% literal\")
  (format \"%10s\" \"hi\")
  (format \"%-10s|\" \"hi\")
  (format \"%05d\" 7)) ",
    );
}

#[test]
fn divergence_read_nested_structures() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let* ((data '((1 (2 3)) [4 5] \"six\" :kw))
        (printed (prin1-to-string data))
        (re-read (read-from-string printed)))
  (list (equal data (car re-read))
        (cdr re-read)
        (= (cdr re-read) (length printed)))) ",
    );
}

#[test]
fn divergence_print_length_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((print-length 3))
  (list (prin1-to-string '(1 2 3 4 5))
        (prin1-to-string '[1 2 3 4 5])
        (prin1-to-string \"abcdefghij\"))) ",
    );
}

#[test]
fn divergence_print_level_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((print-level 2))
  (list (prin1-to-string '(1 (2 (3 (4)))))
        (prin1-to-string '((a (b (c))))))) ",
    );
}

#[test]
fn divergence_princ_vs_prin1() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((s \"hello\"))
  (list (prin1-to-string s)
        (with-output-to-string (princ s))
        (substring (prin1-to-string '(a \"b\" 3)) 0))) ",
    );
}

#[test]
fn divergence_print_escape_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let* ((s \"caf\\u00e9\")
        (p1 (prin1-to-string s))
        (r1 (car (read-from-string p1))))
  (list (equal s r1)
        (string= s r1)
        p1
        (length s)
        (length r1))) ",
    );
}
