//! Oracle parity tests for GNU `expand-file-name` edge semantics.
//!
//! GNU implements `expand-file-name` in `src/fileio.c`.  It consults the
//! Lisp-visible environment via `get_homedir`, preserves unregistered `~USER`
//! spellings as relative path components, falls back to root when
//! `default-directory` is not a string, and rejects null bytes before path
//! canonicalization.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_expand_file_name_home_default_and_null_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((process-environment (copy-sequence process-environment))
      (default-directory "/tmp/neomacs-oracle-base/dir/"))
  (setenv "HOME" "/tmp/neomacs-oracle-home")
  (list
   (expand-file-name "~")
   (expand-file-name "~/alpha/../beta")
   (expand-file-name "~definitely-no-such-neomacs-oracle-user/leaf"
                     "/tmp/neomacs-oracle-base/")
   (expand-file-name "" "/tmp/neomacs-oracle-base/dir/")
   (expand-file-name "relative" nil)
   (let ((default-directory nil))
     (expand-file-name "relative"))
   (expand-file-name "../x/./y/" "/tmp/neomacs-oracle-base/dir")
   (expand-file-name "///triple//slash" "/ignored")
   (condition-case err
       (expand-file-name 42)
     (error (list (car err) (cdr err))))
   (condition-case err
       (expand-file-name (string ?a 0 ?b) "/tmp/neomacs-oracle-base/")
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_expand_file_name_root_and_relative_default_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 ;; GNU documents this root behavior explicitly: `..' is not always a
 ;; filesystem parent traversal after canonicalization.
 (expand-file-name ".." "/")
 (expand-file-name "../.." "/")
 (expand-file-name "a/../../b" "/")
 ;; Relative `default-directory' values are first expanded against
 ;; `invocation-directory' when the buffer default is used.
 (let ((default-directory "relative/base/"))
   (let ((expanded (expand-file-name "leaf")))
     (list (file-name-absolute-p expanded)
           (string-suffix-p "/relative/base/leaf" expanded))))
 ;; Explicit relative DEFAULT-DIRECTORY is recursively expanded against the
 ;; current buffer default directory, then NAME is appended.
 (let ((expanded (expand-file-name "leaf" "relative/default")))
   (list (file-name-absolute-p expanded)
         (string-suffix-p "/relative/default/leaf" expanded)))
 ;; A non-string buffer-local `default-directory' falls back to root.
 (let ((default-directory 42))
   (expand-file-name "leaf"))
 ;; An explicit bad DEFAULT-DIRECTORY is checked before that fallback.
 (condition-case err
     (expand-file-name "leaf" 42)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
