//! Oracle parity tests for `substring`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{
    assert_err_kind, assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm, run_neovm_eval,
    run_oracle_eval,
};

#[test]
fn oracle_prop_substring_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello world" 0 5)"#);
    assert_ok_eq(r#""hello""#, &o, &n);

    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello world" 6)"#);
    assert_ok_eq(r#""world""#, &o, &n);
}

#[test]
fn oracle_prop_substring_full() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 0)"#);
    assert_ok_eq(r#""hello""#, &o, &n);
}

#[test]
fn oracle_prop_substring_empty_result() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 3 3)"#);
    assert_ok_eq(r#""""#, &o, &n);
}

#[test]
fn oracle_prop_substring_negative_index() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Negative indices count from end
    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello world" -5)"#);
    assert_ok_eq(r#""world""#, &o, &n);

    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" -3 -1)"#);
    assert_ok_eq(r#""ll""#, &o, &n);
}

#[test]
fn oracle_prop_substring_single_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm(r#"(substring "hello" 1 2)"#);
    assert_ok_eq(r#""e""#, &o, &n);
}

#[test]
fn oracle_prop_substring_out_of_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r####"(substring "hello" 0 100)"####;
    let oracle = run_oracle_eval(form).expect("oracle eval should run");
    let neovm = run_neovm_eval(form).expect("neovm eval should run");
    assert_err_kind(&oracle, &neovm, "args-out-of-range");
}

#[test]
fn oracle_prop_substring_with_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r####"(concat (substring "hello" 0 2) (substring "world" 3))"####;
    let (o, n) = eval_oracle_and_neovm(form);
    assert_ok_eq(r#""held""#, &o, &n);
}

#[test]
fn oracle_prop_substring_on_empty_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm(r#"(substring "" 0)"#);
    assert_ok_eq(r#""""#, &o, &n);
}

#[test]
fn oracle_substring_rejects_non_vector_arraylikes_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU src/fns.c:Fsubstring uses CHECK_VECTOR_OR_STRING: strings and
    // ordinary vectors are accepted, but records, byte-code objects,
    // bool-vectors, and char-tables are rejected with `arrayp`.
    // Fsubstring_no_properties starts with CHECK_STRING.
    let form = r#"
(list
 (substring [1 2 3] 0 2)
 (condition-case err
     (substring (record 'neovm--substring-record 1 2) 0 1)
   (error (list (car err) (cdr err))))
 (condition-case err
     (substring #[257 "\300\207" [42] 1] 0 1)
   (error (list (car err) (cdr err))))
 (condition-case err
     (substring (make-bool-vector 3 t) 0 1)
   (error (list (car err) (cdr err))))
 (condition-case err
     (substring (make-char-table 'generic 65) 0 1)
   (error (list (car err) (cdr err))))
 (condition-case err
     (substring-no-properties [1 2 3] 0 1)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity(form);
}
