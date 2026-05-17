//! Oracle parity tests for GNU directory/file name conversion helpers.
//!
//! GNU implements `file-name-as-directory`, `directory-file-name`, and
//! `directory-name-p` in `src/fileio.c`.  These are syntactic operations with
//! specific Unix root and empty-string behavior.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_directory_name_transform_root_and_empty_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (file-name-as-directory "")
 (file-name-as-directory ".")
 (file-name-as-directory "a")
 (file-name-as-directory "a/")
 (file-name-as-directory "/")
 (file-name-as-directory "//")
 (file-name-as-directory "///")
 (directory-file-name "")
 (directory-file-name ".")
 (directory-file-name "a")
 (directory-file-name "a/")
 (directory-file-name "a///")
 (directory-file-name "/")
 (directory-file-name "//")
 (directory-file-name "///")
 (directory-name-p "")
 (directory-name-p ".")
 (directory-name-p "a")
 (directory-name-p "a/")
 (directory-name-p "/")
 (directory-name-p "//")
 (condition-case err
     (file-name-as-directory)
   (error (list (car err) (cdr err))))
 (condition-case err
     (directory-file-name 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (directory-name-p 42)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_unhandled_file_name_directory_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (unhandled-file-name-directory "")
 (unhandled-file-name-directory ".")
 (unhandled-file-name-directory "plain")
 (unhandled-file-name-directory "plain/")
 (unhandled-file-name-directory "/")
 (unhandled-file-name-directory "//")
 (unhandled-file-name-directory "///")
 (unhandled-file-name-directory "/tmp/file")
 (let ((file-name-handler-alist nil))
   (unhandled-file-name-directory "/tmp/no-handler"))
 (condition-case err
     (unhandled-file-name-directory)
   (error (list (car err) (cdr err))))
 (condition-case err
     (unhandled-file-name-directory 42)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
