//! Oracle parity tests for `cond`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_cond_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // first clause matches
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(cond (t 'yes))",
        expect_test::expect![[r#""OK yes""#]],
    );
    assert_ok_eq("yes", &o, &n);

    // second clause matches
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(cond (nil 'no) (t 'yes))",
        expect_test::expect![[r#""OK yes""#]],
    );
    assert_ok_eq("yes", &o, &n);

    // no match returns nil
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(cond (nil 'a) (nil 'b))",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    // empty cond
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(cond)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    // clause with multiple body forms
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(cond (t 1 2 3))",
        expect_test::expect![[r#""OK 3""#]],
    );
    assert_ok_eq("3", &o, &n);

    // test value returned when no body
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(cond (42))",
        expect_test::expect![[r#""OK 42""#]],
    );
    assert_ok_eq("42", &o, &n);

    // numeric test
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(let ((x 3)) (cond ((= x 1) 'one) ((= x 2) 'two) ((= x 3) 'three) (t 'other)))",
        expect_test::expect![[r#""OK three""#]],
    );
    assert_ok_eq("three", &o, &n);

    // side effects only in matching clause
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(let ((v 0)) (cond (nil (setq v 1)) (t (setq v 2))) v)",
        expect_test::expect![[r#""OK 2""#]],
    );
    assert_ok_eq("2", &o, &n);
}
