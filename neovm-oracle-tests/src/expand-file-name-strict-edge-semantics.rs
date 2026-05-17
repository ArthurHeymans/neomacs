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
