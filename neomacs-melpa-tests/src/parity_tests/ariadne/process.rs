use expect_test::expect;

use super::assert_ariadne_parity;

#[test]
fn connect_constructs_exact_local_network_process_and_binary_buffer() {
    let elisp_form = r##"(let ((buffer
                (get-buffer-create
                 " *ariadne-connect-contract*"))
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function 'make-network-process)
                   (lambda (&rest args)
                     (push args calls)
                     'socket))
                  ((symbol-function 'process-buffer)
                   (lambda (process)
                     (push (list :process-buffer process)
                           calls)
                     buffer))
                  ((symbol-function
                    'set-process-query-on-exit-flag)
                   (lambda (process flag)
                     (push (list :query process flag)
                           calls))))
               (with-current-buffer buffer
                 (set-buffer-multibyte t))
               (list (ariadne-connect)
                     ariadne-process
                     (with-current-buffer buffer
                       enable-multibyte-characters)
                     (nreverse calls)))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (socket socket nil ((:name "ariadne" :host "localhost" :service 39014 :buffer "*ariadne*" :filter ariadne-filter :sentinel ariadne-sentinel) (:process-buffer socket) (:query socket nil)))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn failed_connect_reports_stable_diagnostic_and_leaves_process_nil() {
    let elisp_form = r##"(let ((ariadne-process nil)
               messages)
         (cl-letf
             (((symbol-function 'make-network-process)
               (lambda (&rest _)
                 (error "connection refused")))
              ((symbol-function 'message)
               (lambda (format-string &rest args)
                 (let ((text
                        (apply #'format
                               format-string args)))
                   (push text messages)
                   text))))
           (list (ariadne-connect)
                 ariadne-process
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("Failed to connect to Ariadne.  Is ariadne-server running?" nil ("Failed to connect to Ariadne.  Is ariadne-server running?"))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn close_clears_global_connection_and_kills_its_process_buffer() {
    let elisp_form = r##"(let* ((buffer
                 (get-buffer-create
                  " *ariadne-close-contract*"))
                (ariadne-process 'socket))
         (cl-letf (((symbol-function 'process-buffer)
                    (lambda (process)
                      (list process)
                      buffer)))
           (list (ariadne-close 'socket)
                 ariadne-process
                 (buffer-live-p buffer))))"##;
    let expect = expect!["OK (t nil nil)"];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn filter_appends_binary_chunks_then_processes_every_available_frame() {
    let elisp_form = r##"(let ((buffer
                (get-buffer-create
                 " *ariadne-filter-contract*"))
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function 'process-buffer)
                   (lambda (_process) buffer))
                  ((symbol-function
                    'ariadne-process-available-output)
                   (lambda (process)
                     (push (list :process process
                                 :content
                                 (with-current-buffer buffer
                                   (buffer-string)))
                           calls))))
               (with-current-buffer buffer
                 (set-buffer-multibyte nil)
                 (erase-buffer)
                 (insert "prefix"))
               (ariadne-filter 'socket
                               (string 0 1 255))
               (ariadne-filter 'socket "tail")
               (list
                (with-current-buffer buffer
                  (string-to-list (buffer-string)))
                (nreverse calls)))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((112 114 101 102 105 120 0 1 255 116 97 105 108) ((:process socket :content "prefix\0\1��") (:process socket :content "prefix\0\1��tail")))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn available_output_dispatches_complete_messages_until_buffer_is_partial() {
    let elisp_form = r##"(let ((buffer
                (get-buffer-create
                 " *ariadne-output-contract*"))
               (states '(t t nil))
               (events
                (list (vector 'reply (vector 'no_name))
                      (vector 'reply
                              (vector 'loc_unknown
                                      "External.Module"))))
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function 'process-buffer)
                   (lambda (_process) buffer))
                  ((symbol-function 'ariadne-have-input-p)
                   (lambda ()
                     (prog1 (car states)
                       (setq states (cdr states)))))
                  ((symbol-function 'ariadne-read-or-lose)
                   (lambda (process)
                     (push (list :read process) calls)
                     (prog1 (car events)
                       (setq events (cdr events)))))
                  ((symbol-function 'ariadne-dispatch-event)
                   (lambda (event process)
                     (push (list :dispatch event process)
                           calls))))
               (ariadne-process-available-output
                'socket)
               (nreverse calls))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((:read socket) (:dispatch [reply [no_name]] socket) (:read socket) (:dispatch [reply [loc_unknown "External.Module"]] socket))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn dispatch_failure_reschedules_remaining_output_at_idle_and_preserves_error() {
    let elisp_form = r##"(let ((buffer
                (get-buffer-create
                 " *ariadne-reschedule-contract*"))
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function 'process-buffer)
                   (lambda (_process) buffer))
                  ((symbol-function 'ariadne-have-input-p)
                   (lambda () t))
                  ((symbol-function 'ariadne-read-or-lose)
                   (lambda (_process)
                     (vector 'reply (vector 'no_name))))
                  ((symbol-function 'ariadne-dispatch-event)
                   (lambda (&rest _)
                     (error "handler failed")))
                  ((symbol-function 'ariadne-run-when-idle)
                   (lambda (function &rest args)
                     (push (cons function args) calls)
                     'timer)))
               (let ((outcome
                      (condition-case error
                          (list
                           :ok
                           (ariadne-process-available-output
                            'socket))
                        (error (list :error error)))))
                 (list outcome (nreverse calls))))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((:error (error "handler failed")) ((ariadne-process-available-output socket)))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn sentinel_reports_reason_then_closes_the_exact_connection() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (list :message
                                  (apply #'format
                                         format-string args))
                            calls)))
                   ((symbol-function 'ariadne-close)
                    (lambda (process)
                      (push (list :close process) calls))))
           (list (ariadne-sentinel
                  'socket "connection broken by remote peer\n")
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (#1=((:close socket)) ((:message "Ariadne connection closed unexpectedly: connection broken by remote peer\n") . #1#))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn idle_wrapper_forwards_function_and_all_arguments_to_zero_delay_timer() {
    let elisp_form = r##"(let (call)
         (cl-letf (((symbol-function 'run-at-time)
                    (lambda (time repeat function &rest args)
                      (setq call
                            (list time repeat function args))
                      'timer-object)))
           (list
            (ariadne-run-when-idle
             #'list 'alpha 42 "payload")
            call)))"##;
    let expect = expect![[r#"OK (timer-object (0 nil list (alpha 42 "payload")))"#]];
    assert_ariadne_parity(elisp_form, expect);
}
