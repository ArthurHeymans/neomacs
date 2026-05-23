//! Oracle parity tests for GNU `subr.el` docstring line formatting helpers.
//!
//! GNU uses `internal--format-docstring-line` while bootstrapping Lisp
//! docstrings.  Its observable contract combines `format`, a newline
//! rejection path, and recursive single-line filling controlled by
//! `fill-column`.

use super::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn oracle_prop_gnu_internal_format_docstring_line_fill_column_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((fill-column 12))
  (list
   (internal--format-docstring-line "short")
   (internal--format-docstring-line "hello %s" "world")
   (internal--format-docstring-line "alpha beta gamma")
   (internal--format-docstring-line "alpha  beta")
   (internal--format-docstring-line "abcdefghijklm")
   (let ((fill-column 80))
     (internal--format-docstring-line "alpha beta gamma"))))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_gnu_internal_format_docstring_line_rejects_newlines_after_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(mapcar
 (lambda (thunk)
   (condition-case err
       (funcall thunk)
     (error (list (car err) (cadr err)))))
 (list
  (lambda () (internal--format-docstring-line "line1\nline2"))
  (lambda () (internal--format-docstring-line "line%s" "\nbreak"))
  (lambda () (internal--format-docstring-line "%s" "ok"))))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_gnu_internal_fill_string_single_line_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((fill-column 10))
  (list
   (internal--fill-string-single-line "")
   (internal--fill-string-single-line "short")
   (internal--fill-string-single-line "one two three")
   (internal--fill-string-single-line "one  two")
   (internal--fill-string-single-line " leading space")
   (internal--fill-string-single-line "trailing ")
   (let ((fill-column 5))
     (internal--fill-string-single-line "ab cd ef"))))
"#;

    assert_oracle_parity(form);
}
