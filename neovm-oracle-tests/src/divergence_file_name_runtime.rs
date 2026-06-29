//! File-name manipulation parity: directory/nondirectory/extension/base,
//! expand-file-name, split/concat/relative, file-attributes, quoting, and
//! temp-file write/read roundtrip.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn expand_file_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (expand-file-name "b" "/a") (expand-file-name "../c" "/a/b")
        (expand-file-name "./d" "/a") (expand-file-name "~/x" "/a")
        (file-name-absolute-p "/a") (file-name-absolute-p "a"))"##,
        expect_test::expect![[r#""OK (\"/a/b\" \"/a/c\" \"/a/d\" \"/home/exec/x\" t nil)""#]],
    );
}

#[test]
fn file_attrs_tempfile() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (make-temp-file "neo-fa-")))
  (unwind-protect
      (let ((a (file-attributes f)))
        (list (eq (car a) nil) (integerp (file-attribute-size a))
              (file-exists-p f) (file-regular-p f) (file-readable-p f)))
    (delete-file f)))"##,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn file_name_completion_quote() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (file-name-quote "/a/~b") (file-name-unquote "/:/a/~b")
        (file-name-quoted-p "/:/a") (convert-standard-filename "a/b"))"##,
        expect_test::expect![[r#""OK (\"/:/a/~b\" \"/a/~b\" t \"a/b\")""#]],
    );
}

#[test]
fn file_name_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (file-name-directory "/a/b/c.txt") (file-name-nondirectory "/a/b/c.txt")
        (file-name-extension "/a/b/c.txt") (file-name-sans-extension "/a/b/c.txt")
        (file-name-base "/a/b/c.txt") (file-name-as-directory "/a/b"))"##,
        expect_test::expect![[r#""OK (\"/a/b/\" \"c.txt\" \"txt\" \"/a/b/c\" \"c\" \"/a/b/\")""#]],
    );
}

#[test]
fn file_name_split_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (file-name-split "/a/b/c") (file-name-concat "a" "b" "c")
        (directory-file-name "/a/b/") (file-relative-name "/a/b/c" "/a"))"##,
        expect_test::expect![[r#""OK ((\"\" \"a\" \"b\" \"c\") \"a/b/c\" \"/a/b\" \"b/c\")""#]],
    );
}

#[test]
fn temp_file_write_read() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (make-temp-file "neo-rw-")))
  (unwind-protect
      (progn
        (with-temp-file f (insert "line1\nline2\n"))
        (with-temp-buffer (insert-file-contents f)
          (list (buffer-string) (count-lines (point-min) (point-max)))))
    (delete-file f)))"##,
        expect_test::expect![[r#""OK (\"line1\nline2\n\" 2)""#]],
    );
}
