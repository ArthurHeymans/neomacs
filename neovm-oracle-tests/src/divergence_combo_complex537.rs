/// Batch 537: cl-loop complex, cl-iterate, do-all-symbols, do-symbols.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx537_cl_loop_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i below 10 collect (* i i))
"##,
        expect_test::expect![[r#""OK (0 1 4 9 16 25 36 49 64 81)""#]],
    );
}

#[test]
fn div_cx537_cl_loop_with() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop with a = 1 and b = 2
           return (+ a b))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx537_cl_loop_append() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i in '((1) (2 3) (4 5 6)) append i)
"##,
        expect_test::expect![[r#""OK (1 2 3 4 5 6)""#]],
    );
}

#[test]
fn div_cx537_cl_loop_nconc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i in '((a) (b c) (d e f)) nconc i)
"##,
        expect_test::expect![[r#""OK (a b c d e f)""#]],
    );
}

#[test]
fn div_cx537_cl_loop_minimize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i in '(7 2 9 1 5)
           minimize i)
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx537_cl_loop_maximize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i in '(7 2 9 1 5)
           maximize i)
"##,
        expect_test::expect![[r#""OK 9""#]],
    );
}

#[test]
fn div_cx537_cl_loop_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i from 1 to 100 sum i)
"##,
        expect_test::expect![[r#""OK 5050""#]],
    );
}

#[test]
fn div_cx537_do_all_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((count 0))
  (do-all-symbols (s) (setq count (1+ count)))
  count)
"##,
        expect_test::expect![[r#""ERR (void-function do-all-symbols)""#]],
    );
}

#[test]
fn div_cx537_do_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (obarray-make 10)))
  (intern "hello" obs)
  (let ((found nil))
    (do-symbols (s obs) (setq found t))
    found))
"##,
        expect_test::expect![[r#""ERR (void-function do-symbols)""#]],
    );
}

#[test]
fn div_cx537_do_symbols_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (obarray-make 10)))
  (intern "a" obs) (intern "b" obs) (intern "c" obs)
  (let ((count 0))
    (do-symbols (s obs) (setq count (1+ count)))
    count))
"##,
        expect_test::expect![[r#""ERR (void-function do-symbols)""#]],
    );
}

#[test]
fn div_cx537_cl_loop_do() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i from 1 to 5
           do (print i)
           finally return 'done)
"##,
        expect_test::expect![[r#""\n1\n\n2\n\n3\n\n4\n\n5\nOK done""#]],
    );
}

#[test]
fn div_cx537_cl_loop_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop repeat 3 collect 'x)
"##,
        expect_test::expect![[r#""OK (x x x)""#]],
    );
}

#[test]
fn div_cx537_cl_loop_always() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i in '(2 4 6)
           always (evenp i))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx537_cl_loop_never() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i in '(2 4 5)
           never (oddp i))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx537_cl_loop_thereis_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-loop for i in '(1 2 3 4 5)
           thereis (when (> i 3) i))
"##,
        expect_test::expect![[r#""OK 4""#]],
    );
}
