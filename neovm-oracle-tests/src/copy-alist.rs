//! Oracle parity tests for `copy-alist`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{
    assert_ok_eq, assert_oracle_parity_with_bootstrap, eval_oracle_and_neovm,
    eval_oracle_and_neovm_with_bootstrap,
};

#[test]
fn oracle_prop_copy_alist_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm_with_bootstrap("(copy-alist '((a . 1) (b . 2) (c . 3)))");
    assert_ok_eq("((a . 1) (b . 2) (c . 3))", &o, &n);

    let (o, n) = eval_oracle_and_neovm_with_bootstrap("(copy-alist nil)");
    assert_ok_eq("nil", &o, &n);

    let (o, n) = eval_oracle_and_neovm_with_bootstrap("(copy-alist '((x . 10)))");
    assert_ok_eq("((x . 10))", &o, &n);

    // verify it's a distinct copy (setcdr on copy doesn't affect original)
    let (o, n) = eval_oracle_and_neovm_with_bootstrap(
        "(let* ((orig '((k . 1))) (cp (copy-alist orig))) (setcdr (car cp) 99) (cdar orig))",
    );
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_copy_alist_improper_tail_error_payload() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU src/fns.c:Fcopy_alist first CHECK_LISTs the alist, so dotted alists
    // signal with the offending tail.  Non-cons alist elements are otherwise
    // legal and copied as shared elements.
    let form = r#"
(list
 (copy-alist '(loose (a . 1) atom (b . 2)))
 (condition-case err
     (copy-alist '(loose (a . 1) . tail))
   (error (list (car err) (cdr err))))
 (condition-case err
     (copy-alist 42)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
