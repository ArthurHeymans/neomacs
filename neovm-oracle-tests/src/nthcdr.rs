//! Oracle parity tests for `nthcdr`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use proptest::prelude::*;

use super::common::{ORACLE_PROP_CASES, assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_nthcdr_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nthcdr 2 '(a b c d e))",
        expect_test::expect![[r#""OK (c d e)""#]],
    );
    assert_ok_eq("(c d e)", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nthcdr 0 '(10 20 30))",
        expect_test::expect![[r#""OK (10 20 30)""#]],
    );
    assert_ok_eq("(10 20 30)", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nthcdr 5 '(1 2))",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nthcdr 0 nil)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(nthcdr 1 '(solo))",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_nthcdr_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(nthcdr 'x '(1 2))",
        expect_test::expect![[r#""ERR (wrong-type-argument integerp x)""#]],
    );
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_nthcdr_circular_and_improper_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((cycle (list 'a 'b 'c))
       (_ (setcdr (last cycle) cycle))
       (big (expt 10 40)))
  (list
   (eq (nthcdr 3 cycle) cycle)
   (car (nthcdr 4 cycle))
   (car (nthcdr big cycle))
   (condition-case err
       (nthcdr 2 '(a . b))
     (error (list (car err) (cdr err))))
   (condition-case err
       (nth 2 '(a . b))
     (error (list (car err) (cdr err))))))
"#;

    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK (t b b (wrong-type-argument (listp (a . b))) (wrong-type-argument (listp (a . b))))""#
        ]],
    );
}

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(ORACLE_PROP_CASES))]

    #[test]
    fn oracle_prop_nthcdr_random_offset(
        offset in 0i64..6i64,
        a in -500i64..500i64,
        b in -500i64..500i64,
        c in -500i64..500i64,
        d in -500i64..500i64,
    ) {
        return_if_neovm_enable_oracle_proptest_not_set!(Ok(()));

        let form = format!("(nthcdr {} (list {} {} {} {}))", offset, a, b, c, d);
        let (oracle, neovm) = eval_oracle_and_neovm(&form);
        assert_eq!(neovm, oracle, "nthcdr parity failed for: {form}");
    }
}
