//! Divergence tests: minibuffer, completion, and ring data structure.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_minibuffer_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (booleanp minibuf-window)
  (windowp minibuf-window)
  (integerp minibuffer-depth-indicator-function)
  (booleanp enable-recursive-minibuffers))"#,
    );
}

#[test]
fn divergence_minibuffer_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (windowp (minibuffer-window))
  (window-live-p (minibuffer-window))
  (bufferp (window-buffer (minibuffer-window)))
  (minibuffer-window-active-p (minibuffer-window))
  (window-minibuffer-p (minibuffer-window)))"#,
    );
}

#[test]
fn divergence_minibuffer_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (stringp (minibuffer-prompt-width))
  (fboundp 'minibuffer-contents)
  (fboundp 'minibuffer-contents-no-properties))"#,
    );
}

#[test]
fn divergence_completion_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'try-completion)
  (fboundp 'all-completions)
  (fboundp 'completion-boundaries)
  (string= (try-completion "fo" '(("foo") ("bar") ("foobar"))) "foobar"))"#,
    );
}

#[test]
fn divergence_all_completions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((coll '(("alpha") ("beta") ("gamma") ("alphabetic"))))
  (list (sort (all-completions "al" coll) #'string<)
        (try-completion "al" coll)))"#,
    );
}

#[test]
fn divergence_completion_regexp_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((completion-regexp-list '("lph")))
  (list (all-completions "" '(("alpha") ("beta") ("gamma")))
        completion-regexp-list))"#,
    );
}

#[test]
fn divergence_ring_create_and_access() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'ring)
(let ((r (make-ring 5)))
  (ring-insert r 'a)
  (ring-insert r 'b)
  (ring-insert r 'c)
  (list (ring-ref r 0)
        (ring-ref r 1)
        (ring-ref r 2)
        (ring-length r)
        (ring-size r)))"#,
    );
}

#[test]
fn divergence_ring_overflow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'ring)
(let ((r (make-ring 3)))
  (dotimes (i 5)
    (ring-insert r i))
  (list (ring-ref r 0)
        (ring-ref r 1)
        (ring-ref r 2)
        (ring-length r)
        (ring-size r)))"#,
    );
}

#[test]
fn divergence_ring_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'ring)
(let ((r (make-ring 5)))
  (ring-insert r 'a)
  (ring-insert r 'b)
  (ring-insert r 'c)
  (ring-remove r 1)
  (list (ring-ref r 0)
        (ring-ref r 1)
        (ring-length r)))"#,
    );
}

#[test]
fn divergence_ring_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'ring)
(let ((r (make-ring 5)))
  (ring-insert r 3)
  (ring-insert r 1)
  (ring-insert r 4)
  (ring-insert r 1)
  (ring-insert r 5)
  (list (ring-elements r)
        (ring-copy r)))"#,
    );
}
