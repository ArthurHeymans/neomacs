use expect_test::expect;

use super::assert_acp_parity;

#[test]
fn acp_call_request_failure_honors_callback_arity_and_buffer_precedence() {
    let elisp_form = r##"(let ((context
                (generate-new-buffer
                 " *acp-failure-context*"))
               (explicit
                (generate-new-buffer
                 " *acp-failure-explicit*"))
               calls)
         (unwind-protect
             (let ((client
                    `((:context-buffer
                       .
                       ,context))))
               (acp--call-request-failure
                :client client
                :incoming-response
                `((:buffer
                   .
                   ,explicit)
                  (:on-failure
                   .
                   ,(lambda (error message)
                      (push
                       (list
                        'two
                        error
                        (map-elt
                         message
                         :object)
                        (buffer-name))
                       calls))))
                :error-data
                '((code . 1))
                :message
                '((:object
                   (id . 7))))
               (acp--call-request-failure
                :client client
                :incoming-response
                `((:on-failure
                   .
                   ,(lambda (error)
                      (push
                       (list
                        'one
                        error
                        (buffer-name))
                       calls))))
                :error-data
                '((code . 2))
                :message
                'ignored)
               (nreverse calls))
           (when
               (buffer-live-p context)
             (kill-buffer context))
           (when
               (buffer-live-p explicit)
             (kill-buffer explicit))))"##;
    let expect = expect![[
        r#"OK ((two ((code . 1)) ((id . 7)) " *acp-failure-explicit*") (one ((code . 2)) " *acp-failure-context*"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_fail_pending_requests_clears_queue_builds_errors_and_isolates_callback_failures() {
    let elisp_form = r##"(let (calls
               logs)
         (let* ((one
                 `((:on-failure
                    .
                    ,(lambda (error)
                       (push
                        (list
                         'one
                         error)
                        calls)))))
                (two
                 `((:on-failure
                    .
                    ,(lambda (error message)
                       (push
                        (list
                         'two
                         error
                         (map-elt
                          message
                          :object))
                        calls)))))
                (broken
                 `((:on-failure
                    .
                    ,(lambda (error)
                       error
                       (error
                        "callback broke")))))
                (client
                 `((:context-buffer . nil)
                   (:pending-requests
                    (1 . ,one)
                    (2 . ,two)
                    (3 . ,broken)
                    (4
                     (:on-failure . nil))))))
           (cl-letf
               (((symbol-function
                  'acp--log)
                 (lambda (_client label format-string &rest arguments)
                   (push
                    (list
                     label
                     format-string
                     arguments)
                    logs))))
             (acp--fail-pending-requests
              :client client
              :event " exited 42\n")
             (list
              (map-elt
               client
               :pending-requests)
              (nreverse calls)
              (nreverse logs)))))"##;
    let expect = expect![[
        r#"OK (nil ((one #1=((code . -32603) (message . "Agent process ended before completing request: exited 42"))) (two #1# ((jsonrpc . "2.0") (id . 2) (error . #1#)))) (("REQUEST FAILURE CALLBACK ERROR" "Failed with error: %S" ((error "callback broke")))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_fail_pending_requests_uses_plain_message_for_empty_event_and_ignores_empty_queue() {
    let elisp_form = r##"(let (calls)
         (let ((client
                `((:pending-requests
                   (5
                    (:on-failure
                     .
                     ,(lambda (error)
                        (push error calls))))))))
           (acp--fail-pending-requests
            :client client
            :event " \n")
           (acp--fail-pending-requests
            :client client
            :event "unused")
           (list
            (nreverse calls)
            (map-elt
             client
             :pending-requests))))"##;
    let expect = expect![[
        r#"OK ((((code . -32603) (message . "Agent process ended before completing request"))) nil)"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_route_incoming_success_handles_nil_results_callbacks_and_unhandled_responses() {
    let elisp_form = r##"(let (calls)
         (let* ((handled
                 `((:on-success
                    .
                    ,(lambda (result)
                       (push
                        (list
                         'success
                         result)
                        calls)))))
                (unhandled
                 '((:on-success . nil)))
                (client
                 `((:pending-requests
                    (1 . ,handled)
                    (2 . ,unhandled))
                   (:request-resolver . nil))))
           (map-put!
            client
            :request-resolver
            (lambda (&rest arguments)
              (map-nested-elt
               client
               (list
                :pending-requests
                (plist-get
                 arguments
                 :id)))))
           (cl-letf
               (((symbol-function
                  'acp--log)
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'log
                     (cdr arguments))
                    calls)))
                ((symbol-function
                  'acp--log-traffic)
                 (lambda (_client direction kind message)
                   (push
                    (list
                     'traffic
                     direction
                     kind
                     (map-elt
                      message
                      :object))
                    calls))))
             (list
              (acp--route-incoming-message
               :client client
               :message
               (acp--make-message
                :json "{\"id\":1,\"result\":null}"
                :object
                '((jsonrpc . "2.0")
                  (id . 1)
                  (result . nil)))
               :on-notification #'ignore
               :on-request #'ignore)
              (acp--route-incoming-message
               :client client
               :message
               (acp--make-message
                :json "{\"id\":2,\"result\":3}"
                :object
                '((jsonrpc . "2.0")
                  (id . 2)
                  (result . 3)))
               :on-notification #'ignore
               :on-request #'ignore)
              (map-elt
               client
               :pending-requests)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (t t nil ((log nil "↳ Routing as response (result)") (traffic incoming response ((jsonrpc . "2.0") (id . 1) (result))) (success nil) (log nil "↳ Routing as response (result)") (traffic incoming response #1=((jsonrpc . "2.0") (id . 2) (result . 3))) (log nil "Unhandled result:\n\n%s" ((:object . #1#) (:json . "{\"id\":2,\"result\":3}")))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_route_incoming_success_uses_response_buffer_then_client_context_then_temporary_fallback() {
    let elisp_form = r##"(let ((explicit
                (generate-new-buffer
                 " *acp-success-explicit*"))
               (context
                (generate-new-buffer
                 " *acp-success-context*"))
               calls)
         (unwind-protect
             (let* ((explicit-response
                     `((:buffer . ,explicit)
                       (:on-success
                        .
                        ,(lambda (result)
                           (push
                            (list
                             'explicit
                             (buffer-name)
                             result)
                            calls)))))
                    (context-response
                     `((:on-success
                        .
                        ,(lambda (result)
                           (push
                            (list
                             'context
                             (buffer-name)
                             result)
                            calls)))))
                    (fallback-response
                     `((:on-success
                        .
                        ,(lambda (result)
                           (push
                            (list
                             'fallback
                             (buffer-live-p
                              (current-buffer))
                             (not
                              (eq
                               (current-buffer)
                               explicit))
                             (not
                              (eq
                               (current-buffer)
                               context))
                             result)
                            calls)))))
                    (client
                     `((:context-buffer . ,context)
                       (:pending-requests
                        (1 . ,explicit-response)
                        (2 . ,context-response)
                        (3 . ,fallback-response))
                       (:request-resolver . nil))))
               (map-put!
                client
                :request-resolver
                (lambda (&rest arguments)
                  (map-nested-elt
                   client
                   (list
                    :pending-requests
                    (plist-get
                     arguments
                     :id)))))
               (cl-letf
                   (((symbol-function
                      'acp--log)
                     #'ignore)
                    ((symbol-function
                      'acp--log-traffic)
                     #'ignore))
                 (dolist
                     (id
                      '(1 2))
                   (acp--route-incoming-message
                    :client client
                    :message
                    (acp--make-message
                     :json
                     (format
                      "{\"id\":%d,\"result\":%d}"
                      id
                      id)
                     :object
                     `((jsonrpc . "2.0")
                       (id . ,id)
                       (result . ,id)))
                    :on-notification #'ignore
                    :on-request #'ignore))
                 (map-put!
                  client
                  :context-buffer
                  nil)
                 (acp--route-incoming-message
                  :client client
                  :message
                  (acp--make-message
                   :json
                   "{\"id\":3,\"result\":3}"
                   :object
                   '((jsonrpc . "2.0")
                     (id . 3)
                     (result . 3)))
                  :on-notification #'ignore
                  :on-request #'ignore)
                 (list
                  (map-elt
                   client
                   :pending-requests)
                  (nreverse calls))))
           (when
               (buffer-live-p explicit)
             (kill-buffer explicit))
           (when
               (buffer-live-p context)
             (kill-buffer context))))"##;
    let expect = expect![[
        r#"OK (nil ((explicit " *acp-success-explicit*" 1) (context " *acp-success-context*" 2) (fallback t t t 3)))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_route_incoming_failure_supports_two_callback_arities_and_unhandled_errors() {
    let elisp_form = r##"(let (calls)
         (let* ((one
                 `((:on-failure
                    .
                    ,(lambda (error)
                       (push
                        (list
                         'one
                         error)
                        calls)))))
                (two
                 `((:on-failure
                    .
                    ,(lambda (error message)
                       (push
                        (list
                         'two
                         error
                         (map-elt
                          message
                          :json))
                        calls)))))
                (none
                 '((:on-failure . nil)))
                (client
                 `((:pending-requests
                    (1 . ,one)
                    (2 . ,two)
                    (3 . ,none))
                   (:request-resolver . nil))))
           (map-put!
            client
            :request-resolver
            (lambda (&rest arguments)
              (map-nested-elt
               client
               (list
                :pending-requests
                (plist-get
                 arguments
                 :id)))))
           (cl-letf
               (((symbol-function
                  'acp--log)
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'log
                     (cdr arguments))
                    calls)))
                ((symbol-function
                  'acp--log-traffic)
                 (lambda (&rest arguments)
                   (push
                    (list
                     'traffic
                     (nth 2 arguments)
                     (nth 3 arguments))
                    calls))))
             (dolist
                 (id
                  '(1 2 3))
               (acp--route-incoming-message
                :client client
                :message
                (acp--make-message
                 :json
                 (format
                  "{\"id\":%d,\"error\":{}}"
                  id)
                 :object
                 `((jsonrpc . "2.0")
                   (id . ,id)
                   (error
                    (code . -1)
                    (message . "bad"))))
                :on-notification #'ignore
                :on-request #'ignore))
             (list
              (map-elt
               client
               :pending-requests)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (nil ((log nil "↳ Routing as response (error)") (traffic response ((:object (jsonrpc . "2.0") (id . 1) (error . #1=((code . -1) (message . "bad")))) (:json . "{\"id\":1,\"error\":{}}"))) (one #1#) (log nil "↳ Routing as response (error)") (traffic response ((:object (jsonrpc . "2.0") (id . 2) (error . #2=((code . -1) (message . "bad")))) (:json . "{\"id\":2,\"error\":{}}"))) (two #2# "{\"id\":2,\"error\":{}}") (log nil "↳ Routing as response (error)") (traffic response #3=((:object (jsonrpc . "2.0") (id . 3) (error (code . -1) (message . "bad"))) (:json . "{\"id\":3,\"error\":{}}"))) (log nil "Unhandled error:\n\n%s" #3#)))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_route_incoming_dispatches_requests_notifications_and_unknown_objects_exactly() {
    let elisp_form = r##"(let (calls)
         (let ((client
                `((:pending-requests)
                  (:request-resolver
                   .
                   ,(lambda (&rest arguments)
                      arguments
                      nil)))))
           (cl-letf
               (((symbol-function
                  'acp--log)
                 (lambda (&rest arguments)
                   (push
                    (list
                     'log
                     (nth 2 arguments))
                    calls)))
                ((symbol-function
                  'acp--log-traffic)
                 (lambda (_client direction kind message)
                   (push
                    (list
                     'traffic
                     direction
                     kind
                     (map-elt
                      message
                      :object))
                    calls))))
             (let ((request
                    '((jsonrpc . "2.0")
                      (id . 9)
                      (method . "fs/read")))
                   (notification
                    '((jsonrpc . "2.0")
                      (method . "session/update")))
                   (unknown
                    '((jsonrpc . "2.0")
                      (id . 10))))
               (list
                (acp--route-incoming-message
                 :client client
                 :message
                 (acp--make-message
                  :object request)
                 :on-request
                 (lambda (value)
                   (push
                    (list
                     'request
                     value)
                    calls))
                 :on-notification
                 (lambda (value)
                   (push
                    (list
                     'notification
                     value)
                    calls)))
                (acp--route-incoming-message
                 :client client
                 :message
                 (acp--make-message
                  :object notification)
                 :on-request #'ignore
                 :on-notification
                 (lambda (value)
                   (push
                    (list
                     'notification
                     value)
                    calls)))
                (acp--route-incoming-message
                 :client client
                 :message
                 (acp--make-message
                  :object unknown)
                 :on-request #'ignore
                 :on-notification #'ignore)
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (t t #3=((traffic incoming unknown ((jsonrpc . "2.0") (id . 10)))) ((log "↳ Routing as incoming request") (traffic incoming request #1=((jsonrpc . "2.0") (id . 9) (method . "fs/read"))) (request #1#) (log "↳ Routing as notification") (traffic incoming notification #2=((jsonrpc . "2.0") (method . "session/update"))) (notification #2#) (log "↳ Routing undefined (could not recognize)\n\n%s") . #3#))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_route_incoming_required_argument_errors_match_exactly() {
    let elisp_form = r##"(cl-labels
         ((capture
           (function)
           (condition-case error
               (funcall function)
             (error
              (list
               (car error)
               (cadr error))))))
         (mapcar
          #'capture
          (list
           (lambda ()
             (acp--route-incoming-message
              :client nil
              :on-notification #'ignore
              :on-request #'ignore))
           (lambda ()
             (acp--route-incoming-message
              :message
              '((:object))
              :on-request #'ignore))
           (lambda ()
             (acp--route-incoming-message
              :message
              '((:object))
              :on-notification #'ignore)))))"##;
    let expect = expect![[
        r#"OK ((error ":object is required") (error ":on-notification is required") (error ":on-request is required"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_stderr_api_error_parser_handles_valid_nested_api_errors_and_all_malformed_cases() {
    let elisp_form = r##"(list
         (acp--parse-stderr-api-error
          "Attempt 2 failed with status 429. Retrying soon ApiError: {\"error\":{\"message\":\"{\\\"error\\\":{\\\"code\\\":429,\\\"message\\\":\\\"quota\\\"}}\"}}")
         (acp--parse-stderr-api-error
          "Attempt 1 failed with status 500. Retrying ApiError: {bad}")
         (acp--parse-stderr-api-error
          "Attempt 1 failed with status 500. Retrying ApiError: {\"error\":{\"message\":\"bad inner\"}}")
         (acp--parse-stderr-api-error
          "ordinary stderr")
         (acp--parse-stderr-api-error
          ""))"##;
    let expect = expect![[r#"OK (((code . 429) (message . "quota")) nil nil nil nil)"#]];
    assert_acp_parity(elisp_form, expect);
}
