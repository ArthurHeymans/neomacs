//! Divergence tests: file operations deep, file attributes, directory ops.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_file_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((attrs (file-attributes ".")))
  (list (consp attrs)
        (length attrs)
        (file-directory-p ".")
        (file-symlink-p ".")
        (integerp (nth 7 attrs))))"#,
    );
}

#[test]
fn divergence_file_mtime_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((tmp (make-temp-file "neovm-mtime-"))
          (attrs (file-attributes tmp)))
  (unwind-protect
      (list (consp (nth 5 attrs))
            (integerp (nth 7 attrs))
            (= (nth 7 attrs) 0)
            (file-writable-p tmp)
            (file-readable-p tmp))
    (delete-file tmp)))"#,
    );
}

#[test]
fn divergence_expand_file_name_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (expand-file-name "foo/bar/../baz")
  (expand-file-name "~/test")
  (expand-file-name "./test")
  (expand-file-name "/absolute/path")
  (file-name-absolute-p "/foo")
  (file-name-absolute-p "foo"))"#,
    );
}

#[test]
fn divergence_file_name_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (file-name-directory "/a/b/c.txt")
  (file-name-nondirectory "/a/b/c.txt")
  (file-name-extension "test.el")
  (file-name-extension "test.tar.gz")
  (file-name-sans-extension "test.el")
  (file-name-base "test.el"))"#,
    );
}

#[test]
fn divergence_file_copy_rename() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((src (make-temp-file "neovm-copy-src-"))
        dst)
  (unwind-protect
      (progn
        (write-region "content" nil src nil 'silent)
        (setq dst (make-temp-file "neovm-copy-dst-"))
        (copy-file src dst t)
        (list (file-exists-p src)
              (file-exists-p dst)
              (with-temp-buffer
                (insert-file-contents dst)
                (buffer-string))))
    (when (file-exists-p src) (delete-file src))
    (when (file-exists-p dst) (delete-file dst))))"#,
    );
}

#[test]
fn divergence_make_directory() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((dir (make-temp-file "neovm-mkdir-" t)))
  (unwind-protect
      (list (file-directory-p dir)
            (file-exists-p dir)
            (directory-files dir)
            (length (directory-files dir)))
    (delete-directory dir t)))"#,
    );
}

#[test]
fn divergence_path_separators() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (stringp path-separator)
  (string= path-separator ":")
  (stringp directory-sep-char)
  (= directory-sep-char ?/))"#,
    );
}

#[test]
fn divergence_file_executable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (file-executable-p "/bin/ls")
  (file-executable-p "/bin/sh")
  (file-modes "/bin/ls")
  (integerp (file-modes "/bin/ls")))"#,
    );
}

#[test]
fn divergence_write_read_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((tmp (make-temp-file "neovm-rw-")))
  (unwind-protect
      (progn
        (write-region "Hello World" nil tmp nil 'silent)
        (list (with-temp-buffer
                (insert-file-contents tmp)
                (buffer-string))
              (file-attribute-size (file-attributes tmp))))
    (delete-file tmp)))"#,
    );
}

#[test]
fn divergence_insert_file_contents_partial() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((tmp (make-temp-file "neovm-partial-")))
  (unwind-protect
      (progn
        (write-region "ABCDEFGHIJ" nil tmp nil 'silent)
        (with-temp-buffer
          (insert-file-contents tmp nil 3 7)
          (buffer-string)))
    (delete-file tmp)))"#,
    );
}
