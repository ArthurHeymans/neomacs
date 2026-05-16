//! Oracle parity tests for GNU repository metadata helpers.
//!
//! GNU implements these in `lisp/version.el`.  In the studied GNU source,
//! `emacs-repository-get-version` and `emacs-repository-get-branch` accept
//! optional directory arguments, while `emacs-repository-get-dirty` is absent.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_repository_metadata_optional_args_and_dirty_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((dir temporary-file-directory))
  (list
   (condition-case err
       (emacs-repository-get-version dir)
     (error (cons (car err) (cdr err))))
   (condition-case err
       (emacs-repository-get-branch dir)
     (error (cons (car err) (cdr err))))
   (condition-case err
       (emacs-repository-get-version dir t)
     (error (cons (car err) (cdr err))))
   (fboundp 'emacs-repository-get-dirty)
   (condition-case err
       (emacs-repository-get-dirty dir)
     (error (cons (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
