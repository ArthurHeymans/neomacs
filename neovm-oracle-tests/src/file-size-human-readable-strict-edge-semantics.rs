//! Oracle parity tests for GNU `file-size-human-readable` semantics.
//!
//! GNU implements this in `lisp/files.el`.  The behavior is pure Elisp:
//! nil/`iec` use a 1024 divisor, any other non-nil flavor uses 1000, and
//! formatting follows GNU `ls -lh` one-decimal rounding rules.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_size_human_readable_flavors_rounding_units_and_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (file-size-human-readable 0)
 (file-size-human-readable 1)
 (file-size-human-readable 999)
 (file-size-human-readable 1000)
 (file-size-human-readable 1024)
 (file-size-human-readable 1536)
 (file-size-human-readable 1048576)
 (file-size-human-readable 1536 'iec)
 (file-size-human-readable 1536 'si)
 ;; Any non-nil, non-iec flavor follows the SI divisor branch.
 (file-size-human-readable 1536 'gnu)
 (file-size-human-readable 1536 nil "" "B")
 (file-size-human-readable 1536 'iec " ")
 (file-size-human-readable-iec 1536)
 ;; Rounding boundaries from GNU's one-decimal rule.
 (file-size-human-readable 1075)
 (file-size-human-readable 1996)
 (file-size-human-readable 2000 'si)
 (file-size-human-readable -1)
 (file-size-human-readable 0 'iec " " nil)
 (file-size-human-readable 0 nil 42 "B")
 (condition-case err
     (file-size-human-readable "1536")
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-size-human-readable 1 nil nil 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-size-human-readable-iec "1536")
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
