/// Batch 500: milestone — trace, tracer, profiler, coverage, ert deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx500_trace_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'trace)
  (defun neo-cx500-fn (x) (* x 2))
  (trace-function 'neo-cx500-fn)
  (list (fboundp 'trace-function) (fboundp 'untrace-function)))
"##,
    );
}

#[test]
fn div_cx500_profiler() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'profiler)
  (list (fboundp 'profiler-start)
        (fboundp 'profiler-stop)
        (fboundp 'profiler-report)))
"##,
    );
}

#[test]
fn div_cx500_elp_profile() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'elp)
  (list (fboundp 'elp-instrument-function)
        (fboundp 'elp-results)))
"##,
    );
}

#[test]
fn div_cx500_cover_test() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'testcover)
  (list (fboundp 'testcover-start)
        (fboundp 'testcover-this-defun)))
"##,
    );
}

#[test]
fn div_cx500_ert_deftest_run() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ert)
  (ert-deftest neo-cx500-test ()
    (should (equal 1 1)))
  (ert-run-test 'neo-cx500-test))
"##,
    );
}

#[test]
fn div_cx500_ert_explainer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ert)
  (ert-deftest neo-cx500-e ()
    (should (= 2 2)))
  (list (fboundp 'ert-run-tests-batch)
        (fboundp 'ert-results-pop-to-buffer)))
"##,
    );
}

#[test]
fn div_cx500_check_declare() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cl-lib)
  (cl-declare (ftype (function (integer) integer) neo-cx500-fn))
  (fboundp 'neo-cx500-fn))
"##,
    );
}

#[test]
fn div_cx500_check_lib() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (featurep 'cl-lib) (featurep 'cl-seq) (featurep 'cl-macs))
"##,
    );
}

#[test]
fn div_cx500_check_features() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (featurep 'make-temp-file) (featurep 'advice) (featurep 'font-lock))
"##,
    );
}

#[test]
fn div_cx500_check_load_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((lp load-path)) (list (listp lp) (> (length lp) 0)))
"##,
    );
}

#[test]
fn div_cx500_check_exec_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ep exec-path)) (list (listp ep) (> (length ep) 0)))
"##,
    );
}

#[test]
fn div_cx500_check_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'process-environment)
      (boundp 'initial-environment)
      (listp process-environment))
"##,
    );
}

#[test]
fn div_cx500_check_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (stringp (documentation 'car))
      (stringp (documentation-property 'car 'function-documentation)))
"##,
    );
}

#[test]
fn div_cx500_check_keymaps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (keymapp (current-global-map))
      (keymapp (current-minor-mode-maps)))
"##,
    );
}

#[test]
fn div_cx500_check_macros() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (macroexpand '(when t (message "hi")))
      (macroexpand-all '(when t (message "hi"))))
"##,
    );
}
