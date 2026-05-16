//! Oracle parity tests for GNU plist edge semantics.
//!
//! GNU implements these primitives in `src/fns.c`.  `plist-get` is documented
//! not to signal for invalid plists, while `plist-member` and `plist-put`
//! validate malformed tails with `plistp`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_plist_get_tolerates_malformed_plists() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (plist-get '(a 1 b . bad-tail) 'b)
 (plist-get '(a 1 b . bad-tail) 'missing)
 (plist-get '(a 1 b) 'b)
 (plist-get 'not-a-list 'anything)
 (let ((x (list 'a 1 'b 2)))
   (setcdr (cdr (cdr (cdr x))) x)
   (list (plist-get x 'b)
         (plist-get x 'missing))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_plist_member_and_put_validate_malformed_tails() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (condition-case err
     (plist-member '(a 1 b . bad-tail) 'missing)
   (error (list (car err) (cdr err))))
 (condition-case err
     (plist-member '(a 1 b) 'missing)
   (error (list (car err) (cdr err))))
 (condition-case err
     (plist-put '(a 1 b . bad-tail) 'c 3)
   (error (list (car err) (cdr err))))
 (condition-case err
     (plist-put 'not-a-list 'c 3)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_plist_member_matches_key_before_tail_validation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU src/fns.c:plist_member checks the property key before validating
    // the tail shape.  A malformed tail that starts with the searched key is
    // returned verbatim; a missing key on the same malformed plist still
    // signals `wrong-type-argument plistp'.
    let form = r#"
(list
 (plist-get '(a 1 b) 'b)
 (condition-case err
     (plist-member '(a 1 b) 'b)
   (error (cons (car err) (cdr err))))
 (condition-case err
     (plist-put '(a 1 b) 'b 2)
   (error (cons (car err) (cdr err))))
 (plist-get '(a 1 b . bad-tail) 'b)
 (condition-case err
     (plist-member '(a 1 b . bad-tail) 'b)
   (error (cons (car err) (cdr err))))
 (condition-case err
     (plist-member '(a 1 b . bad-tail) 'missing)
   (error (cons (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_plist_put_preserves_tail_and_mutates_existing_pair() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((orig (list 'a 1 'b 2))
       (same (plist-put orig 'b 22))
       (extended (plist-put orig 'c 3)))
  (list
   (eq orig same)
   (eq orig extended)
   orig
   extended
   (plist-member extended 'c)
   (plist-member extended 'b)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_plist_predicate_argument_uses_call_result() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((plist (list (copy-sequence "name") 1 "NAME" 2)))
  (list
   (plist-get plist "name")
   (plist-get plist "name" #'equal)
   (plist-get plist "name" #'string-equal)
   (plist-member plist "name" #'equal)
   (plist-put plist "name" 9 #'equal)
   plist))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
