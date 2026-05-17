//! Oracle parity tests for GNU `file-in-directory-p` semantics.
//!
//! GNU implements this in `lisp/files.el`: `DIR` must be an existing
//! directory, then both arguments are resolved through `file-truename`, split
//! into path components, and the common root is confirmed with `file-equal-p`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_in_directory_symlink_missing_and_self_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((dir (make-temp-file "neomacs-oracle-file-in-dir-" t))
       (root (expand-file-name "root" dir))
       (child (expand-file-name "child" root))
       (other (expand-file-name "other" dir))
       (file (expand-file-name "file.txt" child))
       (link-root (expand-file-name "link-root" dir))
       (link-child-file (expand-file-name "link-child-file" dir))
       (missing-under-link (expand-file-name "child/missing.txt" link-root))
       (missing-dir (expand-file-name "missing-dir" dir)))
  (unwind-protect
      (progn
        (make-directory child t)
        (make-directory other)
        (write-region "x" nil file nil 'silent)
        (make-symbolic-link "root" link-root)
        (make-symbolic-link "root/child/file.txt" link-child-file)
        (list
         ;; A directory is considered a parent of itself.
         (file-in-directory-p root root)
         (file-in-directory-p (file-name-as-directory root) root)
         ;; Direct descendants and symlinked parents resolve through
         ;; `file-truename`.
         (file-in-directory-p file root)
         (file-in-directory-p file link-root)
         (file-in-directory-p link-child-file root)
         (file-in-directory-p missing-under-link root)
         ;; Existing sibling directories and missing directory arguments are
         ;; rejected.
         (file-in-directory-p file other)
         (file-in-directory-p file missing-dir)
         (file-in-directory-p missing-under-link missing-dir)
         ;; A missing file can still be inside an existing directory because
         ;; `file-truename` preserves the missing suffix.
         (file-in-directory-p (expand-file-name "missing.txt" child) root)
         (condition-case err
             (file-in-directory-p 42 root)
           (error (list (car err) (cdr err))))
         (condition-case err
             (file-in-directory-p file 42)
           (error (list (car err) (cdr err)))))))
    (ignore-errors (delete-file link-child-file))
    (ignore-errors (delete-file link-root))
    (ignore-errors (delete-file file))
    (ignore-errors (delete-directory child))
    (ignore-errors (delete-directory root))
    (ignore-errors (delete-directory other))
    (ignore-errors (delete-directory dir))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
