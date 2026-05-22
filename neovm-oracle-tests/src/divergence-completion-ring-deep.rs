//! Divergence tests: minibuffer history, ring operations, completion deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_ring_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((ring (make-ring 5)))
  (ring-insert ring 'a)
  (ring-insert ring 'b)
  (ring-insert ring 'c)
  (list (ring-length ring)
        (ring-ref ring 0)
        (ring-ref ring 1)
        (ring-ref ring 2)
        (ring-size ring))) "#,
    );
}

#[test]
fn divergence_ring_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((ring (make-ring 5)))
  (ring-insert ring 'a)
  (ring-insert ring 'b)
  (ring-insert ring 'c)
  (ring-remove ring 0)
  (list (ring-length ring)
        (ring-ref ring 0)
        (ring-ref ring 1)
        (ring-elements ring))) "#,
    );
}

#[test]
fn divergence_ring_overflow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((ring (make-ring 3)))
  (ring-insert ring 'a)
  (ring-insert ring 'b)
  (ring-insert ring 'c)
  (ring-insert ring 'd)
  (list (ring-length ring)
        (ring-size ring)
        (ring-ref ring 0)
        (ring-elements ring))) "#,
    );
}

#[test]
fn divergence_minibuffer_history() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'minibuffer-history)
  (listp minibuffer-history)
  (boundp 'file-name-history)
  (listp file-name-history)
  (fboundp 'add-to-history)) "#,
    );
}

#[test]
fn divergence_history_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'history-length)
  (integerp history-length)
  (boundp 'history-delete-duplicates)
  (booleanp history-delete-duplicates)) "#,
    );
}

#[test]
fn divergence_completion_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'try-completion)
  (fboundp 'all-completions)
  (fboundp 'test-completion)
  (fboundp 'completion-boundaries))"#,
    );
}

#[test]
fn divergence_completion_try() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((coll '(\"apple\" \"apricot\" \"banana\" \"cherry\")))
  (list (try-completion "ap" coll)
        (try-completion "b" coll)
        (try-completion "z" coll)
        (all-completions "ap" coll)
        (test-completion "apple" coll)
        (test-completion "appl" coll))) "#,
    );
}

#[test]
fn divergence_completion_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((completions (all-completions "ca" obarray)))
  (list (member "car" completions)
        (member "cdr" completions)
        (member "catch" completions)
        (listp completions))) "#,
    );
}

#[test]
fn divergence_completion_ignore_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((coll '(\"Hello\" \"HELLO\" \"hello\")))
  (list (try-completion "hel" coll)
        (try-completion "HEL" coll)
        (all-completions "hel" coll)
        (all-completions "HEL" coll))) "#,
    );
}

#[test]
fn divergence_completion_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'completion-metadata)
  (fboundp 'completion-try-completion)
  (fboundp 'completion-all-completions)
  (fboundp 'completion--field-completion-function)) "#,
    );
}
