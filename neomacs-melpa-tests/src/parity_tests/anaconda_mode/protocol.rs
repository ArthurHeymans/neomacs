use expect_test::expect;

use super::{assert_anaconda_mode_parity, assert_anaconda_mode_signal_parity};

#[test]
fn rpc_policy_allows_local_and_remote_calls_exactly_as_configured() {
    let elisp_form = r##"(let (events
      remote)
  (cl-letf (((symbol-function 'pythonic-remote-p) (lambda () remote))
            ((symbol-function 'anaconda-mode-start)
             (lambda (callback)
               (push 'start events)
               (funcall callback)))
            ((symbol-function 'anaconda-mode-jsonrpc)
             (lambda (command callback)
               (push (list 'jsonrpc command callback) events))))
    (mapcar
     (lambda (case)
       (let ((anaconda-mode-disable-rpc (car case)))
         (setq remote (cadr case)
               events nil)
         (anaconda-mode-call "infer" 'test-callback)
         (list case (nreverse events))))
     '((never nil) (never t)
       (remote nil) (remote t)
       (always nil) (always t)))))"##;
    let expect = expect![[
        r#"OK (((never nil) (start (jsonrpc "infer" test-callback))) ((never t) (start (jsonrpc "infer" test-callback))) ((remote nil) (start (jsonrpc "infer" test-callback))) ((remote t) nil) ((always nil) nil) ((always t) nil))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn synchronous_rpc_delivers_the_complete_result_to_the_user_callback() {
    let elisp_form = r##"(let (events)
  (cl-letf (((symbol-function 'anaconda-mode-call)
             (lambda (command callback)
               (push (list 'request command) events)
               (funcall callback
                        [("alpha" . 1)
                         ("beta" . [2 3])]))))
    (let ((value
           (anaconda-mode-call-sync
            "complete"
            (lambda (result)
              (push (list 'callback result) events)
              (list 'received
                    (length result)
                    (aref result 1))))))
      (list value (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ((received 2 #1=("beta" . [2 3])) ((request "complete") (callback [("alpha" . 1) #1#])))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn synchronous_rpc_timeout_has_a_command_specific_error_and_polls_the_process_loop() {
    let elisp_form = r##"(let ((anaconda-mode-sync-request-timeout -1)
      polls)
  (cl-letf (((symbol-function 'anaconda-mode-call)
             (lambda (command _callback)
               (push (list 'started command) polls)))
            ((symbol-function 'accept-process-output)
             (lambda (process seconds)
               (push (list 'poll process seconds) polls)
               nil)))
    (anaconda-mode-call-sync "get_references" #'identity)))"##;
    let expect = expect![[r#"ERR (error "get_references request timed out")"#]];
    assert_anaconda_mode_signal_parity(elisp_form, expect);
}

#[test]
fn request_data_captures_real_source_cursor_coordinates_and_python_readable_path() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/project/demo.py")
  (insert "class Café:\n"
          "    def total(self, tax):\n"
          "        return self.value + tax\n")
  (goto-char (point-min))
  (forward-line 2)
  (search-forward "self")
  (cl-letf (((symbol-function 'pythonic-python-readable-file-name)
             (lambda (path) (concat "python://" path))))
    (anaconda-mode-jsonrpc-request-data "infer")))"##;
    let expect = expect![[
        r#"OK ((jsonrpc . "2.0") (id . 1) (method . "infer") (params (source . "class Café:\n    def total(self, tax):\n        return self.value + tax\n") (line . 3) (column . 19) (path . "python:///workspace/project/demo.py")))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn encoded_jsonrpc_request_round_trips_unicode_source_and_null_unsaved_path() {
    let elisp_form = r##"(with-temp-buffer
  (insert "message = \"λ → café\"\nprint(message)\n")
  (goto-char (point-max))
  (let* ((encoded (anaconda-mode-jsonrpc-request "show_doc"))
         (json-object-type 'alist)
         (json-array-type 'vector)
         (json-key-type 'symbol)
         (decoded (json-read-from-string encoded)))
    (list
     (multibyte-string-p encoded)
     (secure-hash 'sha256 encoded)
     (length encoded)
     decoded)))"##;
    let expect = expect![[
        r#"OK (nil "61ae2d8043fe3d54a192eb5f6dee612fe36db679a8992c00d3e9bee3503a8e57" 143 ((jsonrpc . "2.0") (id . 1) (method . "show_doc") (params (source . "message = \"���� ������ caf����\"\nprint(message)\n") (line . 3) (column . 0) (path))))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn jsonrpc_posts_the_exact_request_to_the_bound_local_endpoint() {
    let elisp_form = r##"(with-temp-buffer
  (insert "target.call(argument)\n")
  (goto-char 8)
  (let (retrieval callback-seen)
    (cl-letf (((symbol-function 'anaconda-mode-port) (lambda () 43117))
              ((symbol-function 'anaconda-mode-create-response-handler)
               (lambda (callback)
                 (setq callback-seen callback)
                 'response-handler))
              ((symbol-function 'url-retrieve)
               (lambda (url handler callback-args silent)
                 (setq retrieval
                       (list url handler callback-args silent
                             url-request-method
                             (let ((json-object-type 'alist)
                                   (json-array-type 'vector)
                                   (json-key-type 'symbol))
                               (json-read-from-string url-request-data)))))))
      (anaconda-mode-jsonrpc "complete" 'completion-callback)
      (list callback-seen retrieval))))"##;
    let expect = expect![[
        r#"OK (completion-callback ("http://127.0.0.1:43117" response-handler nil t "POST" ((jsonrpc . "2.0") (id . 1) (method . "complete") (params (source . "target.call(argument)\n") (line . 1) (column . 7) (path)))))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn response_handler_applies_each_json_result_item_in_the_original_request_buffer() {
    let elisp_form = r##"(save-window-excursion
  (let ((request-buffer (generate-new-buffer " *anaconda-request*"))
        (http-buffer (generate-new-buffer " *anaconda-http*"))
        callback-observation)
    (unwind-protect
        (progn
          (switch-to-buffer request-buffer)
          (insert "value = service.lookup()\n")
          (goto-char 10)
          (let ((handler
                 (anaconda-mode-create-response-handler
                  (lambda (&rest values)
                    (setq callback-observation
                          (list
                           (buffer-name)
                           (point)
                           values))))))
            (with-current-buffer http-buffer
              (insert "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n"
                      "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":[\"first\",{\"kind\":\"definition\",\"line\":8}]}")
              (goto-char (point-min))
              (funcall handler '(:peer "local")))
            (list callback-observation
                  (buffer-live-p http-buffer)
                  (buffer-string))))
      (when (buffer-live-p request-buffer) (kill-buffer request-buffer))
      (when (buffer-live-p http-buffer) (kill-buffer http-buffer)))))"##;
    let expect = expect![[
        r#"OK ((" *anaconda-request*" 10 (["first" ((kind . "definition") (line . 8))])) nil "value = service.lookup()\n")"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn malformed_http_response_is_preserved_for_diagnosis_and_never_calls_the_callback() {
    let elisp_form = r##"(save-window-excursion
  (let ((request-buffer (generate-new-buffer " *anaconda-request*"))
        (http-buffer (generate-new-buffer " *anaconda-http*"))
        (anaconda-mode-response-buffer " *anaconda-bad-response*")
        callback-called
        diagnostic)
    (unwind-protect
        (progn
          (switch-to-buffer request-buffer)
          (insert "broken_call()\n")
          (goto-char 4)
          (let ((handler
                 (anaconda-mode-create-response-handler
                  (lambda (&rest _values) (setq callback-called t)))))
            (with-current-buffer http-buffer
              (insert "HTTP/1.1 502 Bad Gateway\r\nX-Test: yes\r\n\r\n{\"result\":")
              (goto-char (point-min))
              (funcall handler '(:error "connection closed")))
            (setq diagnostic
                  (with-current-buffer anaconda-mode-response-buffer
                    (buffer-string)))
            (list
             callback-called
             (buffer-live-p http-buffer)
             diagnostic)))
      (when (buffer-live-p request-buffer) (kill-buffer request-buffer))
      (when (buffer-live-p http-buffer) (kill-buffer http-buffer))
      (when (get-buffer anaconda-mode-response-buffer)
        (kill-buffer anaconda-mode-response-buffer)))))"##;
    let expect = expect![[
        r##"OK (nil nil "# status: (:error connection closed)\n# point: 52\nHTTP/1.1 502 Bad Gateway\15\nX-Test: yes\15\n\15\n{\"result\":")"##
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn jsonrpc_error_response_reports_server_details_without_invoking_the_success_callback() {
    let elisp_form = r##"(save-window-excursion
  (let ((request-buffer (generate-new-buffer " *anaconda-request*"))
        (http-buffer (generate-new-buffer " *anaconda-http*"))
        (anaconda-mode-process-buffer "*test-anaconda-process*")
        callback-called
        messages)
    (unwind-protect
        (progn
          (switch-to-buffer request-buffer)
          (insert "unknown.symbol\n")
          (let ((handler
                 (anaconda-mode-create-response-handler
                  (lambda (&rest _values) (setq callback-called t)))))
            (cl-letf (((symbol-function 'message)
                       (lambda (format-string &rest arguments)
                         (let ((text (apply #'format format-string arguments)))
                           (push text messages)
                           text))))
              (with-current-buffer http-buffer
                (insert "HTTP/1.1 200 OK\r\n\r\n"
                        "{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"message\":\"NameError\",\"data\":\"unknown symbol\"}}")
                (goto-char (point-min))
                (funcall handler nil)))
            (list callback-called
                  (nreverse messages)
                  (buffer-live-p http-buffer))))
      (when (buffer-live-p request-buffer) (kill-buffer request-buffer))
      (when (buffer-live-p http-buffer) (kill-buffer http-buffer)))))"##;
    let expect = expect![[
        r#"OK (nil ("NameError: unknown symbol - see *test-anaconda-process* for more information.") nil)"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn stale_response_is_discarded_after_the_user_moves_point() {
    let elisp_form = r##"(save-window-excursion
  (let ((request-buffer (generate-new-buffer " *anaconda-request*"))
        (http-buffer (generate-new-buffer " *anaconda-http*"))
        callback-called)
    (unwind-protect
        (progn
          (switch-to-buffer request-buffer)
          (insert "alpha = beta\n")
          (goto-char 2)
          (let ((handler
                 (anaconda-mode-create-response-handler
                  (lambda (&rest _values) (setq callback-called t)))))
            (goto-char 8)
            (with-current-buffer http-buffer
              (insert "HTTP/1.1 200 OK\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":[42]}")
              (goto-char (point-min))
              (funcall handler nil))
            (list callback-called
                  (point)
                  (buffer-live-p http-buffer))))
      (when (buffer-live-p request-buffer) (kill-buffer request-buffer))
      (when (buffer-live-p http-buffer) (kill-buffer http-buffer)))))"##;
    let expect = expect!["OK (nil 8 nil)"];
    assert_anaconda_mode_parity(elisp_form, expect);
}
