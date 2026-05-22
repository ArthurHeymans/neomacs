//! Divergence tests: rx macro, pcase pattern, pattern-matching edge cases.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_rx_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'rx)
  (fboundp 'rx-to-string)
  (rx-to-string 'bol)
  (rx-to-string 'eol)
  (rx-to-string '(any "aeiou"))) "#,
    );
}

#[test]
fn divergence_rx_composition() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (rx-to-string '(seq "foo" (optional "bar") "baz"))
  (rx-to-string '(or "cat" "dog"))
  (rx-to-string '(one-or-more (any "a-z")))
  (rx-to-string '(zero-or-more (any "0-9")))) "#,
    );
}

#[test]
fn divergence_rx_named_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (rx-to-string '(group-n 1 (one-or-more (any "a-z"))))
  (rx-to-string '(group (one-or-more (any "0-9"))))) "#,
    );
}

#[test]
fn divergence_rx_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (rx-to-string '(= 3 (any "a")))
  (rx-to-string '(>= 2 (any "b")))
  (rx-to-string '(** 1 5 (any "c")))) "#,
    );
}

#[test]
fn divergence_pcase_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (pcase 1 (1 'one) (2 'two))
  (pcase 'foo ('bar 'no) ('foo 'yes))
  (pcase '(1 2) ((list a b) (list a b)))) "#,
    );
}

#[test]
fn divergence_pcase_guard() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (pcase 5 ((guard (> it 3)) 'big) (_ 'small))
  (pcase '(1 2 3)
    (`(,a ,b ,c) (list a b c)))) "#,
    );
}

#[test]
fn divergence_pcase_or_and() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (pcase 3 ((or 1 2 3) 'match))
  (pcase '(1 2)
    ((and `(,a ,b) (guard (> a 0))) (list a b)))) "#,
    );
}

#[test]
fn divergence_pcase_app_let() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (pcase '(1 2)
    ((app car x) x))
  (pcase 42
    ((let x x) x))) "#,
    );
}

#[test]
fn divergence_pcase_pred() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (pcase "hello"
    ((pred stringp) 'string)
    (_ 'other))
  (pcase 42
    ((pred integerp) 'int)
    (_ 'other))) "#,
    );
}

#[test]
fn divergence_pcase_map_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (pcase [1 2 3]
    ((pred vectorp) 'vec)
    (_ 'other))
  (pcase '((a . 1) (b . 2))
    ((pred listp) 'list)
    (_ 'other))) "#,
    );
}
