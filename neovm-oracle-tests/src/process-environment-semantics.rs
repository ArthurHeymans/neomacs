//! Oracle parity tests for GNU `process-environment` semantics.
//!
//! GNU layers `setenv`, `getenv`, and `substitute-env-in-file-name` in
//! `lisp/env.el` over `getenv-internal` from `src/callproc.c`.  The central
//! contract is that Lisp-visible `process-environment` is authoritative for
//! let-bound environment changes.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_setenv_mutates_let_bound_process_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((process-environment (copy-sequence process-environment)))
  (setenv "NEOMACS_ORACLE_ENV_A" "one")
  (setenv "NEOMACS_ORACLE_ENV_B" "two")
  (list
   (getenv "NEOMACS_ORACLE_ENV_A")
   (getenv "NEOMACS_ORACLE_ENV_B")
   (seq-filter (lambda (entry)
                 (and (stringp entry)
                      (string-match-p "\\`NEOMACS_ORACLE_ENV_[AB]\\(=\\|\\'\\)" entry)))
               process-environment)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_setenv_nil_creates_negative_entry() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((process-environment (copy-sequence process-environment)))
  (setenv "NEOMACS_ORACLE_ENV_NEG" "present")
  (let ((before process-environment))
    (setenv "NEOMACS_ORACLE_ENV_NEG")
    (list
     (getenv "NEOMACS_ORACLE_ENV_NEG")
     (car process-environment)
     (getenv-internal "NEOMACS_ORACLE_ENV_NEG" process-environment)
     (not (equal before process-environment)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_getenv_internal_explicit_env_list_first_match_and_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((env '("A=first" "B" "A=second" "C=")))
  (list
   (getenv-internal "A" env)
   (getenv-internal "B" env)
   (getenv-internal "C" env)
   (getenv-internal "D" env)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_substitute_env_in_file_name_uses_lisp_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((process-environment (copy-sequence process-environment)))
  (setenv "NEOMACS_ORACLE_ENV_DIR" "/tmp/env-root")
  (setenv "NEOMACS_ORACLE_ENV_LEAF" "leaf")
  (list
   (substitute-env-in-file-name "$NEOMACS_ORACLE_ENV_DIR/$NEOMACS_ORACLE_ENV_LEAF")
   (substitute-env-in-file-name "${NEOMACS_ORACLE_ENV_DIR}/x")
   (substitute-env-in-file-name "$NEOMACS_ORACLE_ENV_MISSING/x")))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
