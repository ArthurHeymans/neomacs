//! Oracle parity tests for GNU invisibility-spec helper semantics.
//!
//! GNU implements `add-to-invisibility-spec` and
//! `remove-from-invisibility-spec` in `lisp/subr.el`.  They mutate the
//! buffer-local `buffer-invisibility-spec` using exact `t`/list conversion and
//! `delete` semantics.

use super::common::{
    assert_oracle_parity_with_bootstrap, return_if_neovm_enable_oracle_proptest_not_set,
};

#[test]
fn oracle_prop_gnu_invisibility_spec_helpers_preserve_exact_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((results nil))
  (dolist (initial '(t nil (t) (alpha beta alpha) ((outline . t) t)))
    (with-temp-buffer
      (setq buffer-invisibility-spec (copy-tree initial))
      (let ((add-ret (add-to-invisibility-spec 'alpha))
            (after-add buffer-invisibility-spec)
            (remove-ret (remove-from-invisibility-spec 'alpha))
            (after-remove buffer-invisibility-spec)
            (remove-missing-ret (remove-from-invisibility-spec 'missing))
            (after-remove-missing buffer-invisibility-spec))
        (push (list initial
                    add-ret
                    after-add
                    remove-ret
                    after-remove
                    remove-missing-ret
                    after-remove-missing)
              results))))
  (nreverse results))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_gnu_remove_from_invisibility_spec_converts_non_lists_to_t_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((results nil))
  (dolist (initial '(t nil hidden 42 "hidden"))
    (with-temp-buffer
      (setq buffer-invisibility-spec initial)
      (let ((ret (remove-from-invisibility-spec 'hidden)))
        (push (list initial ret buffer-invisibility-spec) results))))
  (nreverse results))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
