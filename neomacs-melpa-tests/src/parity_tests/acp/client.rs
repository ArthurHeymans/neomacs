use expect_test::expect;

use super::assert_acp_parity;

#[test]
fn acp_make_client_populates_defaults_custom_hooks_and_monotonic_instance_ids() {
    let elisp_form = r##"(let ((acp-instance-count 0)
               (context
                (generate-new-buffer
                 " *acp-client-context*")))
         (unwind-protect
             (let ((default
                    (acp-make-client
                     :command "agent"))
                   (custom
                    (acp-make-client
                     :context-buffer context
                     :command "custom"
                     :command-params
                     '("--one" "two")
                     :environment-variables
                     '("A=1")
                     :request-sender
                     #'ignore
                     :notification-sender
                     #'identity
                     :request-resolver
                     #'car
                     :response-sender
                     #'cdr
                     :outgoing-request-decorator
                     #'copy-tree)))
               (mapcar
                (lambda (client)
                  (list
                   (and
                    (bufferp
                     (map-elt
                      client
                      :context-buffer))
                    (buffer-name
                     (map-elt
                      client
                      :context-buffer)))
                   (map-elt client :instance-count)
                   (map-elt client :process)
                   (map-elt client :command)
                   (map-elt client :command-params)
                   (map-elt client :environment-variables)
                   (map-elt client :pending-requests)
                   (map-elt client :request-id)
                   (map-elt client :notification-handlers)
                   (map-elt client :request-handlers)
                   (map-elt client :error-handlers)
                   (map-elt client :request-sender)
                   (map-elt client :notification-sender)
                   (map-elt client :request-resolver)
                   (map-elt client :response-sender)
                   (map-elt
                    client
                    :outgoing-request-decorator)))
                (list default custom)))
           (when
               (buffer-live-p context)
             (kill-buffer context))))"##;
    let expect = expect![[
        r#"OK ((nil 1 nil "agent" nil nil nil 0 nil nil nil acp--request-sender acp--notification-sender acp--request-resolver acp--response-sender nil) (" *acp-client-context*" 2 nil "custom" ("--one" "two") ("A=1") nil 0 nil nil nil ignore identity car cdr copy-tree))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_make_client_and_started_predicate_validate_command_and_process_liveness() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'process-live-p)
               (lambda (process)
                 (push process calls)
                 (eq process
                     'live))))
           (list
            (condition-case error
                (acp-make-client)
              (error
               (list
                (car error)
                (cadr error))))
            (acp--client-started-p
             '((:process . nil)))
            (acp--client-started-p
             '((:process . dead)))
            (acp--client-started-p
             '((:process . live)))
            (nreverse calls))))"##;
    let expect = expect![[r#"OK ((error ":command is required") nil nil t (dead live))"#]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_subscriptions_validate_inputs_prepend_handlers_and_select_exact_buffer_precedence() {
    let elisp_form = r##"(let ((context
                (generate-new-buffer
                 " *acp-sub-context*"))
               (explicit
                (generate-new-buffer
                 " *acp-sub-explicit*"))
               (dead
                (generate-new-buffer
                 " *acp-sub-dead*"))
               calls)
         (kill-buffer dead)
         (unwind-protect
             (let ((client
                    (acp-make-client
                     :context-buffer context
                     :command "agent"))
                   (fallback-client
                    (acp-make-client
                     :command "agent")))
               (acp-subscribe-to-notifications
                :client client
                :buffer explicit
                :on-notification
                (lambda (value)
                  (push
                   (list
                    'notification
                    value
                    (buffer-name))
                   calls)))
               (acp-subscribe-to-notifications
                :client client
                :buffer explicit
                :on-notification
                (lambda (value)
                  (push
                   (list
                    'notification-new
                    value
                    (buffer-name))
                   calls)))
               (acp-subscribe-to-requests
                :client client
                :on-request
                (lambda (value)
                  (push
                   (list
                    'request
                    value
                    (buffer-name))
                   calls)))
               (acp-subscribe-to-errors
                :client client
                :buffer dead
                :on-error
                (lambda (value)
                  (push
                   (list
                    'error
                    value
                    (buffer-name))
                   calls)))
               (acp-subscribe-to-notifications
                :client fallback-client
                :on-notification
                (lambda (value)
                  (push
                   (list
                    'fallback
                    value
                    (buffer-live-p
                     (current-buffer))
                    (not
                     (eq
                      (current-buffer)
                      context))
                    (not
                     (eq
                      (current-buffer)
                      explicit)))
                   calls)))
               (mapc
                (lambda (handler)
                  (funcall
                   handler
                   'n))
                (map-elt
                 client
                 :notification-handlers))
               (funcall
                (car
                 (map-elt
                  client
                  :request-handlers))
                'r)
               (funcall
                (car
                 (map-elt
                  client
                  :error-handlers))
                'e)
               (funcall
                (car
                 (map-elt
                  fallback-client
                  :notification-handlers))
                'f)
               (list
                (mapcar
                 (lambda (handlers)
                   (length
                    (map-elt
                     client
                     handlers)))
                 '(:notification-handlers
                   :request-handlers
                   :error-handlers))
                (nreverse calls)))
           (when
               (buffer-live-p context)
             (kill-buffer context))
           (when
               (buffer-live-p explicit)
             (kill-buffer explicit))))"##;
    let expect = expect![[
        r#"OK ((2 1 1) ((notification-new n " *acp-sub-explicit*") (notification n " *acp-sub-explicit*") (request r " *acp-sub-context*") (error e " *acp-sub-context*") (fallback f t t t)))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_subscription_required_argument_errors_match_for_all_three_channels() {
    let elisp_form = r##"(cl-labels
         ((capture
           (function)
           (condition-case error
               (funcall function)
             (error
              (list
               (car error)
               (cadr error))))))
         (let ((client
                (acp-make-client
                 :command "agent")))
           (mapcar
            #'capture
            (list
             (lambda ()
               (acp-subscribe-to-notifications
                :on-notification #'ignore))
             (lambda ()
               (acp-subscribe-to-notifications
                :client client))
             (lambda ()
               (acp-subscribe-to-requests
                :on-request #'ignore))
             (lambda ()
               (acp-subscribe-to-requests
                :client client))
             (lambda ()
               (acp-subscribe-to-errors
                :on-error #'ignore))
             (lambda ()
               (acp-subscribe-to-errors
                :client client))))))"##;
    let expect = expect![[
        r#"OK ((error ":client is required") (error ":on-notification is required") (error ":client is required") (error ":on-request is required") (error ":client is required") (error ":on-error is required"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_public_send_wrappers_start_only_when_needed_and_forward_exact_keywords() {
    let elisp_form = r##"(let (calls
               started)
         (let ((client
                (acp-make-client
                 :command "agent"
                 :request-sender
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'request
                     arguments)
                    calls)
                   'request-result)
                 :notification-sender
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'notification
                     arguments)
                    calls)
                   'notification-result)
                 :response-sender
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'response
                     arguments)
                    calls)
                   'response-result))))
           (cl-letf
               (((symbol-function
                  'acp--client-started-p)
                 (lambda (value)
                   value
                   started))
                ((symbol-function
                  'acp--start-client)
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'start
                     arguments)
                    calls)
                   (setq started t))))
             (list
              (acp-send-request
               :client client
               :request
               '((:method . "one"))
               :buffer 'buffer
               :on-success 'success
               :on-failure 'failure
               :sync t)
              (acp-send-notification
               :client client
               :notification
               '((:method . "note"))
               :sync nil)
              (acp-send-response
               :client client
               :response
               '((:request-id . 4)))
              (mapcar
               (lambda (call)
                 (cons
                  (car call)
                  (let ((arguments
                         (copy-sequence
                          (cdr call))))
                    (plist-put
                     arguments
                     :client
                     (eq
                      (plist-get
                       arguments
                       :client)
                      client)))))
               (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (request-result notification-result response-result ((start :client t) (request :client t :request ((:method . "one")) :buffer buffer :on-success success :on-failure failure :sync t) (notification :client t :notification ((:method . "note")) :sync nil) (response :client t :response ((:request-id . 4)))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_public_send_wrapper_validation_errors_match_exactly() {
    let elisp_form = r##"(cl-labels
         ((capture
           (function)
           (condition-case error
               (funcall function)
             (error
              (list
               (car error)
               (cadr error))))))
         (let ((client
                (acp-make-client
                 :command "agent")))
           (mapcar
            #'capture
            (list
             (lambda ()
               (acp-send-request
                :request 'x))
             (lambda ()
               (acp-send-request
                :client client))
             (lambda ()
               (acp-send-notification
                :notification 'x))
             (lambda ()
               (acp-send-notification
                :client client))
             (lambda ()
               (acp-send-response
                :response 'x))
             (lambda ()
               (acp-send-response
                :client client))))))"##;
    let expect = expect![[
        r#"OK ((error ":client is required") (error ":request is required") (error ":client is required") (error ":notification is required") (error ":client is required") (error ":response is required"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_shutdown_deletes_live_process_kills_buffers_and_validates_client_state() {
    let elisp_form = r##"(let ((acp-instance-count 0)
               calls)
         (let ((fresh
                (acp-make-client
                 :command "fresh"))
               (dead
                (acp-make-client
                 :command "dead")))
           (acp-logs-buffer
            :client fresh)
           (acp-traffic-buffer
            :client fresh)
           (map-put!
            fresh
            :process
            'live-process)
           (map-put!
            dead
            :process
            'dead-process)
           (cl-letf
               (((symbol-function
                  'process-live-p)
                 (lambda (process)
                   (push
                   (list
                     'live
                     process)
                    calls)
                   (eq process
                       'live-process)))
                ((symbol-function
                  'delete-process)
                 (lambda (process)
                   (push
                    (list
                     'delete
                     process)
                    calls)
                   nil))
                ((symbol-function
                  'message)
                 (lambda (&rest arguments)
                   (push
                    (cons
                     'message
                     arguments)
                    calls))))
             (list
              (acp-shutdown
               :client fresh)
              (get-buffer
               "*acp-(fresh)-1 log*")
              (get-buffer
               "*acp-(fresh)-1 traffic*")
              (condition-case error
                  (acp-shutdown)
                (error
                 (list
                  (car error)
                  (cadr error))))
              (acp-shutdown
               :client dead)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (t nil nil (error ":client is required") #1=((message "Client already shut down")) ((live live-process) (live live-process) (delete live-process) (live dead-process) . #1#))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}
