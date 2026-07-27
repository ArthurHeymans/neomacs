use expect_test::expect;

use super::assert_anki_editor_parity;

#[test]
fn queue_primitives_push_toggle_and_clear_only_the_previous_active_queue() {
    let elisp_form = r##"(let ((anki-editor--api-active-queue
                           1)
                          (anki-editor--api-request-queue-1
                           nil)
                          (anki-editor--api-request-queue-2
                           '(:preserved)))
                      (let ((request-a
                             (anki-editor-api--make-queued-request
                              '(:action a)
                              'success-a
                              'error-a))
                            (request-b
                             (anki-editor-api--make-queued-request
                              '(:action b)
                              'success-b
                              'error-b)))
                        (anki-editor-api--push-active-queue
                         request-a)
                        (anki-editor-api--push-active-queue
                         request-b)
                        (let ((before
                               (list
                                anki-editor--api-active-queue
                                (anki-editor-api--get-active-queue)
                                anki-editor--api-request-queue-1
                                anki-editor--api-request-queue-2)))
                          (anki-editor-api--toggle-active-queue)
                          (list
                           before
                           anki-editor--api-active-queue
                           (anki-editor-api--get-active-queue)
                           anki-editor--api-request-queue-1
                           anki-editor--api-request-queue-2))))"##;
    let expect = expect![
        "OK ((1 #1=((:request (:action b) :success success-b :error error-b) (:request (:action a) :success success-a :error error-a)) #1# #2=(:preserved)) 2 #2# nil #2#)"
    ];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn enqueue_builds_exact_versioned_request_plists_with_optional_params_and_callbacks() {
    let elisp_form = r##"(let ((anki-editor--api-active-queue
                           1)
                          (anki-editor--api-request-queue-1
                           nil)
                          (anki-editor--api-request-queue-2
                           nil))
                      (list
                       (anki-editor-api-enqueue-request
                        'deckNames
                        nil
                        :success 'deck-success)
                       (anki-editor-api-enqueue-request
                        'findNotes
                        '(:query
                          "deck:Study tag:due")
                        :success 'find-success
                        :error 'find-error)
                       anki-editor--api-active-queue
                       anki-editor--api-request-queue-1
                       anki-editor--api-request-queue-2))"##;
    let expect = expect![[
        r#"OK (#1=((:request (:action deckNames :version 6) :success deck-success :error nil)) #2=((:request (:action findNotes :version 6 :params (:query "deck:Study tag:due")) :success find-success :error find-error) . #1#) 1 #2# nil)"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn dispatch_batches_requests_in_enqueue_order_and_routes_success_error_callbacks() {
    let elisp_form = r##"(let ((anki-editor--api-active-queue
                           1)
                          (anki-editor--api-request-queue-1
                           nil)
                          (anki-editor--api-request-queue-2
                           nil)
                          callback-events
                          api-calls
                          progress)
                      (cl-letf
                          (((symbol-function
                             'anki-editor-api-call)
                            (lambda (action &rest params)
                              (push
                               (cons action params)
                               api-calls)
                              '((result
                                 ((result . 10)
                                  (error))
                                 ((result)
                                  (error
                                   . "bad query"))
                                 ((result . 30)
                                  (error)))
                                (error))))
                           ((symbol-function
                             'anki-editor--draw-progress-bar)
                            (lambda (&rest arguments)
                              (push arguments progress))))
                        (anki-editor-api-enqueue-request
                         'first
                         nil
                         :success
                         (lambda (value)
                           (push
                            (list 'first value)
                            callback-events)
                           'first-ok))
                        (anki-editor-api-enqueue-request
                         'second
                         '(:query "bad")
                         :success
                         (lambda (value)
                           (push
                            (list 'unexpected value)
                            callback-events))
                         :error
                         (lambda (message)
                           (push
                            (list 'second message)
                            callback-events)
                           'second-error))
                        (anki-editor-api-enqueue-request
                         'third
                         '(:value 3)
                         :success
                         (lambda (value)
                           (push
                            (list 'third value)
                            callback-events)
                           'third-ok))
                        (list
                         (anki-editor-api-dispatch-queue)
                         anki-editor--api-active-queue
                         anki-editor--api-request-queue-1
                         anki-editor--api-request-queue-2
                         (nreverse api-calls)
                         (nreverse callback-events)
                         (nreverse progress))))"##;
    let expect = expect![[
        r#"OK ((:count 3 :successes 2 :errors 1 :results (first-ok second-error third-ok)) 2 nil nil ((multi :actions [(:action first :version 6) (:action second :version 6 :params (:query "bad")) (:action third :version 6 :params (:value 3))])) ((first 10) (second "bad query") (third 30)) (("Processing responses" 1 3 0) ("Processing responses" 2 3 0) ("Processing responses" 3 3 1)))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn callback_failure_is_warned_and_callback_enqueued_followup_uses_other_queue() {
    let elisp_form = r##"(let ((anki-editor--api-active-queue
                           1)
                          (anki-editor--api-request-queue-1
                           nil)
                          (anki-editor--api-request-queue-2
                           nil)
                          (dispatch-count 0)
                          api-calls warnings)
                      (cl-letf
                          (((symbol-function
                             'anki-editor-api-call)
                            (lambda (action &rest params)
                              (cl-incf dispatch-count)
                              (push
                               (cons action params)
                               api-calls)
                              (if
                                  (= dispatch-count 1)
                                  '((result
                                     ((result . 1)
                                      (error))
                                     ((result . 2)
                                      (error)))
                                    (error))
                                '((result
                                   ((result . 99)
                                    (error)))
                                  (error)))))
                           ((symbol-function
                             'anki-editor--draw-progress-bar)
                            (lambda (&rest _)))
                           ((symbol-function 'warn)
                            (lambda (&rest arguments)
                              (push arguments warnings))))
                        (anki-editor-api-enqueue-request
                         'broken
                         nil
                         :success
                         (lambda (_value)
                           (error
                            "handler exploded")))
                        (anki-editor-api-enqueue-request
                         'producer
                         nil
                         :success
                         (lambda (value)
                           (anki-editor-api-enqueue-request
                            'followup
                            (list :from value)
                            :success
                            (lambda (followup)
                              (list
                               'followup-result
                               followup)))
                           'followup-queued))
                        (let ((first
                               (anki-editor-api-dispatch-queue))
                              active-after-first
                              queue-after-first)
                          (setq active-after-first
                                anki-editor--api-active-queue
                                queue-after-first
                                (anki-editor-api--get-active-queue))
                          (list
                           first
                           active-after-first
                           queue-after-first
                           (anki-editor-api-dispatch-queue)
                           anki-editor--api-active-queue
                           (anki-editor-api--get-active-queue)
                           (nreverse api-calls)
                           (nreverse warnings)))))"##;
    let expect = expect![[
        r#"OK ((:count 2 :successes 2 :errors 0 :results (#5=(("%s handler failed.\n\nrequest: %s\n\nresponse: %s\n\nhandler: %s" "success" (:request #3=(:action broken :version 6) :success #1=#[(_value) ((error "handler exploded")) #2=(t)] :error nil) ((result . 1) (error)) #1#)) followup-queued)) 2 ((:request #4=(:action followup :version 6 :params (:from 2)) :success #[(followup) ((list 'followup-result followup)) #2#] :error nil)) (:count 1 :successes 1 :errors 0 :results ((followup-result 99))) 1 nil ((multi :actions [#3# (:action producer :version 6)]) (multi :actions [#4#])) #5#)"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}
