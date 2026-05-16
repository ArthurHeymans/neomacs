//! Oracle parity tests for `member`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use proptest::prelude::*;

use super::common::{
    ORACLE_PROP_CASES, assert_err_kind, assert_ok_eq, assert_oracle_parity_with_bootstrap,
    eval_oracle_and_neovm,
};

#[test]
fn oracle_prop_member_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle_found, neovm_found) = eval_oracle_and_neovm("(member 2 '(1 2 3))");
    assert_ok_eq("(2 3)", &oracle_found, &neovm_found);

    let (oracle_missing, neovm_missing) = eval_oracle_and_neovm("(member 9 '(1 2 3))");
    assert_ok_eq("nil", &oracle_missing, &neovm_missing);
}

#[test]
fn oracle_prop_member_wrong_type_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(member 1 2)");
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_member_ignore_case_ignores_non_strings_and_returns_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU lisp/subr.el:member-ignore-case is a Lisp loop over LIST.  It skips
    // non-string elements, returns the original list tail at the first
    // string-equal-ignore-case match, and only signals if cdr traversal reaches
    // a malformed tail before any match.
    let form = r#"
(list
 (member-ignore-case "foo" '(1 "bar" foo "FoO" "later"))
 (member-ignore-case "foo" '(1 foo "FoO" . bad-tail))
 (condition-case err
     (member-ignore-case "missing" '(1 "bar" foo . bad-tail))
   (error (list (car err) (cdr err))))
 (condition-case err
     (member-ignore-case 'foo '("foo"))
   (error (list (car err) (cdr err)))))
"#;
    assert_oracle_parity_with_bootstrap(form);
}

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(ORACLE_PROP_CASES))]

    #[test]
    fn oracle_prop_member_returns_tail(
        a in -100_000i64..100_000i64,
        b in -100_000i64..100_000i64,
        c in -100_000i64..100_000i64,
    ) {
        return_if_neovm_enable_oracle_proptest_not_set!(Ok(()));
        prop_assume!(a != b);

        let form = format!("(member {} (list {} {} {}))", a, b, a, c);
        let expected = format!("({} {})", a, c);
        let (oracle, neovm) = eval_oracle_and_neovm(&form);
        assert_ok_eq(expected.as_str(), &oracle, &neovm);
    }
}
