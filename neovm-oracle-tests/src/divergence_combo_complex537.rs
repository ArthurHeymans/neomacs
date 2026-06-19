/// Batch 537: cl-loop complex, cl-iterate, do-all-symbols, do-symbols.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx537_cl_loop_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i below 10 collect (* i i))
"##,
    );
}

#[test]
fn div_cx537_cl_loop_with() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop with a = 1 and b = 2
           return (+ a b))
"##,
    );
}

#[test]
fn div_cx537_cl_loop_append() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '((1) (2 3) (4 5 6)) append i)
"##,
    );
}

#[test]
fn div_cx537_cl_loop_nconc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '((a) (b c) (d e f)) nconc i)
"##,
    );
}

#[test]
fn div_cx537_cl_loop_minimize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(7 2 9 1 5)
           minimize i)
"##,
    );
}

#[test]
fn div_cx537_cl_loop_maximize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(7 2 9 1 5)
           maximize i)
"##,
    );
}

#[test]
fn div_cx537_cl_loop_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i from 1 to 100 sum i)
"##,
    );
}

#[test]
fn div_cx537_do_all_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((count 0))
  (do-all-symbols (s) (setq count (1+ count)))
  count)
"##,
    );
}

#[test]
fn div_cx537_do_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((obs (obarray-make 10)))
  (intern "hello" obs)
  (let ((found nil))
    (do-symbols (s obs) (setq found t))
    found))
"##,
    );
}

#[test]
fn div_cx537_do_symbols_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((obs (obarray-make 10)))
  (intern "a" obs) (intern "b" obs) (intern "c" obs)
  (let ((count 0))
    (do-symbols (s obs) (setq count (1+ count)))
    count))
"##,
    );
}

#[test]
fn div_cx537_cl_loop_do() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i from 1 to 5
           do (print i)
           finally return 'done)
"##,
    );
}

#[test]
fn div_cx537_cl_loop_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop repeat 3 collect 'x)
"##,
    );
}

#[test]
fn div_cx537_cl_loop_always() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(2 4 6)
           always (evenp i))
"##,
    );
}

#[test]
fn div_cx537_cl_loop_never() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(2 4 5)
           never (oddp i))
"##,
    );
}

#[test]
fn div_cx537_cl_loop_thereis_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(1 2 3 4 5)
           thereis (when (> i 3) i))
"##,
    );
}
