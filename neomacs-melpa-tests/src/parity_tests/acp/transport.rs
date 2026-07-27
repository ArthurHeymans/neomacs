use expect_test::expect;

use super::assert_acp_parity;

#[test]
fn acp_start_client_validates_client_command_executable_and_started_state() {
    let elisp_form = r##"(cl-labels
         ((capture
           (function)
           (condition-case error
               (funcall function)
             (error
              (list
               (car error)
               (cadr error))))))
         (cl-letf
             (((symbol-function
                'executable-find)
               (lambda (command &optional remote)
                 remote
                 (unless
                     (equal command
                            "missing")
                   (concat
                    "/fixture/bin/"
                    command))))
              ((symbol-function
                'acp--client-started-p)
               (lambda (client)
                 (equal
                  (map-elt client :command)
                  "started"))))
           (mapcar
            #'capture
            (list
             (lambda ()
               (acp--start-client))
             (lambda ()
               (acp--start-client
                :client
                '((:command . nil))))
             (lambda ()
               (acp--start-client
                :client
                '((:command . "missing"))))
             (lambda ()
               (acp--start-client
                :client
                '((:command . "started"))))))))"##;
    let expect = expect![[
        r#"OK ((error ":client is required") (error ":command is required") (error "\"missing\" command line utility not found.  Please install it") (error "Client already started"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_start_client_builds_remote_utf8_environment_hooks_and_process_contract() {
    let elisp_form = r##"(let ((client
                (acp-make-client
                 :command "agent"
                 :command-params
                 '("--stdio" "✓")
                 :environment-variables
                 '("ACP_FIXTURE=client"
                   "CLIENT_ONLY=1")))
               (process-environment
                (cons
                 "ACP_FIXTURE=ambient"
                 process-environment))
               process-spec
               environment-seen
               executable-seen)
         (unwind-protect
             (cl-letf
                  (((symbol-function
                    'executable-find)
                   (lambda (command &optional remote)
                     (setq executable-seen
                           (list command remote))
                     "/fixture/bin/agent"))
                  ((symbol-function
                    'file-remote-p)
                   (lambda (&rest arguments)
                     arguments
                     "remote-marker"))
                  ((symbol-function
                    'acp--client-started-p)
                   (lambda (value)
                     value
                     nil))
                  ((symbol-function
                    'make-process)
                   (lambda (&rest arguments)
                     (setq process-spec
                           arguments
                           environment-seen
                           (list
                            (nth 0 process-environment)
                            (nth 1 process-environment)
                            (nth 2 process-environment)
                            coding-system-for-read
                            coding-system-for-write))
                     'fixture-process)))
               (acp--start-client
                :client client)
               (let ((stderr
                      (plist-get
                       process-spec
                       :stderr)))
                 (list
                  (map-elt
                   client
                   :process)
                  (plist-get
                   process-spec
                   :name)
                  (plist-get
                   process-spec
                   :command)
                  (and
                   (bufferp stderr)
                   (buffer-name stderr))
                  (plist-get
                   process-spec
                   :connection-type)
                  (plist-get
                   process-spec
                   :noquery)
                  (plist-get
                   process-spec
                   :file-handler)
                  executable-seen
                  (functionp
                   (plist-get
                    process-spec
                    :filter))
                  (functionp
                   (plist-get
                    process-spec
                    :sentinel))
                  environment-seen
                  (with-current-buffer stderr
                    (list
                     (local-variable-p
                      'after-change-functions)
                     (length
                      after-change-functions))))))
           (when-let*
               ((buffer
                 (get-buffer
                  "acp-client-stderr(agent)-1")))
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (fixture-process "acp-client(agent)-1" ("agent" "--stdio" "✓") "acp-client-stderr(agent)-1" pipe t "remote-marker" ("agent" "remote-marker") t t ("ACP_FIXTURE=client" "CLIENT_ONLY=1" "ACP_FIXTURE=ambient" utf-8-unix utf-8-unix) (t 2))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_start_client_filter_buffers_partial_lines_queues_valid_json_and_recovers_after_errors() {
    let elisp_form = r##"(let ((client
                (acp-make-client
                 :command "agent"))
               process-spec
               events
               traffic
               logs
               timers
               scheduled)
         (map-put!
          client
          :notification-handlers
          (list
           (lambda (object)
             object
             (error
              "notification fixture failure"))
           (lambda (object)
             (push
              (list
               'notification
               object)
              events))))
         (map-put!
          client
          :request-handlers
          (list
           (lambda (object)
             object
             (error
              "request fixture failure"))
           (lambda (object)
             (push
              (list
               'request
               object)
              events))))
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'executable-find)
                   (lambda (&rest arguments)
                     arguments
                     "/fixture/bin/agent"))
                  ((symbol-function
                    'acp--client-started-p)
                   (lambda (value)
                     value
                     nil))
                  ((symbol-function
                    'make-process)
                   (lambda (&rest arguments)
                     (setq process-spec
                           arguments)
                     'fixture-process))
                  ((symbol-function
                    'acp--parse-json)
                   (lambda (json)
                     (cond
                      ((equal json
                              "one")
                       '((method . "one")))
                      ((equal json
                              "two")
                       '((id . 2)
                         (method . "two")))
                      (t
                       (error
                        "invalid fixture JSON")))))
                  ((symbol-function
                    'acp--log)
                   (lambda (_client label format-string &rest arguments)
                     (push
                      (list
                       label
                       format-string
                       arguments)
                      logs)))
                  ((symbol-function
                    'run-at-time)
                   (lambda (_seconds _repeat function)
                     (setq timers
                           (1+
                            (or timers 0)))
                     (push function scheduled)
                     'fixture-timer))
                  ((symbol-function
                    'acp--log-traffic)
                   (lambda (_client direction kind message)
                     (push
                      (list
                       direction
                       kind
                       (map-elt
                        message
                        :json)
                       (map-elt
                        message
                        :object))
                      traffic))))
               (acp--start-client
                :client client)
               (let ((filter
                      (plist-get
                       process-spec
                       :filter)))
                 (funcall
                  filter
                  'fixture-process
                  "one")
                 (funcall
                  filter
                  'fixture-process
                  "\ntwo\nbad\npartial")
                 (funcall
                 filter
                  'fixture-process
                  "-tail\n")
                 (let ((before-drain
                        (list
                         timers
                         (length scheduled)
                         (length events)
                         (length traffic))))
                   (funcall
                    (car scheduled))
                   (let ((after-first-drain
                          (list
                           timers
                           (length scheduled)
                           (length events)
                           (length traffic))))
                     (funcall
                      filter
                      'fixture-process
                      "one\n")
                     (let ((after-reschedule
                            (list
                             timers
                             (length scheduled)
                             (length events)
                             (length traffic))))
                       (funcall
                        (car scheduled))
                       (list
                        before-drain
                        after-first-drain
                        after-reschedule
                        (nreverse events)
                        (nreverse traffic)
                        (nreverse logs)))))))
           (when-let*
               ((buffer
                 (get-buffer
                  "acp-client-stderr(agent)-1")))
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((1 1 0 0) (1 1 2 2) (2 2 2 2) ((notification #1=((method . "one"))) (request #2=((id . 2) (method . "two"))) (notification #1#)) ((incoming notification "one" #1#) (incoming request "two" #2#) (incoming notification "one" #1#)) (("INCOMING TEXT" "%s" ("one")) ("INCOMING TEXT" "%s" ("\ntwo\nbad\npartial")) ("INCOMING LINE" "%s" ("one")) ("INCOMING LINE" "%s" ("two")) ("INCOMING LINE" "%s" ("bad")) ("JSON PARSE ERROR" "Invalid JSON: %s" ("bad")) ("INCOMING TEXT" "%s" ("-tail\n")) ("INCOMING LINE" "%s" ("partial-tail")) ("JSON PARSE ERROR" "Invalid JSON: %s" ("partial-tail")) (nil "↳ Routing as notification" nil) ("NOTIFICATION HANDLER ERROR" "Failed with error: %S" ((error "notification fixture failure"))) (nil "↳ Routing as incoming request" nil) ("REQUEST HANDLER ERROR" "Failed with error: %S" ((error "request fixture failure"))) ("INCOMING TEXT" "%s" ("one\n")) ("INCOMING LINE" "%s" ("one")) (nil "↳ Routing as notification" nil) ("NOTIFICATION HANDLER ERROR" "Failed with error: %S" ((error "notification fixture failure")))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_start_client_stderr_hook_routes_api_and_generic_errors_and_sentinel_cleans_up() {
    let elisp_form = r##"(let ((client
                (acp-make-client
                 :command "agent"))
               process-spec
               handled
               logs
               failed)
         (map-put!
          client
          :error-handlers
          (list
           (lambda (error)
             (push error handled))))
         (cl-letf
             (((symbol-function
                'executable-find)
               (lambda (&rest arguments)
                 arguments
                 "/fixture/bin/agent"))
              ((symbol-function
                'acp--client-started-p)
               (lambda (value)
                 value
                 nil))
              ((symbol-function
                'make-process)
               (lambda (&rest arguments)
                 (setq process-spec
                       arguments)
                 'fixture-process))
              ((symbol-function
                'acp--parse-stderr-api-error)
               (lambda (raw-output)
                 (and
                  (equal raw-output
                         "api")
                  '((code . 7)
                    (message . "api error")))))
              ((symbol-function
                'acp--make-internal-error)
               (lambda (raw-output)
                 `((code . -32603)
                   (message . ,raw-output))))
              ((symbol-function
                'acp--log)
               (lambda (_client label format-string &rest arguments)
                 (push
                  (list
                   label
                   format-string
                   arguments)
                  logs)))
              ((symbol-function
                'process-status)
               (lambda (process)
                 process
                 'exit))
              ((symbol-function
                'acp--fail-pending-requests)
               (lambda (&rest arguments)
                 (setq failed
                       (list
                        (eq
                         (plist-get
                          arguments
                          :client)
                         client)
                        (plist-get
                         arguments
                         :event))))))
           (acp--start-client
            :client client)
           (let* ((stderr
                   (plist-get
                    process-spec
                    :stderr))
                  (sentinel
                   (plist-get
                    process-spec
                    :sentinel)))
             (with-current-buffer stderr
               (insert
                "api")
               (insert
                "  ")
               (insert
                "plain"))
             (funcall
              sentinel
              'fixture-process
              "finished\n")
             (list
              (nreverse handled)
              (nreverse logs)
              failed
              (buffer-live-p stderr)))))"##;
    let expect = expect![[
        r#"OK ((((code . 7) (message . "api error")) ((code . -32603) (message . "plain"))) (("STDERR" "%s" ("api")) ("API-ERROR" "%s" ("api")) ("STDERR" "%s" ("")) ("STDERR" "%s" ("plain")) ("API-ERROR" "%s" ("plain"))) (t "finished\n") nil)"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_request_sender_decorates_builds_ids_tracks_pending_and_sends_exact_json_lines() {
    let elisp_form = r##"(let (calls)
         (let ((client
                (acp-make-client
                 :command "agent"
                 :outgoing-request-decorator
                 (lambda (request)
                   (append
                    request
                    '((:params
                       (decorated . t))))))))
           (map-put!
            client
            :process
            'fixture-process)
           (cl-letf
               (((symbol-function
                  'acp--client-started-p)
                 (lambda (value)
                   value
                   t))
                ((symbol-function
                  'process-send-string)
                 (lambda (process string)
                   (push
                    (list
                     'send
                     process
                     string)
                    calls)))
                ((symbol-function
                  'acp--log)
                 (lambda (_client label format-string &rest arguments)
                   (push
                    (list
                     'log
                     label
                     format-string
                     arguments)
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
                      :object)
                     (map-elt
                      message
                      :json))
                    calls))))
             (let ((result
                    (acp--request-sender
                     :client client
                     :request
                     '((:method . "initialize"))
                     :buffer 'fixture-buffer
                     :on-success 'success
                     :on-failure 'failure)))
               (list
                result
                (map-elt
                 client
                 :request-id)
                (mapcar
                 (lambda (entry)
                   (let ((value
                          (cdr entry)))
                     (list
                      (car entry)
                      (map-elt value :request)
                      (map-elt value :buffer)
                      (map-elt value :on-success)
                      (map-elt value :on-failure))))
                 (map-elt
                  client
                  :pending-requests))
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (nil 1 ((1 #1=((jsonrpc . "2.0") (method . "initialize") (id . 1) (params (decorated . t))) fixture-buffer success failure)) ((log "OUTGOING OBJECT" "%s" (#1#)) (log "OUTGOING TEXT" "%s" ("{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"id\":1,\"params\":{\"decorated\":true}}\n")) (traffic outgoing request #1# "{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"id\":1,\"params\":{\"decorated\":true}}\n") (send fixture-process "{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"id\":1,\"params\":{\"decorated\":true}}\n")))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_request_sender_nil_decorator_logs_error_and_sends_original_request() {
    let elisp_form = r##"(let (calls)
         (let ((client
                (acp-make-client
                 :command "agent"
                 :outgoing-request-decorator
                 (lambda (request)
                   request
                   nil))))
           (map-put!
            client
            :process
            'fixture-process)
           (cl-letf
               (((symbol-function
                  'acp--client-started-p)
                 (lambda (value)
                   value
                   t))
                ((symbol-function
                  'process-send-string)
                 (lambda (process string)
                   (push
                    (list
                     'send
                     process
                     string)
                    calls)))
                ((symbol-function
                  'acp--log)
                 (lambda (_client label format-string &rest arguments)
                   (push
                    (list
                     'log
                     label
                     format-string
                     arguments)
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
             (acp--request-sender
              :client client
              :request
              '((:method . "plain")
                (:params
                 (x . 1))))
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((log "DECORATOR ERROR" "Outgoing request decorator returned nil for \"%s\", sending original request" ("plain")) (log "OUTGOING OBJECT" "%s" (#1=((jsonrpc . "2.0") (method . "plain") (id . 1) (params (x . 1))))) (log "OUTGOING TEXT" "%s" ("{\"jsonrpc\":\"2.0\",\"method\":\"plain\",\"id\":1,\"params\":{\"x\":1}}\n")) (traffic request ((:object . #1#) (:json . "{\"jsonrpc\":\"2.0\",\"method\":\"plain\",\"id\":1,\"params\":{\"x\":1}}\n"))) (send fixture-process "{\"jsonrpc\":\"2.0\",\"method\":\"plain\",\"id\":1,\"params\":{\"x\":1}}\n"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_request_sender_sync_returns_success_and_signals_failure_from_pending_callbacks() {
    let elisp_form = r##"(cl-labels
         ((scenario
           (failure)
           (let ((client
                  (acp-make-client
                   :command "agent"))
                 delivered)
             (map-put!
              client
              :process
              'fixture-process)
             (cl-letf
                 (((symbol-function
                    'acp--client-started-p)
                   (lambda (value)
                     value
                     t))
                  ((symbol-function
                    'process-send-string)
                   (lambda (&rest arguments)
                     arguments
                     nil))
                  ((symbol-function
                    'acp--log)
                   #'ignore)
                  ((symbol-function
                    'acp--log-traffic)
                   #'ignore)
                  ((symbol-function
                    'accept-process-output)
                   (lambda (&rest arguments)
                     arguments
                     (unless delivered
                       (setq delivered t)
                       (let ((pending
                              (cdar
                               (map-elt
                                client
                                :pending-requests))))
                         (funcall
                          (map-elt
                           pending
                           (if failure
                               :on-failure
                             :on-success))
                          (if failure
                              '((code . -1))
                            '((ok . t))))))
                     t)))
               (condition-case error
                   (list
                    'value
                    (acp--request-sender
                     :client client
                     :request
                     '((:method . "sync"))
                     :sync t))
                 (error
                  (list
                   'signal
                   (car error)
                   (cadr error))))))))
         (list
          (scenario nil)
          (scenario t)))"##;
    let expect =
        expect![[r#"OK ((value ((ok . t))) (signal error "ACP request failed: ((code . -1))"))"#]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_response_sender_serializes_alist_nil_success_and_error_envelopes() {
    let elisp_form = r##"(let (calls)
         (let ((client
                '((:process
                   .
                   fixture-process))))
           (cl-letf
               (((symbol-function
                  'process-send-string)
                 (lambda (process string)
                   (push
                    (list
                     'send
                     process
                     string)
                    calls)
                   nil))
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
                      :object)
                     (map-elt
                      message
                      :json))
                    calls)
                   nil)))
             (list
              (acp--response-sender
               :client client
               :response
                 '((:request-id . 4)
                   (:result
                    (value . nil))))
              (acp--response-sender
               :client client
               :response
               '((:request-id . 5)
                 (:result . nil)))
              (acp--response-sender
               :client client
               :response
               '((:request-id . 6)
                 (:error
                  (code . -1)
                  (message . "bad"))))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (nil nil nil ((traffic outgoing response ((jsonrpc . "2.0") (id . 4) (result (value))) "{\"jsonrpc\":\"2.0\",\"id\":4,\"result\":{\"value\":{}}}\n") (send fixture-process "{\"jsonrpc\":\"2.0\",\"id\":4,\"result\":{\"value\":{}}}\n") (traffic outgoing response ((jsonrpc . "2.0") (id . 5) (result)) "{\"jsonrpc\":\"2.0\",\"id\":5,\"result\":{}}\n") (send fixture-process "{\"jsonrpc\":\"2.0\",\"id\":5,\"result\":{}}\n") (traffic outgoing response ((jsonrpc . "2.0") (id . 6) (error (code . -1) (message . "bad"))) "{\"jsonrpc\":\"2.0\",\"id\":6,\"error\":{\"code\":-1,\"message\":\"bad\"}}\n") (send fixture-process "{\"jsonrpc\":\"2.0\",\"id\":6,\"error\":{\"code\":-1,\"message\":\"bad\"}}\n")))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_notification_sender_serializes_params_and_paramless_notifications_exactly() {
    let elisp_form = r##"(let (calls)
         (let ((client
                (acp-make-client
                 :command "agent")))
           (map-put!
            client
            :process
            'fixture-process)
           (cl-letf
               (((symbol-function
                  'acp--client-started-p)
                 (lambda (value)
                   value
                   t))
                ((symbol-function
                  'process-send-string)
                 (lambda (process string)
                   (push
                    (list
                     'send
                     process
                     string)
                    calls)))
                ((symbol-function
                  'acp--log)
                 (lambda (_client label format-string &rest arguments)
                   (push
                    (list
                     'log
                     label
                     format-string
                     arguments)
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
                      :object)
                     (map-elt
                      message
                      :json))
                    calls))))
             (list
              (acp--notification-sender
               :client client
               :notification
               '((:method . "session/update")
                 (:params
                  (id . "s"))))
              (acp--notification-sender
               :client client
               :notification
               '((:method . "tick")))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (nil nil ((log "OUTGOING OBJECT" "%s" (#1=((jsonrpc . "2.0") (method . "session/update") (params (id . "s"))))) (log "OUTGOING TEXT" "%s" ("{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"id\":\"s\"}}\n")) (traffic outgoing notification #1# "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"id\":\"s\"}}\n") (send fixture-process "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"id\":\"s\"}}\n") (log "OUTGOING OBJECT" "%s" (#2=((jsonrpc . "2.0") (method . "tick")))) (log "OUTGOING TEXT" "%s" ("{\"jsonrpc\":\"2.0\",\"method\":\"tick\"}\n")) (traffic outgoing notification #2# "{\"jsonrpc\":\"2.0\",\"method\":\"tick\"}\n") (send fixture-process "{\"jsonrpc\":\"2.0\",\"method\":\"tick\"}\n")))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_notification_sender_sync_enters_process_wait_loop_without_a_completion_path() {
    let elisp_form = r##"(let ((client
                (acp-make-client
                 :command "agent"))
               calls)
         (map-put!
          client
          :process
          'fixture-process)
         (cl-letf
             (((symbol-function
                'acp--client-started-p)
               (lambda (value)
                 value
                 t))
              ((symbol-function
                'process-send-string)
               (lambda (process string)
                 (push
                  (list
                   'send
                   process
                   string)
                  calls)))
              ((symbol-function
                'acp--log)
               #'ignore)
              ((symbol-function
                'acp--log-traffic)
               #'ignore)
              ((symbol-function
                'accept-process-output)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'accept
                   arguments)
                  calls)
                 (error
                  "bounded sync notification probe"))))
           (condition-case error
               (list
                'value
                (acp--notification-sender
                 :client client
                 :notification
                 '((:method . "sync-note"))
                 :sync t)
                (nreverse calls))
             (error
              (list
               'signal
               (car error)
               (cadr error)
               (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (signal error "bounded sync notification probe" ((send fixture-process "{\"jsonrpc\":\"2.0\",\"method\":\"sync-note\"}\n") (accept fixture-process 0.01)))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_internal_sender_validation_errors_match_for_requests_responses_and_notifications() {
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
               (acp--request-sender
                :request 'x))
             (lambda ()
               (acp--request-sender
                :client client))
             (lambda ()
               (acp--response-sender
                :response 'x))
             (lambda ()
               (acp--response-sender
                :client client))
             (lambda ()
               (acp--notification-sender
                :notification 'x))
             (lambda ()
               (acp--notification-sender
                :client client))))))"##;
    let expect = expect![[
        r#"OK ((error ":client is required") (error ":request is required") (error ":client is required") (error ":response is required") (error ":client is required") (error ":notification is required"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_upstream_sync_request_fails_promptly_when_agent_exits_after_read() {
    let elisp_form = r##"(let ((client
                (acp-make-client
                 :command "sh"
                 :command-params
                 '("-c"
                   "IFS= read -r _; exit 42"))))
         (unwind-protect
             (condition-case error
                 (list
                  'value
                  (acp-send-request
                   :client client
                   :request
                   '((:method . "initialize"))
                   :sync t))
               (error
                (list
                 'signal
                 (car error)
                 (cadr error))))
           (when-let*
               ((process
                 (map-elt
                  client
                  :process))
                ((process-live-p process)))
             (delete-process process))))"##;
    let expect = expect![[
        r#"OK (signal error "ACP request failed: ((code . -32603) (message . Agent process ended before completing request: exited abnormally with code 42))")"#
    ]];
    assert_acp_parity(elisp_form, expect);
}
