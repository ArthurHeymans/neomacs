//! Oracle parity tests for GNU `file-name-concat` semantics.
//!
//! GNU implements this in `src/fileio.c`: nil and empty components are
//! skipped, slashes are inserted only between non-final non-empty components,
//! and absolute-looking later components are concatenated syntactically rather
//! than normalized.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_name_concat_filters_and_separator_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (file-name-concat "")
 (file-name-concat nil)
 (file-name-concat "" nil "")
 (file-name-concat "a")
 (file-name-concat "a" "b")
 (file-name-concat "a/" "b")
 (file-name-concat "a" "b/")
 (file-name-concat "a/" "b/")
 (file-name-concat "" "a" nil "" "b" "")
 (file-name-concat "/tmp" "a" "b")
 (file-name-concat "/tmp/" "/absolute" "tail")
 (file-name-concat "a" "/b" "c")
 (file-name-concat "a" "." ".." "b")
 (file-name-concat "a" nil "b" nil "c")
 (condition-case err
     (file-name-concat)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-concat "a" 42)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
