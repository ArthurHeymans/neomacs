//! Oracle parity tests for GNU `truncate-string-to-width` semantics.
//!
//! GNU implements this in `lisp/international/mule-util.el` on top of
//! `char-width` and `string-width`.  These cases pin the observable behavior
//! around start columns, padding, explicit ellipses, and display-property
//! ellipsis mode.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_truncate_ascii_start_and_end_columns() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (truncate-string-to-width "abcdefghij" 5)
 (truncate-string-to-width "abcdefghij" 5 2)
 (truncate-string-to-width "abcdefghij" 0)
 (truncate-string-to-width "abcdefghij" 20 8))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_truncate_wide_chars_and_padding_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (concat "a" (char-to-string #x4e2d) "b"
                 (char-to-string #x6587) "c")))
  (list
   (string-width s)
   (truncate-string-to-width s 1)
   (truncate-string-to-width s 2)
   (truncate-string-to-width s 3)
   (truncate-string-to-width s 4)
   (truncate-string-to-width s 3 1 ?.)
   (truncate-string-to-width s 4 2 ?_)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_truncate_padding_when_string_is_too_short() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (truncate-string-to-width "" 4 nil ?.)
 (truncate-string-to-width "ab" 5 nil ?_)
 (truncate-string-to-width "ab" 5 10 ?x)
 (truncate-string-to-width "ab" 5 1 ?-))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_truncate_explicit_ellipsis_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (concat "abcd" (char-to-string #x4e2d) "efgh")))
  (list
   (truncate-string-to-width s 4 nil nil "...")
   (truncate-string-to-width s 5 nil nil "...")
   (truncate-string-to-width s 6 nil nil "<>")
   (truncate-string-to-width "abc" 2 nil nil "...")
   (truncate-string-to-width "abc" 2 nil nil "")))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_truncate_ellipsis_text_property_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (copy-sequence "abcdefghij")))
  (put-text-property 2 7 'face 'bold s)
  (let ((r (truncate-string-to-width s 5 nil nil "..." t)))
    (list r
          (substring-no-properties r)
          (text-properties-at 2 r)
          (text-properties-at 5 r)
          (get-text-property 5 'display r))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_truncate_argument_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((cases
       (list
        (lambda () (truncate-string-to-width "abc" "2"))
        (lambda () (truncate-string-to-width "abc" 2 "1"))
        (lambda () (truncate-string-to-width "abc" 2 nil "x"))
        (lambda () (truncate-string-to-width 123 2)))))
  (mapcar
   (lambda (fn)
     (condition-case err
         (funcall fn)
       (error (list (car err) (cadr err)))))
   cases))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
