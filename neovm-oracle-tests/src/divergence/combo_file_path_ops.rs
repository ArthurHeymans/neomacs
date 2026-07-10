//! Divergence tests: file-name + directory + path + expand combos.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_file_name_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""OK (\"/home/user/\" t \"file.txt\" t \"test\" t \"test\" t \"el\" t nil t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (list (file-name-directory "/home/user/file.txt")
        (string= (file-name-directory "/home/user/file.txt") "/home/user/")
        (file-name-nondirectory "/home/user/file.txt")
        (string= (file-name-nondirectory "/home/user/file.txt") "file.txt")
        (file-name-sans-extension "test.el")
        (string= (file-name-sans-extension "test.el") "test")
        (file-name-sans-extension "test.elc")
        (string= (file-name-sans-extension "test.elc") "test")
        (file-name-extension "test.el")
        (string= (file-name-extension "test.el") "el")
        (file-name-extension "test")
        (null (file-name-extension "test")))) "#,
        expect,
    );
}

#[test]
fn divergence_expand_file_name_relative() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (t t t t t t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (let ((abs (expand-file-name "foo/bar.txt" "/home/user"))
        (abs2 (expand-file-name "../bar.txt" "/home/user/docs"))
        (abs3 (expand-file-name "./test.el" "/tmp")))
    (list (string= abs "/home/user/foo/bar.txt")
          (string= abs2 "/home/user/bar.txt")
          (string= abs3 "/tmp/test.el")
          (> (length abs) 0)
          (file-name-absolute-p abs)
          (file-name-absolute-p abs2)
          (not (file-name-absolute-p "relative/path"))))) "#,
        expect,
    );
}

#[test]
fn divergence_directory_file_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""OK (\"/home/user\" t \"/home/user\" t \"/home/user/\" t \"/home/user/\" t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (list (directory-file-name "/home/user/")
        (string= (directory-file-name "/home/user/") "/home/user")
        (directory-file-name "/home/user")
        (string= (directory-file-name "/home/user") "/home/user")
        (file-name-as-directory "/home/user")
        (string= (file-name-as-directory "/home/user") "/home/user/")
        (file-name-as-directory "/home/user/")
        (string= (file-name-as-directory "/home/user/") "/home/user/"))) "#,
        expect,
    );
}

#[test]
fn divergence_file_name_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""OK (\"/a/b/file.txt\" t \"/a/b/file.txt\" t \"/tmp/sub/file.txt\" t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (list (concat (file-name-directory "/a/b/") "file.txt")
        (string= (concat (file-name-directory "/a/b/") "file.txt") "/a/b/file.txt")
        (expand-file-name "file.txt" "/a/b/")
        (string= (expand-file-name "file.txt" "/a/b/") "/a/b/file.txt")
        (file-name-concat "/tmp" "sub" "file.txt")
        (string= (file-name-concat "/tmp" "sub" "file.txt") "/tmp/sub/file.txt"))) "#,
        expect,
    );
}

#[test]
fn divergence_path_split_components() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""OK ((\"home\" \"user\" \"docs\" \"file.txt\") t t \"file.txt\" t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (let ((split (split-string "/home/user/docs/file.txt" "/" t)))
    (list split
          (equal split '("home" "user" "docs" "file.txt"))
          (= (length split) 4)
          (car (last split))
          (string= (car (last split)) "file.txt")))) "#,
        expect,
    );
}

#[test]
fn divergence_file_truename_tilde() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (t t 0 t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (let ((expanded (expand-file-name "~")))
    (list (file-name-absolute-p expanded)
          (> (length expanded) 1)
          (string-match "^/" expanded)
          (= (string-match "^/" expanded) 0)
          (not (string= expanded "~"))))) "#,
        expect,
    );
}

#[test]
fn divergence_file_attributes_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (t t 219 t 27540 t)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (let ((attrs (file-attributes "/tmp")))
    (list (car attrs)
          (eq (car attrs) t)
          (nth 1 attrs)
          (integerp (nth 1 attrs))
          (nth 7 attrs)
          (integerp (nth 7 attrs))))) "#,
        expect,
    );
}

#[test]
fn divergence_make_temp_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (t t t 22 t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (let ((tmp (make-temp-file "test-div-")))
    (unwind-protect
        (list (file-exists-p tmp)
              (file-regular-p tmp)
              (> (length tmp) 0)
              (string-match "test-div-" tmp)
              (file-writable-p tmp)
              (= (nth 7 (file-attributes tmp)) 0))
      (delete-file tmp)))) "#,
        expect,
    );
}

#[test]
fn divergence_make_directory() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (t t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (let ((dir (make-temp-name "/tmp/test-dir-div-")))
    (unwind-protect
        (progn
          (make-directory dir)
          (list (file-directory-p dir)
                (file-exists-p dir)
                (> (length dir) 0)))
      (delete-directory dir)))) "#,
        expect,
    );
}

#[test]
fn divergence_file_size_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (t t t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (let ((src (make-temp-file "test-src-"))
        (content "Hello, World!"))
    (unwind-protect
        (progn
          (write-region content nil src nil 'silent)
          (let ((dst (make-temp-file "test-dst-")))
            (unwind-protect
                (progn
                  (copy-file src dst t)
                  (list (file-exists-p src)
                        (file-exists-p dst)
                        (= (nth 7 (file-attributes src))
                           (nth 7 (file-attributes dst)))
                        (> (nth 7 (file-attributes src)) 0)))
              (when (file-exists-p dst) (delete-file dst)))))
      (delete-file src)))) "#,
        expect,
    );
}
