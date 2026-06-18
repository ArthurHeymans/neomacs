/// Batch 456: signal/error/warning/debug deep probes.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx456_signal_complex_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (signal 'error '(("complex" data (1 2 3) . "tail")))
  (error (list (car e) (nth 1 e))))"##,
    );
}

#[test]
fn div_cx456_error_message_string_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (condition-case e (car 1 2) (error (error-message-string e)))
      (condition-case e (+ "a" 1) (error (error-message-string e)))
      (condition-case e (signal 'void-variable '(x)) (error (error-message-string e)))
      (condition-case e (aref [1] 5) (error (error-message-string e))))"##,
    );
}

#[test]
fn div_cx456_define_error_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((p (make-symbol "neo-cx456-p"))
      (c (make-symbol "neo-cx456-c")))
  (define-error p "parent error" 'error)
  (define-error c "child error" p)
  (list (get p 'error-conditions)
        (get c 'error-conditions)
        (condition-case e (signal c '(test))
          (p (cons 'parent (cadr e)))
          (error (cons 'error (cadr e))))))"##,
    );
}

#[test]
fn div_cx456_warning_suppress() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'warnings)
  (let ((warning-suppress-types '((neo-cx456))))
    (display-warning '(neo-cx456) "should be suppressed" :warning)
    (list (warning-suppress-p '(neo-cx456))
          (warning-numeric-level :warning))))"##,
    );
}

#[test]
fn div_cx456_condition_case_success_failure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (condition-case :success val (progn (+ 1 2) "ok") (:success val))
      (condition-case :success val (error "fail") (:success val)
        ((error) (list :err val)))
      (condition-case :failure val (error "fail") (:failure val)))"##,
    );
}

#[test]
fn div_cx456_backtrace_full() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((bt (condition-case e (let ((f (lambda () (error "test")))) (funcall f)) (error (backtrace-frames)))))
  (list (listp bt) (> (length bt) 1)))"##,
    );
}

#[test]
fn div_cx456_debug_on_signal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (condition-case e (debug-on-signal 'error) (error (car e)))
      (condition-case e (cancel-debug-on-signal) (error (car e))))"##,
    );
}

#[test]
fn div_cx456_with_temp_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-message "temp msg"
  (current-message))"##,
    );
}

#[test]
fn div_cx456_inhibit_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((inhibit-message t))
  (message "inhibited")
  (current-message))"##,
    );
}

#[test]
fn div_cx456_define_widget_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wid-edit)
  (define-widget 'neo-cx456-widget 'editable-field "Custom Widget")
  (widget-type (widget-create 'neo-cx456-widget "value")))"##,
    );
}

#[test]
fn div_cx456_custom_type_validate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (defcustom neo-cx456-opt "default" "test" :type 'string :options '("a" "b"))
  (custom-variable-p 'neo-cx456-opt))"##,
    );
}

#[test]
fn div_cx456_error_conditions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (condition-case e (car 1 2) (error (get (car e) 'error-conditions)))
      (condition-case e (+ "a" 1) (error (get (car e) 'error-conditions)))
      (condition-case e (aref [1] 5) (error (get (car e) 'error-conditions))))"##,
    );
}

#[test]
fn div_cx456_debugger_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (fboundp 'debug)
      (fboundp 'backtrace)
      (fboundp 'edebug-defun))"##,
    );
}

#[test]
fn div_cx456_top_level_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (fboundp 'top-level)
      (fboundp 'exit-recursive-edit)
      (fboundp 'abort-recursive-edit))"##,
    );
}

#[test]
fn div_cx456_condition_case_unless_debug() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case-unless-debug e
    (+ "a" 1)
  (error (car e)))"##,
    );
}
