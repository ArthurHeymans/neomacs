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
