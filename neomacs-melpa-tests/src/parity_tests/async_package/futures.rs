use expect_test::expect;

use super::{assert_async_batch};

#[test]
fn futures_public_surface_batch() {
    assert_async_batch(&[
        (
            "async_start_future_returns_structured_unicode_and_transitions_to_ready",
            r##"(let* ((future
                      (async-start
                       (lambda ()
                         (sleep-for 0.1)
                         (list
                          "λ雪"
                          [1 two 3]
                          '(:nested
                            ((left . right)))))))
                     (ready-before
                      (async-ready future))
                     (value (async-get future)))
               (list
                ready-before
                value
                (async-ready future)
                (buffer-live-p
                 (process-buffer future))))"##,
            true,
            expect![[r#"OK (nil ("λ雪" [1 two 3] (:nested ((left . right)))) t nil)"#]],
        ),
        (
            "async_start_future_resignals_the_exact_child_error",
            r##"(async-get
               (async-start
                (lambda ()
                  (signal
                   'wrong-type-argument
                   '(integerp child-value)))))"##,
            false,
            expect![[r#"ERR (wrong-type-argument integerp child-value)"#]],
        ),
        (
            "async_start_callback_receives_messages_before_the_final_result",
            r##"(let (events)
               (let ((future
                      (async-start
                       (lambda ()
                         (async-send
                          :phase 'first
                          :payload "λ")
                         (async-send
                          :phase 'second
                          :payload '(1 2 3))
                         'finished)
                       (lambda (value)
                         (push value events)))))
                 (async-wait future)
                 (list
                  (nreverse events)
                  (async-get future)
                  (buffer-live-p
                   (process-buffer future)))))"##,
            true,
            expect![[
        r#"OK (((:phase first :payload "����" :async-message t) (:phase second :payload (1 2 3) :async-message t) finished) nil nil)"#
    ]],
        ),
        (
            "async_send_and_receive_transport_a_parent_message_into_the_child",
            r##"(let (received)
               (let ((future
                      (async-start
                       (lambda ()
                         (let ((message
                                (async-receive)))
                           (list
                            (plist-get
                             message :operation)
                            (apply
                             #'+
                             (plist-get
                              message :values))
                            (async-message-p
                             message))))
                       (lambda (value)
                         (setq received value)))))
                 (async-send
                  future
                  :operation 'sum
                  :values '(2 3 5 7))
                 (async-wait future)
                 (list
                  received
                  (async-get future)
                  (async-ready future))))"##,
            true,
            expect![[r#"OK ((sum 17 t) nil t)"#]],
        ),
        (
            "async_start_callback_reassembles_a_message_larger_than_a_process_chunk",
            r##"(let (events)
               (let ((future
                      (async-start
                       (lambda ()
                         (async-send
                          :payload
                          (make-string 65536 ?x))
                         'finished)
                       (lambda (value)
                         (push
                          (if
                              (async-message-p
                               value)
                              (list
                               'message
                               (length
                                (plist-get
                                 value :payload))
                               (substring
                                (plist-get
                                 value :payload)
                                0 3)
                               (substring
                                (plist-get
                                 value :payload)
                                -3))
                            value)
                          events)))))
                 (async-wait future)
                 (nreverse events)))"##,
            true,
            expect![[r#"OK ((message 65536 "xxx" "xxx") finished)"#]],
        ),
        (
            "async_inject_variables_recreates_the_selected_parent_environment_in_child",
            r##"(progn
               (defvar
                 neomacs-async-child-string nil)
               (defvar
                 neomacs-async-child-list nil)
               (setq
                neomacs-async-child-string
                (propertize
                 "from-parent" 'face 'bold)
                neomacs-async-child-list
                '(one (two . three)))
               (let ((injection
                      (async-inject-variables
                       "\\`neomacs-async-child-"
                       nil nil t)))
                 (async-get
                  (async-start
                   `(lambda ()
                      ,injection
                      (list
                       neomacs-async-child-string
                       (text-properties-at
                        0
                        neomacs-async-child-string)
                       neomacs-async-child-list))))))"##,
            true,
            expect![[r#"OK ("from-parent" nil (one (two . three)))"#]],
        ),
        (
            "async_callback_future_yields_nil_to_async_get_after_delivery",
            r##"(let (received)
               (let ((future
                      (async-start
                       (lambda () 42)
                       (lambda (value)
                         (setq received value)))))
                 (async-wait future)
                 (list
                  received
                  (async-get future)
                  (async-ready future))))"##,
            true,
            expect![[r#"OK (42 nil t)"#]],
        ),
        (
            "async_let_delivers_all_binding_values_to_its_final_parent_callback",
            r##"(let (received)
               (let ((outer
                      (async-let
                          ((x (+ 1 2))
                           (y (+ 3 4)))
                        (setq received
                              (list x y)))))
                 (async-wait outer)
                 (let ((deadline
                        (+ (float-time) 10)))
                   (while
                       (and
                        (null received)
                        (< (float-time)
                           deadline))
                     (accept-process-output
                      nil 0.05)))
                 received))"##,
            true,
            expect![[r#"OK (3 7)"#]],
        ),
        (
            "async_sandbox_returns_the_child_value_synchronously",
            r##"(async-sandbox
               (lambda ()
                 (let ((values
                        '(1 2 3 4 5)))
                   (list
                    (apply #'+ values)
                    (mapcar
                     (lambda (value)
                       (* value value))
                     values)
                    "λ雪"))))"##,
            true,
            expect![[r#"OK (15 (1 4 9 16 25) "λ雪")"#]],
        ),
    ]);
}
