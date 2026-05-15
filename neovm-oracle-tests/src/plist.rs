//! Oracle parity tests for `plist-get`, `plist-put`, `plist-member`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, assert_oracle_parity_with_bootstrap, eval_oracle_and_neovm};

#[test]
fn oracle_prop_plist_get_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(plist-get '(:a 1 :b 2 :c 3) :b)");
    assert_ok_eq("2", &o, &n);

    let (o, n) = eval_oracle_and_neovm("(plist-get '(:a 1 :b 2 :c 3) :a)");
    assert_ok_eq("1", &o, &n);

    let (o, n) = eval_oracle_and_neovm("(plist-get '(:a 1 :b 2 :c 3) :c)");
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_prop_plist_get_missing_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(plist-get '(:a 1 :b 2) :z)");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_plist_get_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(plist-get nil :a)");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_plist_put_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(let ((pl '(:a 1 :b 2)))
                  (plist-get (plist-put pl :c 3) :c))";
    let (o, n) = eval_oracle_and_neovm(form);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_prop_plist_put_overwrite() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(let ((pl '(:a 1 :b 2)))
                  (plist-get (plist-put pl :a 99) :a))";
    let (o, n) = eval_oracle_and_neovm(form);
    assert_ok_eq("99", &o, &n);
}

#[test]
fn oracle_prop_plist_member_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // plist-member returns the tail starting from the matching key
    assert_oracle_parity_with_bootstrap("(plist-member '(:a 1 :b 2 :c 3) :b)");
}

#[test]
fn oracle_prop_plist_member_missing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(plist-member '(:a 1 :b 2) :z)");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_plist_chained_puts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(let* ((pl nil)
                       (pl (plist-put pl :x 10))
                       (pl (plist-put pl :y 20))
                       (pl (plist-put pl :z 30)))
                  (list (plist-get pl :x)
                        (plist-get pl :y)
                        (plist-get pl :z)))";
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_plist_with_non_keyword_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // plist-get/plist-put work with any eq-comparable keys
    let (o, n) = eval_oracle_and_neovm("(plist-get '(a 1 b 2 c 3) 'b)");
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_prop_plist_complex_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(plist-get '(:data (1 2 3) :name \"test\" :flag t) :data)";
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_plist_optional_predicate_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((key-a (copy-sequence "key"))
       (key-b (copy-sequence "key"))
       (plist (list key-a 1 :other 2)))
  (list
   (plist-get plist key-b)
   (plist-get plist key-b 'equal)
   (plist-member plist key-b)
   (plist-member plist key-b 'equal)
   (let ((copy (copy-sequence plist)))
     (list (eq (plist-put copy key-b 9 'equal) copy)
           (plist-get copy key-a)
           copy))
   (let ((copy (copy-sequence plist)))
     (plist-put copy key-b 9)
     copy)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_plist_malformed_tail_error_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (plist-get '(:a 1 . bogus) :z)
 (plist-get '(:a) :z)
 (plist-get '(:a . bogus) :z)
 (condition-case err
     (plist-member '(:a 1 . bogus) :z)
   (error (list (car err) (cdr err))))
 (condition-case err
     (plist-member '(:a) :z)
   (error (list (car err) (cdr err))))
 (condition-case err
     (plist-member '(:a . bogus) :z)
   (error (list (car err) (cdr err))))
 (condition-case err
     (plist-put '(:a 1 . bogus) :z 3)
   (error (list (car err) (cdr err))))
 (condition-case err
     (plist-put '(:a) :z 3)
   (error (list (car err) (cdr err))))
 (condition-case err
     (plist-put '(:a . bogus) :z 3)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
