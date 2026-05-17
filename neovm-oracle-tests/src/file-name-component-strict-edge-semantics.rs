//! Oracle parity tests for GNU file name component splitting.
//!
//! GNU implements `file-name-directory` and `file-name-nondirectory` in
//! `src/fileio.c`.  These functions are syntactic and preserve repeated slash
//! structure in the returned component.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_name_directory_and_nondirectory_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (file-name-directory "")
 (file-name-nondirectory "")
 (file-name-directory "plain")
 (file-name-nondirectory "plain")
 (file-name-directory "/")
 (file-name-nondirectory "/")
 (file-name-directory "//")
 (file-name-nondirectory "//")
 (file-name-directory "///")
 (file-name-nondirectory "///")
 (file-name-directory "a/b")
 (file-name-nondirectory "a/b")
 (file-name-directory "a/b/")
 (file-name-nondirectory "a/b/")
 (file-name-directory "a//b")
 (file-name-nondirectory "a//b")
 (file-name-directory "/a//b")
 (file-name-nondirectory "/a//b")
 (file-name-directory "/a//b/")
 (file-name-nondirectory "/a//b/")
 (condition-case err
     (file-name-directory)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-nondirectory 42)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_file_name_extension_and_version_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (mapcar (lambda (name)
           (list name
                 (file-name-extension name)
                 (file-name-extension name t)
                 (file-name-sans-extension name)
                 (file-name-base name)
                 (file-name-sans-versions name)
                 (file-name-sans-versions name t)))
         '("plain"
           "plain."
           ".emacs"
           ".emacs.el"
           "archive.tar.gz"
           "/tmp/archive.tar.gz"
           "/tmp/.hidden"
           "/tmp/.hidden.el"
           "/tmp/dir.with.dots/file"
           "/tmp/dir.with.dots/file."
           "foo.~12~"
           "foo.el.~12~"
           "foo.el.~12~.~3~"
           "foo.~~"
           "foo.js.~HEAD~1~"))
 (condition-case err
     (file-name-extension 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-extension "x" nil nil)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-sans-extension 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-sans-versions 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-base)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
