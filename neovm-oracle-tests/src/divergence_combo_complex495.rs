/// Batch 495: compiling-before-save, before-save-hook, write-file, save-some-buffers.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx495_before_save_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(boundp 'before-save-hook)
"##,
    );
}

#[test]
fn div_cx495_write_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx495-wf-")))
  (unwind-protect
      (progn (write-region "test" nil f)
             (with-temp-buffer (insert-file-contents f) (buffer-string)))
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx495_save_some_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (save-some-buffers t)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx495_revert_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx495-rb-")))
  (unwind-protect
      (with-temp-file f
        (insert "original")
        (with-temp-buffer
          (insert "modified")
          (revert-buffer nil t)
          (buffer-string)))
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx495_insert_file_visit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx495-ifv-")))
  (with-temp-file f (insert "hello"))
  (unwind-protect
      (with-temp-buffer
        (let ((result (insert-file-contents f)))
          (list (nth 1 result) (buffer-string))))
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx495_file_local_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx495-flc-")))
  (unwind-protect
      (file-local-copy f)
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx495_file_name_directory() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (file-name-directory "/a/b/c.el")
      (file-name-nondirectory "/a/b/c.el")
      (file-name-base "/a/b/c.el")
      (file-name-extension "/a/b/c.el"))
"##,
    );
}

#[test]
fn div_cx495_file_name_sans() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (file-name-sans-extension "foo.tar.gz")
      (file-name-sans-extension "foo.tar.gz" t)
      (file-name-base "foo.tar.gz"))
"##,
    );
}

#[test]
fn div_cx495_directory_files() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((d (make-temp-file "cx495-df-" t)))
  (with-temp-file (expand-file-name "a" d) (insert "x"))
  (unwind-protect
      (length (directory-files d nil "\\.*" t))
    (delete-directory d t)))
"##,
    );
}

#[test]
fn div_cx495_directory_files_and_attrs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((d (make-temp-file "cx495-dfa-" t)))
  (with-temp-file (expand-file-name "f" d) (insert "x"))
  (unwind-protect
      (let ((attrs (directory-files-and-attributes d nil nil t)))
        (length attrs))
    (delete-directory d t)))
"##,
    );
}

#[test]
fn div_cx495_file_attr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx495-fa-")))
  (unwind-protect
      (file-exists-p f)
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx495_file_modes_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx495-fms-")))
  (unwind-protect
      (set-file-modes f #o600)
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx495_file_selinux() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx495-fse-")))
  (unwind-protect
      (condition-case e (file-selinux-context f) (error (car e)))
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx495_file_acl() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx495-fac-")))
  (unwind-protect
      (condition-case e (file-acl f) (error (car e)))
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx495_file_notify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'file-notify--library) (fboundp 'file-notify-add-watch))
"##,
    );
}
