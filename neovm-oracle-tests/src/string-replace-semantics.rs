//! Oracle parity tests for GNU `string-replace` semantics.
//!
//! GNU implements this helper in `lisp/subr.el`: empty FROM signals
//! `wrong-length-argument`, no-match returns the original string object, and
//! successful replacement rebuilds the string from substring pieces.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_string_replace_empty_from_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((cases
       (list
        (lambda () (string-replace "" "-" "abc"))
        (lambda () (string-replace "" "" "abc"))
        (lambda () (string-replace "" "x" "")))))
  (mapcar
   (lambda (fn)
     (condition-case err
         (funcall fn)
       (error (list (car err) (cdr err)))))
   cases))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_string_replace_no_match_returns_same_object() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (copy-sequence "alpha beta")))
  (put-text-property 0 5 'face 'bold s)
  (let ((r (string-replace "zzz" "qqq" s)))
    (list r
          s
          (eq r s)
          (text-properties-at 0 r)
          (text-properties-at 6 r))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_string_replace_success_property_shape() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (copy-sequence "alpha beta alpha")))
  (put-text-property 0 5 'face 'bold s)
  (put-text-property 6 10 'help-echo "mid" s)
  (put-text-property 11 16 'mouse-face 'highlight s)
  (let ((r (string-replace "alpha" "A" s)))
    (list r
          (eq r s)
          (substring-no-properties r)
          (text-properties-at 0 r)
          (text-properties-at 2 r)
          (text-properties-at 7 r))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_string_replace_non_overlapping_matches() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (string-replace "aa" "b" "aaaa")
 (string-replace "aa" "b" "aaaaa")
 (string-replace "ana" "X" "banana")
 (string-replace "aba" "X" "ababa")
 (string-replace "a" "aa" "aaa"))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_string_replace_type_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((cases
       (list
        (lambda () (string-replace 'a "b" "abc"))
        (lambda () (string-replace "a" 'b "abc"))
        (lambda () (string-replace "a" "b" 123)))))
  (mapcar
   (lambda (fn)
     (condition-case err
         (funcall fn)
       (error (list (car err) (cadr err)))))
   cases))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
