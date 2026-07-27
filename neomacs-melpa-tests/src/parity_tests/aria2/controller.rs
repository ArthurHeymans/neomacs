use expect_test::expect;

use super::assert_aria2_parity;

#[test]
fn aria2_controller_request_ids_increment_independently_and_wrap_at_fixnum_boundary() {
    let elisp_form = r##"(let ((first
                (aria2-test-controller))
               (second
                (aria2-test-controller
                 (- most-positive-fixnum 2))))
         (list
          (mapcar
           (lambda (_)
             (list
              (get-next-id first)
              (oref first request-id)))
           '(a b c d))
          (mapcar
           (lambda (_)
             (list
              (get-next-id second)
              (oref second request-id)))
           '(a b c))
          (oref first request-id)
          (oref second request-id)))"##;
    let expect = expect![
        "OK (((1 1) (2 2) (3 3) (4 4)) ((2305843009213693950 2305843009213693950) (2305843009213693951 0) (1 1)) 4 1)"
    ];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_controller_process_status_checks_positive_pid_owner_command_and_debug_message() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller
                 0
                 417))
               messages)
         (cl-letf
             (((symbol-function
                'aria2--is-aria-process-p)
               (lambda (pid)
                 (eq pid 417)))
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
            (let ((aria2--debug nil))
              (is-process-running
               controller))
            (let ((aria2--debug t))
              (is-process-running
               controller))
            (progn
              (oset controller pid 0)
              (is-process-running
               controller))
            (progn
              (oset controller pid -9)
              (is-process-running
               controller))
            (nreverse messages)
            (oref controller pid))))"##;
    let expect = expect![[r#"OK (t t nil nil ("aria2 pid 417") -9)"#]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_controller_run_process_builds_complete_options_discovers_daemon_and_preserves_order() {
    let elisp_form = r##"(let* ((controller
                  (aria2-test-controller))
                 (session
                  (aria2-test-path
                   "resume.session"))
                 (aria2-executable
                  "/opt/aria2/bin/aria2c")
                 (aria2-custom-args
                  '("--continue=true"
                    "--max-connection-per-server=8"))
                 (aria2-rcp-listen-port
                  7711)
                 (aria2-download-directory
                  "/downloads/fixture")
                 (aria2-session-file
                  session)
                 calls)
         (with-temp-file
             session
           (insert
            "gid=fixture\n"))
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'start-process)
                   (lambda (&rest arguments)
                     (push
                      (cons :start arguments)
                      calls)
                     :started))
                  ((symbol-function
                    'sleep-for)
                   (lambda (&rest arguments)
                     (push
                      (cons :sleep arguments)
                      calls)))
                  ((symbol-function
                    'list-system-processes)
                   (lambda ()
                     (push
                      (list :list-system-processes)
                      calls)
                     '(101 202 303)))
                  ((symbol-function
                    'aria2--is-aria-process-p)
                   (lambda (pid)
                     (push
                      (list :process-p pid)
                      calls)
                     (eq pid 202)))
                  ((symbol-function
                    'is-process-running)
                   (lambda (this)
                     (and
                      (eq this controller)
                      (eq
                       (oref this pid)
                       202))))
                  ((symbol-function
                    'message)
                   (lambda (&rest arguments)
                     (push
                      (cons :message arguments)
                      calls))))
               (let ((aria2--debug t))
                 (run-process
                  controller))
               (list
                (oref controller pid)
                (nreverse calls)))
           (delete-file
            session)))"##;
    let expect = expect![[
        r#"OK (202 ((:message "Starting process: %s %s" "/opt/aria2/bin/aria2c" "--continue=true --max-connection-per-server=8 -D --enable-rpc=true --rpc-secret=fixture-secret --rpc-listen-port=7711 --dir=/downloads/fixture --save-session=[ORACLE-SANDBOX]/resume.session --input-file=[ORACLE-SANDBOX]/resume.session") (:start "aria2c" nil "/opt/aria2/bin/aria2c" "--continue=true" "--max-connection-per-server=8" "-D" "--enable-rpc=true" "--rpc-secret=fixture-secret" "--rpc-listen-port=7711" "--dir=/downloads/fixture" "--save-session=[ORACLE-SANDBOX]/resume.session" "--input-file=[ORACLE-SANDBOX]/resume.session") (:sleep 1) (:list-system-processes) (:process-p 101) (:process-p 202) (:process-p 303)))"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_controller_run_process_is_noop_when_daemon_is_already_running() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller
                 0
                 700))
               calls)
         (cl-letf
             (((symbol-function
                'is-process-running)
               (lambda (this)
                 (push
                  (list :running this)
                  calls)
                 t))
              ((symbol-function
                'start-process)
               (lambda (&rest arguments)
                 (push
                  (cons :unexpected-start arguments)
                  calls)))
              ((symbol-function
                'sleep-for)
               (lambda (&rest arguments)
                 (push
                  (cons :unexpected-sleep arguments)
                  calls))))
           (list
            (run-process controller)
            (oref controller pid)
            (mapcar
             (lambda (call)
               (if
                   (and
                    (consp call)
                    (eq
                     (car call)
                     :running))
                   (list
                    :running
                    (eq
                     (cadr call)
                     controller))
                 call))
             (nreverse calls)))))"##;
    let expect = expect!["OK (nil 700 ((:running t)))"];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_controller_run_process_failure_signals_exact_command_and_leaves_pid_stopped() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller))
               (aria2-executable
                "/fixture/aria2c")
               (aria2-custom-args
                '("--seed-time=0"))
               (aria2-rcp-listen-port
                6801)
               (aria2-download-directory
                "/fixture/downloads")
               (aria2-session-file
                (aria2-test-path
                 "absent.session"))
               calls)
         (cl-letf
             (((symbol-function
                'is-process-running)
               (lambda (_)
                 nil))
              ((symbol-function
                'start-process)
               (lambda (&rest arguments)
                 (push arguments calls)
                 :started))
              ((symbol-function
                'sleep-for)
               (lambda (&rest _)))
              ((symbol-function
                'list-system-processes)
               (lambda ()
                 '(41 42)))
              ((symbol-function
                'aria2--is-aria-process-p)
               (lambda (_)
                 nil)))
           (list
            (condition-case error-data
                (list
                 :ok
                 (run-process controller))
              (error
               (list
                :error
                (car error-data)
                (cdr error-data)
                (error-message-string
                 error-data))))
            calls
            (oref controller pid))))"##;
    let expect = expect![[
        r#"OK ((:error aria2-err-failed-to-start "/fixture/aria2c --seed-time=0 -D --enable-rpc=true --rpc-secret=fixture-secret --rpc-listen-port=6801 --dir=/fixture/downloads --save-session=[ORACLE-SANDBOX]/absent.session" "Failed to start") (("aria2c" nil "/fixture/aria2c" "--seed-time=0" "-D" "--enable-rpc=true" "--rpc-secret=fixture-secret" "--rpc-listen-port=6801" "--dir=/fixture/downloads" "--save-session=[ORACLE-SANDBOX]/absent.session")) -1)"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_json_rpc_request_serializes_tokens_ids_params_headers_and_decodes_results() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller
                 40))
               captures)
         (cl-letf
             (((symbol-function
                'url-retrieve-synchronously)
               (lambda (url silent)
                 (push
                  (list
                   url
                   silent
                   url-request-method
                   url-request-data
                   url-request-extra-headers)
                  captures)
                 (let ((buffer
                        (generate-new-buffer
                         " *aria2-rpc-success*")))
                   (with-current-buffer buffer
                     (insert
                      "HTTP/1.1 200 OK\n"
                      "Content-Type: application/json\n"
                      "\n"
                      "{\"jsonrpc\":\"2.0\",\"id\":41,\"result\":{\"gid\":\"abc\",\"items\":[1,2]}}"))
                   buffer))))
           (let ((result
                  (make-request
                   controller
                   "aria2.fixture"
                   "gid-1"
                   nil
                   '((dir . "/downloads"))
                   [1 2])))
             (list
              result
              (oref controller request-id)
              (mapcar
               (lambda (capture)
                 (list
                  (nth 0 capture)
                  (nth 1 capture)
                  (nth 2 capture)
                  (json-read-from-string
                   (nth 3 capture))
                  (nth 4 capture)))
               (nreverse captures))
              (get-buffer
               " *aria2-rpc-success*")))))"##;
    let expect = expect![[
        r#"OK (((gid . "abc") (items . [1 2])) 41 (("http://fixture.invalid:6800/jsonrpc" t "POST" ((jsonrpc . 2.0) (id . 41) (method . "aria2.fixture") (params . ["token:fixture-secret" "gid-1" ((dir . "/downloads")) [1 2]])) (("Content-Type" . "application/json")))) nil)"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}

#[test]
fn aria2_json_rpc_request_starts_optional_server_reports_debug_and_signals_remote_errors() {
    let elisp_form = r##"(let ((controller
                (aria2-test-controller
                 7))
               (aria2-start-rpc-server
                t)
               (aria2--debug
                t)
               calls)
         (cl-letf
             (((symbol-function
                'run-process)
               (lambda (this)
                 (push
                  (list :run
                        (eq this controller))
                  calls)))
              ((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (push
                  (list
                   :message
                   (apply
                    #'format
                    format-string
                    arguments))
                  calls)))
              ((symbol-function
                'url-retrieve-synchronously)
               (lambda (&rest _)
                 (let ((buffer
                        (generate-new-buffer
                         " *aria2-rpc-error*")))
                   (with-current-buffer buffer
                     (insert
                      "headers\n"
                      "{\"jsonrpc\":\"2.0\",\"id\":8,\"error\":{\"code\":-32602,\"message\":\"bad parameters λ\"}}"))
                   buffer))))
           (list
            (condition-case error-data
                (list
                 :ok
                 (make-request
                  controller
                  "aria2.invalid"
                  nil))
              (error
               (list
                :error
                (car error-data)
                (cdr error-data)
                (error-message-string
                 error-data))))
            (oref controller request-id)
            (nreverse calls)
            (get-buffer
             " *aria2-rpc-error*"))))"##;
    let expect = expect![[
        r#"OK ((:error error ("ERROR: bad parameters λ") "ERROR: bad parameters λ") 8 ((:run t) (:message "SEND: {\"jsonrpc\":2.0,\"id\":8,\"method\":\"aria2.invalid\",\"params\":[\"token:fixture-secret\"]}") (:message "RECV: ((jsonrpc . 2.0) (id . 8) (error (code . -32602) (message . bad parameters λ)))")) nil)"#
    ]];

    assert_aria2_parity(elisp_form, expect);
}
