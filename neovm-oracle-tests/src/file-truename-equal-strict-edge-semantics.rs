//! Oracle parity tests for GNU `file-truename` and `file-equal-p` semantics.
//!
//! GNU implements `file-truename` and `file-equal-p` in `lisp/files.el`.
//! `file-truename` recursively resolves parent directory symlinks before the
//! leaf, preserves missing suffixes after the last existing component, and
//! treats `.` / `..` after resolving the parent directory.  `file-equal-p`
//! compares `file-attributes` after resolving truenames.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_truename_parent_symlink_missing_tail_and_file_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((dir (make-temp-file "neomacs-oracle-truename-" t))
       (real-dir (expand-file-name "real" dir))
       (nested-dir (expand-file-name "nested" real-dir))
       (target (expand-file-name "target.txt" nested-dir))
       (link-dir (expand-file-name "link-dir" dir))
       (link-file (expand-file-name "link-file" dir))
       (dangling-link (expand-file-name "dangling-link" dir)))
  (unwind-protect
      (progn
        (make-directory nested-dir t)
        (write-region "target" nil target nil 'silent)
        (make-symbolic-link "real" link-dir)
        (make-symbolic-link "real/nested/target.txt" link-file)
        (make-symbolic-link "real/nested/missing.txt" dangling-link)
        (list
         ;; Direct symlink leaf resolution.
         (file-relative-name (file-truename link-file) dir)
         ;; Parent-directory symlink resolution before the leaf.
         (file-relative-name
          (file-truename (expand-file-name "nested/target.txt" link-dir))
          dir)
         ;; Missing suffixes are preserved after resolving existing symlinked
         ;; parents.
         (file-relative-name
          (file-truename (expand-file-name "nested/missing-tail.txt" link-dir))
          dir)
         ;; `.' and `..' are interpreted after parent truename resolution.
         (file-relative-name
          (directory-file-name (file-truename (expand-file-name "." link-dir)))
          dir)
         (file-relative-name
          (directory-file-name (file-truename (expand-file-name ".." link-dir)))
          dir)
         ;; `file-equal-p' resolves truenames before comparing attributes.
         (file-equal-p target link-file)
         (file-equal-p target (expand-file-name "nested/target.txt" link-dir))
         (file-equal-p target dangling-link)
         (file-equal-p target (expand-file-name "missing.txt" dir))
         (condition-case err
             (file-truename 42)
           (error (list (car err) (cdr err))))
         (condition-case err
             (file-equal-p target 42)
           (error (list (car err) (cdr err))))))
    (ignore-errors (delete-file dangling-link))
    (ignore-errors (delete-file link-file))
    (ignore-errors (delete-file link-dir))
    (ignore-errors (delete-file target))
    (ignore-errors (delete-directory nested-dir))
    (ignore-errors (delete-directory real-dir))
    (ignore-errors (delete-directory dir))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
