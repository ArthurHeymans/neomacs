use expect_test::expect;

use super::assert_acp_fakes_parity;

#[test]
fn acp_fakes_make_client_installs_exact_queue_hooks_and_independent_message_copy() {
    let elisp_form = r##"(let* ((messages
                     '(((:direction . outgoing)
                        (:kind . request)
                        (:object
                         (id . 1)))))
                    (client
                     (acp-fakes-make-client
                      messages)))
         (setcar messages 'mutated)
         (list
          (map-elt
           client
           :command)
          (map-elt
           client
           :command-params)
          (map-elt
           client
           :environment-variables)
          (map-elt
           client
           :message-queue)
          (map-elt
           client
           :pending-requests)
          (map-elt
           client
           :request-id)
          (functionp
           (map-elt
            client
            :request-sender))
          (functionp
           (map-elt
            client
            :response-sender))
          (functionp
           (map-elt
            client
            :request-resolver))))"##;
    let expect = expect![[
        r#"OK ("cat" nil nil (((:direction . outgoing) (:kind . request) (:object (id . 1)))) nil 0 t t t)"#
    ]];
    assert_acp_fakes_parity(elisp_form, expect);
}

#[test]
fn acp_fakes_related_incoming_traffic_respects_request_response_windows_and_kinds() {
    let elisp_form = r##"(let ((messages
                '(((:direction . incoming)
                   (:kind . notification)
                   (:object
                    (method . "before")))
                  ((:direction . outgoing)
                   (:kind . request)
                   (:object
                    (id . 1)))
                  ((:direction . incoming)
                   (:kind . notification)
                   (:object
                    (method . "note")))
                  ((:direction . incoming)
                   (:kind . request)
                   (:object
                    (id . 20)
                    (method . "fs/read")))
                  ((:direction . incoming)
                   (:kind . unknown)
                   (:object
                    (method . "skip")))
                  ((:direction . incoming)
                   (:kind . response)
                   (:object
                    (id . 1)
                    (result . ok)))
                  ((:direction . incoming)
                   (:kind . notification)
                   (:object
                    (method . "after")))
                  ((:direction . outgoing)
                   (:kind . request)
                   (:object
                    (id . 2)))
                  ((:direction . incoming)
                   (:kind . notification)
                   (:object
                    (method . "two"))))))
         (list
          (acp-fakes--get-related-incoming-traffic
           :messages messages
           :request-id 1)
          (acp-fakes--get-related-incoming-traffic
           :messages messages
           :request-id 2)
          (acp-fakes--get-related-incoming-traffic
           :messages messages
           :request-id 9)))"##;
    let expect = expect![[
        r#"OK ((((:direction . incoming) (:kind . notification) (:object (method . "note"))) ((:direction . incoming) (:kind . request) (:object (id . 20) (method . "fs/read")))) (((:direction . incoming) (:kind . notification) (:object (method . "two")))) nil)"#
    ]];
    assert_acp_fakes_parity(elisp_form, expect);
}

#[test]
fn acp_fakes_request_sender_routes_related_traffic_then_resolves_success_and_removes_response() {
    let elisp_form = r##"(let* ((messages
                     '(((:direction . outgoing)
                        (:kind . request)
                        (:object
                         (id . 1)
                         (method . "initialize")))
                       ((:direction . incoming)
                        (:kind . notification)
                        (:object
                         (method . "update")))
                       ((:direction . incoming)
                        (:kind . request)
                        (:object
                         (id . 50)
                         (method . "fs/read")))
                       ((:direction . incoming)
                        (:kind . response)
                        (:object
                         (id . 1)
                         (result
                          (ok . t))))
                       ((:direction . incoming)
                        (:kind . notification)
                        (:object
                         (method . "later")))))
                    (client
                     (acp-fakes-make-client
                      messages))
                    calls)
         (acp-subscribe-to-notifications
          :client client
          :on-notification
          (lambda (value)
            (push
             (list
              'notification
              value)
             calls)))
         (acp-subscribe-to-requests
          :client client
          :on-request
          (lambda (value)
            (push
             (list
              'request
              value)
             calls)))
         (let ((result
                (acp-fakes--request-sender
                 :client client
                 :request
                 '((:method . "initialize"))
                 :on-success
                 (lambda (value)
                   (push
                    (list
                     'success
                     value)
                    calls))
                 :on-failure
                 (lambda (value)
                   (push
                    (list
                     'failure
                     value)
                    calls)))))
           (list
            result
            (map-elt
             client
             :request-id)
            (map-elt
             client
             :pending-requests)
            (map-elt
             client
             :message-queue)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (#3=((ok . t)) 1 nil (((:direction . outgoing) (:kind . request) (:object (id . 1) (method . "initialize"))) ((:direction . incoming) (:kind . notification) (:object . #1=((method . "update")))) ((:direction . incoming) (:kind . request) (:object . #2=((id . 50) (method . "fs/read")))) ((:direction . incoming) (:kind . notification) (:object (method . "later")))) ((notification #1#) (request #2#) (success #3#)))"#
    ]];
    assert_acp_fakes_parity(elisp_form, expect);
}

#[test]
fn acp_fakes_request_sender_resolves_errors_nil_results_and_missing_callbacks() {
    let elisp_form = r##"(cl-labels
         ((scenario
           (response on-success on-failure)
           (let* ((messages
                   `(((:direction . outgoing)
                      (:kind . request)
                      (:object
                       (id . 1)))
                     ((:direction . incoming)
                      (:kind . response)
                      (:object
                       (id . 1)
                       ,@response))))
                  (client
                   (acp-fakes-make-client
                    messages))
                  calls)
             (condition-case error
                 (list
                  'value
                  (acp-fakes--request-sender
                   :client client
                   :request 'ignored
                   :on-success
                   (and on-success
                        (lambda (value)
                          (push
                           (list
                            'success
                            value)
                           calls)))
                   :on-failure
                   (and on-failure
                        (lambda (value)
                          (push
                           (list
                            'failure
                            value)
                           calls))))
                  (nreverse calls))
               (error
                (list
                 'signal
                 (car error)
                 (cadr error)
                 (nreverse calls)))))))
         (list
          (scenario
           '((error
              (code . -1)
              (message . "bad")))
           nil
           t)
          (scenario
           '((result . nil))
           t
           nil)
          (scenario
           '((result . 3))
           nil
           nil)))"##;
    let expect = expect![[
        r#"OK ((value #1=((code . -1) (message . "bad")) ((failure #1#))) (value nil ((success nil))) (signal error "No matching response found for request 1" nil))"#
    ]];
    assert_acp_fakes_parity(elisp_form, expect);
}

#[test]
fn acp_fakes_replay_emits_outgoing_and_routes_only_incoming_requests_or_notifications() {
    let elisp_form = r##"(let* ((messages
                     '(((:direction . outgoing)
                        (:kind . request)
                        (:object
                         (method . "one")))
                       ((:direction . incoming)
                        (:kind . notification)
                        (:object
                         (method . "note")))
                       ((:direction . incoming)
                        (:kind . request)
                        (:object
                         (id . 8)
                         (method . "read")))
                       ((:direction . incoming)
                        (:kind . response)
                        (:object
                         (id . 1)
                         (result . ok)))))
                    (client
                     (acp-fakes-make-client
                      messages))
                    calls)
         (acp-subscribe-to-notifications
          :client client
          :on-notification
          (lambda (value)
            (push
             (list
              'notification
              value)
             calls)))
         (acp-subscribe-to-requests
          :client client
          :on-request
          (lambda (value)
            (push
             (list
              'request
              value)
             calls)))
         (list
          (acp-fakes-replay
           :client client
           :on-outgoing
           (lambda (value)
             (push
              (list
               'outgoing
               value)
              calls)))
          (nreverse calls)))"##;
    let expect = expect![[
        r#"OK (nil ((outgoing ((method . "one"))) (notification ((method . "note"))) (request ((id . 8) (method . "read")))))"#
    ]];
    assert_acp_fakes_parity(elisp_form, expect);
}

#[test]
fn acp_fakes_authenticate_lookup_returns_first_matching_outgoing_method() {
    let elisp_form = r##"(let ((messages
                '(((:direction . incoming)
                   (:object
                    (method . "authenticate")
                    (id . 0)))
                  ((:direction . outgoing)
                   (:object
                    (method . "initialize")
                    (id . 1)))
                  ((:direction . outgoing)
                   (:object
                    (method . "authenticate")
                    (id . 2)))
                  ((:direction . outgoing)
                   (:object
                    (method . "authenticate")
                    (id . 3))))))
         (list
          (acp-fakes--get-authenticate-request
           :messages messages)
          (acp-fakes--get-authenticate-request
           :messages
           '(((:direction . outgoing)
              (:object
               (method . "other")))))))"##;
    let expect = expect![[
        r#"OK (((:direction . outgoing) (:object (method . "authenticate") (id . 2))) nil)"#
    ]];
    assert_acp_fakes_parity(elisp_form, expect);
}

#[test]
fn acp_fakes_validation_and_noop_response_resolver_contracts_match() {
    let elisp_form = r##"(cl-labels
         ((capture
           (function)
           (condition-case error
               (funcall function)
             (error
              (list
               (car error)
               (cadr error))))))
         (list
          (acp-fakes--response-sender
           :response 'ignored)
          (acp-fakes--request-resolver
           :client 'ignored
           :id 1)
          (mapcar
           #'capture
           (list
            (lambda ()
              (acp-fakes--request-sender
               :request 'x))
            (lambda ()
              (acp-fakes--get-authenticate-request))
            (lambda ()
              (acp-fakes--get-related-incoming-traffic
               :request-id 1))
            (lambda ()
              (acp-fakes--get-related-incoming-traffic
               :messages '(x)))))))"##;
    let expect = expect![[
        r#"OK (nil nil ((error ":client is required") (error ":messages is required") (error ":messages is required") (error ":request-id is required")))"#
    ]];
    assert_acp_fakes_parity(elisp_form, expect);
}

#[test]
fn acp_fakes_embedded_smoke_fixture_without_direction_metadata_is_a_noop() {
    let elisp_form = r##"(let (messages)
         (cl-letf
             (((symbol-function
                'acp--client-started-p)
               (lambda (client)
                 client
                 t))
              ((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (push
                  (apply
                   #'format
                   format-string
                   arguments)
                  messages))))
           (list
            (acp-fakes--test-fake-client)
            (nreverse messages))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_acp_fakes_parity(elisp_form, expect);
}
