//! Oracle parity tests for GNU file predicate symlink semantics.
//!
//! GNU implements these predicates in `src/fileio.c`.  The important split is
//! that `file-symlink-p` reads the link itself and returns the raw target
//! string, while existence/readability/directory/regular predicates follow
//! symlinks and therefore treat dangling links as missing targets.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_predicates_symlink_and_missing_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((dir (make-temp-file "neomacs-oracle-file-pred-" t))
       (file (expand-file-name "plain" dir))
       (subdir (expand-file-name "subdir" dir))
       (file-link (expand-file-name "file-link" dir))
       (dir-link (expand-file-name "dir-link" dir))
       (dangling-link (expand-file-name "dangling-link" dir))
       (missing (expand-file-name "missing" dir)))
  (unwind-protect
      (progn
        (write-region "x" nil file nil 'silent)
        (make-directory subdir)
        (make-symbolic-link "plain" file-link)
        (make-symbolic-link "subdir" dir-link)
        (make-symbolic-link "missing" dangling-link)
        (list
         (file-exists-p file)
         (file-readable-p file)
         (file-writable-p file)
         (file-executable-p subdir)
         (file-directory-p subdir)
         (file-regular-p file)
         (file-symlink-p file)
         (file-symlink-p file-link)
         (file-symlink-p dir-link)
         (file-symlink-p dangling-link)
         (file-exists-p file-link)
         (file-readable-p file-link)
         (file-regular-p file-link)
         (file-directory-p dir-link)
         (file-regular-p dir-link)
         ;; GNU follows the target for existence/readability/regular tests,
         ;; so a dangling symlink is still a symlink but not an existing file.
         (file-exists-p dangling-link)
         (file-readable-p dangling-link)
         (file-regular-p dangling-link)
         (file-directory-p dangling-link)
         (file-exists-p missing)
         (file-symlink-p missing)
         (file-directory-p "")
         (condition-case err
             (file-exists-p)
           (error (list (car err) (cdr err))))
         (condition-case err
             (file-symlink-p 42)
           (error (list (car err) (cdr err))))))
    (ignore-errors (delete-file file-link))
    (ignore-errors (delete-file dir-link))
    (ignore-errors (delete-file dangling-link))
    (ignore-errors (delete-file file))
    (ignore-errors (delete-directory subdir))
    (ignore-errors (delete-directory dir))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
