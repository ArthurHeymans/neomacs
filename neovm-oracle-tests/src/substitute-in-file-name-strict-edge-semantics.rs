//! Oracle parity tests for GNU `substitute-in-file-name` semantics.
//!
//! GNU implements the path discard rules in `src/fileio.c` and delegates
//! environment-variable syntax to `lisp/env.el`.  Undefined variables remain
//! unchanged for this API, `$$` becomes `$`, and embedded absolute file names
//! such as `//...`, `/~`, or a substituted absolute path discard the prefix.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_substitute_in_file_name_env_and_embedded_absolute_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((process-environment process-environment))
  (setenv "NEOMACS_ORACLE_SUBST" "value")
  (setenv "NEOMACS_ORACLE_ABS_SUBST" "/abs/value")
  (setenv "NEOMACS_ORACLE_EMPTY_SUBST" "")
  (setenv "NEOMACS_ORACLE_UNDEF_SUBST" nil)
  (list
   (substitute-in-file-name "$NEOMACS_ORACLE_SUBST/end")
   (substitute-in-file-name "${NEOMACS_ORACLE_SUBST}/end")
   (substitute-in-file-name "$NEOMACS_ORACLE_SUBST_suffix")
   (substitute-in-file-name "${NEOMACS_ORACLE_SUBST}_suffix")
   (substitute-in-file-name "$NEOMACS_ORACLE_EMPTY_SUBST/end")
   (substitute-in-file-name "$NEOMACS_ORACLE_UNDEF_SUBST/end")
   (substitute-in-file-name "$$NEOMACS_ORACLE_SUBST")
   (substitute-in-file-name "$")
   (substitute-in-file-name "$-literal")
   (substitute-in-file-name "${}")
   (substitute-in-file-name "${NEOMACS_ORACLE_SUBST")
   (substitute-in-file-name "prefix//tail")
   (substitute-in-file-name "prefix/~user/tail")
   ;; Absolute results from variable substitution discard the prefix.
   (substitute-in-file-name "prefix/$NEOMACS_ORACLE_ABS_SUBST/tail")
   (condition-case err
       (substitute-in-file-name)
     (error (list (car err) (cdr err))))
   (condition-case err
       (substitute-in-file-name 42)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
