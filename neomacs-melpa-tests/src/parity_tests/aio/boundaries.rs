use expect_test::expect;

use super::assert_aio_parity;

#[test]
fn aio_real_process_filter_and_sentinel_form_a_multistep_callback_session() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (script (expand-file-name "bin/events" sandbox))
                          (filter-stream (aio-make-callback :tag :filter))
                          (sentinel-stream
                           (aio-make-callback :tag :sentinel :once t)))
                      (make-directory (file-name-directory script) t)
                      (with-temp-file script
                        (insert
                         "#!/bin/sh\n"
                         "printf 'alpha\\n'\n"
                         "sleep 0.02\n"
                         "printf 'beta\\n'\n"))
                      (set-file-modes script #o755)
                      (let ((process
                             (make-process
                              :name "aio-parity-process"
                              :command (list script)
                              :connection-type 'pipe
                              :noquery t
                              :filter (car filter-stream)
                              :sentinel (car sentinel-stream))))
                        (unwind-protect
                            (let ((filter-promise (cdr filter-stream)))
                              (list
                               (cdr
                                (aio-wait-for filter-promise))
                               (cdr
                                (aio-wait-for
                                 (car
                                  (aio-wait-for
                                   filter-promise))))
                               (aio-wait-for
                                (cdr sentinel-stream))
                               (process-status process)))
                          (when (process-live-p process)
                            (delete-process process)))))"##;
    let expect = expect![[
        r#"OK ((:filter (:process #1="aio-parity-process" exit) "alpha\n") (:filter (:process #1# exit) "beta\n") (:sentinel (:process #1# exit) "finished\n") exit)"#
    ]];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_url_retrieve_success_clones_callback_buffer_without_external_network() {
    let elisp_form = r##"(let (arguments callback-buffer)
                      (cl-letf
                          (((symbol-function 'url-retrieve)
                            (lambda (url callback silent inhibit-cookies)
                              (setq arguments
                                    (list url silent inhibit-cookies))
                              (with-temp-buffer
                                (insert
                                 "HTTP/1.1 200 OK\n\n"
                                 "deterministic body")
                                (setq callback-buffer
                                      (current-buffer))
                                (funcall
                                 callback
                                 '(:redirect nil :peer
                                   "fixture"))))))
                        (let* ((result
                                (aio-wait-for
                                 (aio-url-retrieve
                                  "https://example.invalid/data"
                                  t t)))
                               (status (car result))
                               (buffer (cdr result)))
                          (unwind-protect
                              (list
                               arguments
                               status
                               (buffer-live-p callback-buffer)
                               (buffer-live-p buffer)
                               (with-current-buffer buffer
                                 (buffer-string))
                               (eq callback-buffer buffer))
                            (when (buffer-live-p buffer)
                              (kill-buffer buffer))))))"##;
    let expect = expect![[
        r#"OK (("https://example.invalid/data" t t) (:redirect nil :peer "fixture") nil t "HTTP/1.1 200 OK\n\ndeterministic body" nil)"#
    ]];
    assert_aio_parity(elisp_form, expect);
}

#[test]
fn aio_url_retrieve_synchronous_boundary_error_is_delivered_by_promise() {
    let elisp_form = r##"(cl-letf
                      (((symbol-function 'url-retrieve)
                        (lambda (&rest _)
                          (signal
                           'file-error
                           '("offline fixture" "network")))))
                      (list
                       (condition-case error
                           (progn
                             (aio-url-retrieve
                              "https://example.invalid")
                             :returned)
                         (error
                          (list :direct-error error)))
                       (aio-wait-for
                        (aio-catch
                         (aio-url-retrieve
                          "https://example.invalid")))))"##;
    let expect = expect![[r#"OK (:returned (:error file-error "offline fixture" "network"))"#]];
    assert_aio_parity(elisp_form, expect);
}
