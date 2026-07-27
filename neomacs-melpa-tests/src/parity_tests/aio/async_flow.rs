use expect_test::expect;

use super::assert_aio_parity;

#[test]
fn aio_nested_async_functions_await_immediate_timer_and_error_results() {
    let elisp_form = r##"(progn
                      (aio-defun aio-parity-divide (a b)
                        (aio-await :immediate)
                        (aio-await (aio-sleep 0))
                        (/ a b))
                      (aio-defun aio-parity-safe-divide (a b)
                        (condition-case error
                            (list :ok
                                  (aio-await
                                   (aio-parity-divide a b)))
                          (arith-error
                           (list :error (car error)))))
                      (list
                       (aio-wait-for
                        (aio-parity-safe-divide 9.0 3.0))
                       (aio-wait-for
                        (aio-parity-safe-divide 1 0))))"##;
    let expect = expect!["OK ((:ok 3.0) (:error arith-error))"];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_with_async_and_all_join_concurrent_tasks_in_input_order() {
    let elisp_form = r##"(let ((events nil)
                          (promises nil))
                      (dolist (item '((:a 0.01) (:b 0) (:c 0.005)))
                        (push
                         (aio-with-async
                           (aio-await (aio-sleep (cadr item)))
                           (push (car item) events)
                           (car item))
                         promises))
                      (setq promises (nreverse promises))
                      (aio-wait-for
                       (aio-with-async
                         (aio-await (aio-all promises))))
                      (list
                       (mapcar
                        (lambda (promise)
                          (funcall (aio-result promise)))
                        promises)
                       (sort (copy-sequence events)
                             (lambda (a b)
                               (string-lessp
                                (symbol-name a)
                                (symbol-name b))))))"##;
    let expect = expect!["OK ((:a :b :c) (:a :b :c))"];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_chain_advances_repeating_callback_promises_inside_async_workflow() {
    let elisp_form = r##"(let* ((stream (aio-make-callback))
                          (emit (car stream))
                          (next (cdr stream))
                          (consumer
                           (aio-with-async
                             (list
                              (cdr (aio-chain next))
                              (cdr (aio-chain next))
                              (cdr (aio-chain next))))))
                      (funcall emit "one")
                      (funcall emit "two" 2)
                      (funcall emit "three" 3 :done)
                      (aio-wait-for consumer))"##;
    let expect = expect!["OK (nil (2) (3 :done))"];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_cancellation_propagates_through_await_and_can_be_recovered() {
    let elisp_form = r##"(let* ((gate (aio-promise))
                          (worker
                           (aio-with-async
                             (condition-case error
                                 (list :value (aio-await gate))
                               (aio-cancel
                                (list :cancelled (cdr error)))))))
                      (aio-cancel gate '(:shutdown 7))
                      (list
                       (aio-wait-for worker)
                       (aio-result gate)
                       (aio-promise-callbacks gate)))"##;
    let expect = expect![
        "OK ((:cancelled (:shutdown 7)) #[nil ((signal 'aio-cancel reason)) ((reason :shutdown 7))] nil)"
    ];
    assert_aio_parity(elisp_form, expect);
}
