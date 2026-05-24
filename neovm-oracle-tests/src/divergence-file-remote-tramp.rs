//! Divergence tests: dired, tramp, remote files, and archive stubs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_dired_mode_vars_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'dired-listing-switches)
  (stringp dired-listing-switches)
  (boundp 'dired-recursive-deletes)
  (boundp 'dired-recursive-copies)
  (fboundp 'dired-get-marked-files)
  (fboundp 'dired-mark-files-regexp))"#,
    );
}

#[test]
fn divergence_tramp_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'tramp-dissect-file-name)
  (fboundp 'tramp-make-tramp-file-name)
  (featurep 'tramp)
  (fboundp 'tramp-connectible-p))"#,
    );
}

#[test]
fn divergence_remote_file_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'file-remote-p)
  (fboundp 'file-local-copy)
  (fboundp 'make-temp-file))"#,
    );
}

#[test]
fn divergence_archive_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'archive-zip-extract)
  (fboundp 'archive-extract)
  (featurep 'arc-mode))"#,
    );
}

#[test]
fn divergence_tilde_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (expand-file-name "~/")
  (string= (expand-file-name "~/") (concat (getenv "HOME") "/"))
  (expand-file-name "~root/")
  (file-name-absolute-p "~/test"))"#,
    );
}

#[test]
fn divergence_file_symlink() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((target (make-temp-file "neovm-sym-target-"))
        link)
  (unwind-protect
      (progn
        (setq link (make-temp-file "neovm-sym-link-"))
        (delete-file link)
        (make-symbolic-link target link)
        (list (stringp (file-symlink-p link))
              (file-exists-p link)
              (file-equal-p target link)))
    (when (file-exists-p target) (delete-file target))
    (when (file-exists-p link) (delete-file link))))"#,
    );
}

#[test]
fn divergence_file_truename() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((f (make-temp-file "neovm-truename-")))
  (unwind-protect
      (list (stringp (file-truename f))
            (file-exists-p f)
            (file-writable-p f))
    (delete-file f)))"#,
    );
}

#[test]
fn divergence_directory_files_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((dir (make-temp-file "neovm-dirattr-" t)))
  (unwind-protect
      (let* ((attrs (file-attributes dir))
             (entries (directory-files-and-attributes dir)))
        (list (consp attrs)
              (eq (car attrs) t)
              (listp entries)))
    (delete-directory dir t)))"#,
    );
}

#[test]
fn divergence_write_region_append() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((tmp (make-temp-file "neovm-append-")))
  (unwind-protect
      (progn
        (write-region "line1" nil tmp nil 'silent)
        (write-region "line2" nil tmp t 'silent)
        (list (with-temp-buffer
                (insert-file-contents tmp)
                (buffer-string))
              (file-attribute-size (file-attributes tmp))))
    (delete-file tmp)))"#,
    );
}

#[test]
fn divergence_file_locked() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'lock-buffer)
  (fboundp 'unlock-buffer)
  (fboundp 'file-locked-p))"#,
    );
}
