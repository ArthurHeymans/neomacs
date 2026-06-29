//! Divergence tests: real file & directory behavioral differences.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_file_name_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (file-name-directory \"/foo/bar/baz.txt\")
  (file-name-nondirectory \"/foo/bar/baz.txt\")
  (file-name-sans-extension \"/foo/bar/baz.txt\")
  (file-name-extension \"baz.txt\")
  (file-name-extension \"baz.tar.gz\")
  (file-name-base \"/foo/bar/baz.txt\")) ",
        expect_test::expect![[
            r#""OK (\"/foo/bar/\" \"baz.txt\" \"/foo/bar/baz\" \"txt\" \"gz\" \"baz\")""#
        ]],
    );
}

#[test]
fn divergence_expand_file_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((default-directory \"/tmp/\"))
  (list (expand-file-name \"foo.txt\")
        (expand-file-name \"foo.txt\" \"/var\")
        (expand-file-name \"../foo.txt\" \"/var/log/\")
        (expand-file-name \"./foo.txt\" \"/var/log/\"))) ",
        expect_test::expect![[
            r#""OK (\"/tmp/foo.txt\" \"/var/foo.txt\" \"/var/foo.txt\" \"/var/log/foo.txt\")""#
        ]],
    );
}

#[test]
fn divergence_file_attributes_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (file-exists-p \"/tmp\")
  (file-directory-p \"/tmp\")
  (file-readable-p \"/tmp\")
  (file-writable-p \"/tmp\")) ",
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_path_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (concat (file-name-as-directory \"/foo\") \"bar\")
  (file-name-as-directory \"/foo\")
  (directory-file-name \"/foo/\")
  (directory-file-name \"/foo\")) ",
        expect_test::expect![[r#""OK (\"/foo/bar\" \"/foo/\" \"/foo\" \"/foo\")""#]],
    );
}

#[test]
fn divergence_file_truename_tilde() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((h1 (expand-file-name \"~\"))
        (h2 (expand-file-name \"~/\")))
  (list (string= h1 h2)
        (string-prefix-p \"/\" h1)
        (string-prefix-p \"/\" h2)
        (> (length h1) 1))) ",
        expect_test::expect![[r#""OK (nil t t t)""#]],
    );
}

#[test]
fn divergence_make_temp_files() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((tmp1 (make-temp-file \"test-neovm-\")))
  (let ((exists (file-exists-p tmp1)))
    (write-region \"hello\" nil tmp1 nil 'silent)
    (let ((contents (with-temp-buffer
                      (insert-file-contents tmp1)
                      (buffer-string))))
      (delete-file tmp1)
      (list exists
            contents
            (file-exists-p tmp1))))) ",
        expect_test::expect![[r#""OK (t \"hello\" nil)""#]],
    );
}

#[test]
fn divergence_directory_listing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((tmp (make-temp-file \"test-dir-\")))
  (delete-file tmp)
  (make-directory tmp)
  (write-region \"a\" nil (expand-file-name \"a\" tmp) nil 'silent)
  (write-region \"b\" nil (expand-file-name \"b\" tmp) nil 'silent)
  (let ((files (directory-files tmp nil \"^[a-z]+$\"))
        (full (directory-files tmp t \"^[a-z]+$\")))
    (delete-file (expand-file-name \"a\" tmp))
    (delete-file (expand-file-name \"b\" tmp))
    (delete-directory tmp)
    (list (sort files #'string<)
          (length full)))) ",
        expect_test::expect![[r#""OK ((\"a\" \"b\") 2)""#]],
    );
}

#[test]
fn divergence_insert_file_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((tmp (make-temp-file \"test-read-\")))
  (write-region \"Hello World\" nil tmp nil 'silent)
  (let ((result (insert-file-contents tmp)))
    (delete-file tmp)
    (list (cadr result) (buffer-string)))) ",
        expect_test::expect![[r#""Hello WorldOK (11 \"Hello World\")""#]],
    );
}

#[test]
fn divergence_write_region_append() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((tmp (make-temp-file \"test-append-\")))
  (write-region \"line1\\n\" nil tmp nil 'silent)
  (write-region \"line2\\n\" nil tmp 'append 'silent)
  (let ((contents (with-temp-buffer
                    (insert-file-contents tmp)
                    (buffer-string))))
    (delete-file tmp)
    (list (split-string contents \"\\n\" t)
          (length contents)))) ",
        expect_test::expect![[r#""OK ((\"line1\" \"line2\") 12)""#]],
    );
}

#[test]
fn divergence_file_symlink() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((tmp1 (make-temp-file \"test-link-target-\")))
  (write-region \"data\" nil tmp1 nil 'silent)
  (let ((tmp2 (make-temp-name \"/tmp/test-link-\")))
    (make-symbolic-link tmp1 tmp2)
    (let ((result (list (and (file-symlink-p tmp2) t)
                        (file-exists-p tmp2)
                        (file-equal-p tmp1 tmp2))))
      (delete-file tmp2)
      (delete-file tmp1)
      result))) ",
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}
