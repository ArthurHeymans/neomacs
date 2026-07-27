use super::assert_assess_call_parity;
use expect_test::{Expect, expect};

#[test]
fn call_capture_library_registers_its_complete_callable_surface() {
    let elisp_form = r##"
(list
 (featurep 'assess-call)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (help-function-arglist
      symbol t)
     (file-name-nondirectory
      (or
       (symbol-file
        symbol 'defun)
       ""))))
  '(assess-call--capture-lambda
    assess-call-capture
    assess-call--hook-capture-lambda
    assess-call-capture-hook)))
"##;
    let expect: Expect = expect![[
        r#"OK (t ((assess-call--capture-lambda nil "assess-call.el") (assess-call-capture (sym-fn fn) "assess-call.el") (assess-call--hook-capture-lambda nil "assess-call.el") (assess-call-capture-hook (hook-var fn &optional append local) "assess-call.el")))"#
    ]];
    assert_assess_call_parity(elisp_form, expect);
}

#[test]
fn capture_lambda_records_arguments_returns_and_reverse_chronological_order() {
    let elisp_form = r##"
(let ((capture
       (assess-call--capture-lambda)))
  (list
   (funcall capture #'+ 1 2 3)
   (funcall capture #'concat "a" "b")
   (funcall capture #'list :x '(1 2))
   (funcall capture :return)
   (funcall capture :return)))
"##;
    let expect: Expect = expect![[
        r#"OK (6 "ab" #2=(:x #1=(1 2)) #3=(((:x #1#) . #2#) (("a" "b") . "ab") ((1 2 3) . 6)) #3#)"#
    ]];
    assert_assess_call_parity(elisp_form, expect);
}

#[test]
fn call_capture_observes_nested_and_repeated_real_calls_without_changing_return_values() {
    let elisp_form = r##"
(progn
  (defun assess-test-multiply (left right)
    (* left right))
  (defun assess-test-workflow ()
    (list
     (assess-test-multiply 2 3)
     (1+
      (assess-test-multiply
       4 5))))
  (let (workflow-result)
    (list
     (assess-call-capture
      'assess-test-multiply
      (lambda ()
        (setq workflow-result
              (assess-test-workflow))))
     workflow-result
     (advice-member-p
      nil
      'assess-test-multiply))))
"##;
    let expect: Expect = expect!["OK ((((4 5) . 20) ((2 3) . 6)) (6 21) nil)"];
    assert_assess_call_parity(elisp_form, expect);
}

#[test]
fn call_capture_removes_advice_after_nonlocal_exit_and_propagates_original_signal() {
    let elisp_form = r##"
(progn
  (defun assess-test-divide (left right)
    (/ left right))
  (let ((before
         (symbol-function
          'assess-test-divide))
        condition)
    (setq condition
          (condition-case data
              (assess-call-capture
               'assess-test-divide
               (lambda ()
                 (assess-test-divide 8 2)
                 (signal
                  'error
                  '("fixture failure"))))
            (error data)))
    (list
     condition
     (eq before
         (symbol-function
          'assess-test-divide))
     (assess-test-divide 9 3))))
"##;
    let expect: Expect = expect![[r#"OK ((error "fixture failure") t 3)"#]];
    assert_assess_call_parity(elisp_form, expect);
}

#[test]
fn hook_capture_lambda_records_zero_one_and_many_argument_invocations() {
    let elisp_form = r##"
(let ((capture
       (assess-call--hook-capture-lambda)))
  (funcall capture)
  (funcall capture 'alpha)
  (funcall capture 'beta 2 "three")
  (list
   (funcall capture :return)
   (funcall capture :return)))
"##;
    let expect: Expect = expect![[r#"OK (#1=((beta 2 "three") (alpha) nil) #1#)"#]];
    assert_assess_call_parity(elisp_form, expect);
}

#[test]
fn hook_capture_handles_global_append_order_arguments_and_cleanup() {
    let elisp_form = r##"
(let ((assess-test-hook nil)
      workflow)
  (add-hook
   'assess-test-hook
   (lambda (&rest args)
     (push
      (cons :existing args)
      workflow)))
  (let ((captured
         (assess-call-capture-hook
          'assess-test-hook
          (lambda ()
            (run-hooks
             'assess-test-hook)
            (run-hook-with-args
             'assess-test-hook
             'alpha 2))
          t)))
    (list
     captured
     (nreverse workflow)
     (length assess-test-hook)
     (mapcar
      (lambda (function)
        (byte-code-function-p
         function))
      assess-test-hook))))
"##;
    let expect: Expect = expect!["OK (((alpha 2) nil) ((:existing) (:existing alpha 2)) 0 nil)"];
    assert_assess_call_parity(elisp_form, expect);
}

#[test]
fn hook_capture_uses_buffer_local_hooks_and_cleans_them_after_signals() {
    let elisp_form = r##"
(let ((assess-test-local-hook
       '(global-sentinel))
      escaped-hook
      condition)
  (with-temp-buffer
    (setq-local
     assess-test-local-hook
     '(local-sentinel))
    (setq condition
          (condition-case data
              (assess-call-capture-hook
               'assess-test-local-hook
               (lambda ()
                 (setq escaped-hook
                       assess-test-local-hook)
                 (signal
                  'error
                  '("hook failure")))
               nil t)
            (error data)))
    (list
     condition
     (length escaped-hook)
     assess-test-local-hook
     (local-variable-p
      'assess-test-local-hook)))
  )
"##;
    let expect: Expect = expect![[r#"OK ((error "hook failure") 1 (local-sentinel) t)"#]];
    assert_assess_call_parity(elisp_form, expect);
}
