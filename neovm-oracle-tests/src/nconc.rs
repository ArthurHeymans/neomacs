//! Oracle parity tests for `nconc`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_nconc_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nconc '(1 2) '(3 4))",
        expect_test::expect![[r#""OK (1 2 3 4)""#]],
    );
    assert_ok_eq("(1 2 3 4)", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nconc '(a b) '(c) '(d e f))",
        expect_test::expect![[r#""OK (a b c d e f)""#]],
    );
    assert_ok_eq("(a b c d e f)", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nconc nil '(5 6))",
        expect_test::expect![[r#""OK (5 6)""#]],
    );
    assert_ok_eq("(5 6)", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nconc '(7 8) nil)",
        expect_test::expect![[r#""OK (7 8)""#]],
    );
    assert_ok_eq("(7 8)", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nconc nil)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nconc '(99))",
        expect_test::expect![[r#""OK (99)""#]],
    );
    assert_ok_eq("(99)", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nconc nil nil nil '(1))",
        expect_test::expect![[r#""OK (1)""#]],
    );
    assert_ok_eq("(1)", &o, &n);
}
