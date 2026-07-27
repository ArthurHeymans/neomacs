use expect_test::expect;

use super::assert_aio_parity;

#[test]
fn aio_promise_resolution_orders_listeners_and_schedules_late_listener() {
    let elisp_form = r##"(let ((promise (aio-promise))
                          events)
                      (aio-listen
                       promise
                       (lambda (value)
                         (push (list 'first (funcall value)) events)))
                      (aio-listen
                       promise
                       (lambda (value)
                         (push (list 'second (funcall value)) events)))
                      (let ((first-resolution
                             (aio-resolve promise (lambda () 42)))
                            (second-resolution
                             (aio-resolve promise (lambda () 99))))
                        (aio-listen
                         promise
                         (lambda (value)
                           (push (list 'late (funcall value)) events)))
                        (aio-wait-for (aio-sleep 0))
                        (list
                         first-resolution
                         second-resolution
                         (funcall (aio-result promise))
                         (nreverse events)
                         (aio-promise-callbacks promise))))"##;
    let expect = expect!["OK (nil nil 42 ((first 42) (second 42) (late 42)) nil)"];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_with_promise_catch_and_cancel_preserve_success_and_error_data() {
    let elisp_form = r##"(let ((success (aio-promise))
                          (failure (aio-promise))
                          (cancelled (aio-promise)))
                      (aio-with-promise success
                        (+ 20 22))
                      (aio-with-promise failure
                        (signal 'wrong-type-argument
                                '(numberp "bad")))
                      (let ((first-cancel
                             (aio-cancel cancelled
                                         '(:user "stop")))
                            (second-cancel
                             (aio-cancel cancelled :again)))
                        (list
                         (aio-wait-for (aio-catch success))
                         (aio-wait-for (aio-catch failure))
                         first-cancel
                         second-cancel
                         (aio-wait-for (aio-catch cancelled)))))"##;
    let expect = expect![[
        r#"OK ((:success . 42) (:error wrong-type-argument numberp "bad") nil nil (:error aio-cancel :user "stop"))"#
    ]];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_repeating_callback_chain_tag_and_once_modes_match() {
    let elisp_form = r##"(let* ((repeat (aio-make-callback :tag :chunk))
                          (repeat-fn (car repeat))
                          (current (cdr repeat))
                          (once (aio-make-callback :tag :done :once t))
                          (once-fn (car once))
                          (once-promise (cdr once)))
                      (funcall repeat-fn "alpha" 1)
                      (let* ((first (aio-wait-for current))
                             (next (car first)))
                        (funcall repeat-fn "beta" 2)
                        (funcall once-fn "final" 9)
                        (funcall once-fn "ignored" 10)
                        (list
                         (cdr first)
                         (cdr (aio-wait-for next))
                         (aio-wait-for once-promise))))"##;
    let expect = expect![[r#"OK ((:chunk "alpha" 1) (:chunk "beta" 2) (:done "final" 9))"#]];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_internal_queue_is_fifo_reusable_and_reports_empty_state() {
    let elisp_form = r##"(let ((queue (cons nil nil)))
                      (list
                       (aio--queue-empty-p queue)
                       (aio--queue-get queue)
                       (aio--queue-put queue :a)
                       (aio--queue-put queue :b)
                       (aio--queue-put queue :c)
                       (aio--queue-empty-p queue)
                       (aio--queue-get queue)
                       (aio--queue-get queue)
                       (aio--queue-put queue :d)
                       (aio--queue-get queue)
                       (aio--queue-get queue)
                       (aio--queue-get queue)
                       (aio--queue-empty-p queue)
                       queue))"##;
    let expect = expect!["OK (t nil :a :b :c nil :a :b :d :c :d nil t (nil))"];
    assert_aio_parity(elisp_form, expect);
}
