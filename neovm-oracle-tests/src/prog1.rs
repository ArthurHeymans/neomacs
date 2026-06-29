//! Oracle parity tests for `prog1`.
//!
//! Note: `prog2` is a Lisp macro defined in `subr.el` (not a C primitive),
//! so it is not available in the bare `Context::new()` used by oracle tests.
//! It is tested via full neomacs which loads `subr.el`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_prog1_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(prog1 10 20 30)",
        expect_test::expect![[r#""OK 10""#]],
    );
    assert_ok_eq("10", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(prog1 'first)",
        expect_test::expect![[r#""OK first""#]],
    );
    assert_ok_eq("first", &o, &n);

    // side effects still happen
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(let ((x 0)) (prog1 x (setq x 99)) )",
        expect_test::expect![[r#""OK 0""#]],
    );
    assert_ok_eq("0", &o, &n);

    // prog1 with no body forms
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(prog1 42)",
        expect_test::expect![[r#""OK 42""#]],
    );
    assert_ok_eq("42", &o, &n);
}
