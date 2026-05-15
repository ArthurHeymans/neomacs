//! Oracle parity tests for GNU `clear-string` semantics.
//!
//! GNU implements `clear-string` in `src/fns.c`: it destructively removes text
//! properties, zeroes the existing bytes, makes the string unibyte, and sets
//! the character length to the previous byte length.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_clear_string_zeroes_bytes_removes_properties_and_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((s (propertize (copy-sequence "éx") 'face 'bold))
       (before (list s (length s) (string-bytes s)
                     (multibyte-string-p s)
                     (text-properties-at 0 s)))
       (ret (clear-string s)))
  (list ret
        before
        s
        (length s)
        (string-bytes s)
        (multibyte-string-p s)
        (text-properties-at 0 s)
        (mapcar (lambda (i) (aref s i))
                (number-sequence 0 (1- (length s))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_clear_string_empty_and_wrong_type_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (let ((s "")) (list (clear-string s) s (length s) (string-bytes s)))
 (let ((s (string ?é)))
   (clear-string s)
   (list s (length s) (string-bytes s) (multibyte-string-p s)))
 (condition-case err
     (clear-string [1 2 3])
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
