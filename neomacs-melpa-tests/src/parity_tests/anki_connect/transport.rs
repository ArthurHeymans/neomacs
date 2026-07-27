use expect_test::expect;

use super::assert_anki_connect_parity;

#[test]
fn request_without_params_posts_versioned_json_and_reads_nested_utf8_result() {
    let elisp_form = r##"(let (captured response-buffer result)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'url-retrieve-synchronously)
                                (lambda
                                    (url &rest arguments)
                                  (setq captured
                                        (list
                                         url
                                         arguments
                                         url-request-method
                                         url-request-extra-headers
                                         (decode-coding-string
                                          url-request-data
                                          'utf-8)
                                         (multibyte-string-p
                                          url-request-data)
                                         (string-bytes
                                          url-request-data)))
                                  (setq response-buffer
                                        (generate-new-buffer
                                         " *anki-response*"))
                                  (with-current-buffer
                                      response-buffer
                                    (insert
                                     "HTTP/1.1 200 OK\nContent-Type: application/json\n\n")
                                    (insert
                                     "{\"result\":{\"decks\":[\"Default\",\"日本語\"],\"count\":2,\"active\":true,\"disabled\":false,\"none\":null},\"error\":null}"))
                                  response-buffer)))
                            (setq result
                                  (anki-connect-request
                                   "version"
                                   nil))
                            (list
                             result
                             captured
                             (buffer-live-p
                              response-buffer)
                             (with-current-buffer
                                 response-buffer
                               (list
                                (point)
                                (buffer-size)))))
                        (when
                            (buffer-live-p
                             response-buffer)
                          (kill-buffer
                           response-buffer))))"##;
    let expect = expect![[
        r#"OK (((decks . ["Default" "日本語"]) (count . 2) (active . t) (disabled . :json-false) (none)) ("http://127.0.0.1:8765" nil "POST" (("Content-Type" . "application/json")) "{\"action\":\"version\",\"version\":6}" nil 32) t (153 152))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn request_with_nested_unicode_params_preserves_json_shape_escaping_and_utf8_bytes() {
    let elisp_form = r##"(let (request-text request-object
                          response-buffer)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'url-retrieve-synchronously)
                                (lambda (url)
                                  (setq request-text
                                        (decode-coding-string
                                         url-request-data
                                         'utf-8))
                                  (let ((json-object-type
                                         'alist)
                                        (json-array-type
                                         'list)
                                        (json-key-type
                                         'symbol))
                                    (setq request-object
                                          (json-read-from-string
                                           request-text)))
                                  (setq response-buffer
                                        (generate-new-buffer
                                         " *anki-unicode-response*"))
                                  (with-current-buffer
                                      response-buffer
                                    (insert
                                     "HTTP/1.1 200 OK\n\n{\"result\":[101,202],\"error\":null}"))
                                  (list
                                   url
                                   response-buffer)
                                  response-buffer)))
                            (list
                             (anki-connect-request
                              "findNotes"
                              '(("query"
                                 . "deck:\"研究\" tag:due")
                                ("options"
                                 ("includeSuspended"
                                  . :json-false)
                                 ("limit" . 3))))
                             request-text
                             request-object
                             (string-to-list
                              (encode-coding-string
                               "研究"
                               'utf-8))))
                        (when
                            (buffer-live-p
                             response-buffer)
                          (kill-buffer
                           response-buffer))))"##;
    let expect = expect![[
        r#"OK ([101 202] "{\"action\":\"findNotes\",\"version\":6,\"params\":{\"query\":\"deck:\\\"研究\\\" tag:due\",\"options\":{\"includeSuspended\":false,\"limit\":3}}}" ((action . "findNotes") (version . 6) (params (query . "deck:\"研究\" tag:due") (options (includeSuspended . :json-false) (limit . 3)))) (231 160 148 231 169 182))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn generic_request_supports_realistic_multi_action_batch_and_nested_results() {
    let elisp_form = r##"(let (captured response-buffer)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'url-retrieve-synchronously)
                                (lambda (_url)
                                  (setq captured
                                        (decode-coding-string
                                         url-request-data
                                         'utf-8))
                                  (setq response-buffer
                                        (generate-new-buffer
                                         " *anki-multi-response*"))
                                  (with-current-buffer
                                      response-buffer
                                    (insert
                                     "HTTP/1.1 200 OK\n\n{\"result\":[[\"Default\",\"Study\"],[\"Basic\",\"Cloze\"],31415],\"error\":null}"))
                                  response-buffer)))
                            (list
                             (anki-connect-request
                              "multi"
                              '(("actions"
                                 . [(("action"
                                     . "deckNames"))
                                    (("action"
                                      . "modelNames"))
                                    (("action"
                                      . "addNote")
                                     ("params"
                                      ("note"
                                       ("deckName"
                                        . "Study")
                                       ("modelName"
                                        . "Basic")
                                       ("fields"
                                        ("Front"
                                         . "Question")
                                        ("Back"
                                         . "Answer"))
                                       ("tags"
                                        . []))))])))
                             captured))
                        (when
                            (buffer-live-p
                             response-buffer)
                          (kill-buffer
                           response-buffer))))"##;
    let expect = expect![[
        r#"OK ([["Default" "Study"] ["Basic" "Cloze"] 31415] "{\"action\":\"multi\",\"version\":6,\"params\":{\"actions\":[{\"action\":\"deckNames\"},{\"action\":\"modelNames\"},{\"action\":\"addNote\",\"params\":{\"note\":{\"deckName\":\"Study\",\"modelName\":\"Basic\",\"fields\":{\"Front\":\"Question\",\"Back\":\"Answer\"},\"tags\":[]}}}]}}")"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn protocol_error_is_silently_nil_while_null_error_returns_result() {
    let elisp_form = r##"(let ((bodies
                           '("{\"result\":null,\"error\":\"collection is unavailable\"}"
                             "{\"result\":42,\"error\":null}"))
                          buffers)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'url-retrieve-synchronously)
                                (lambda (_url)
                                  (let ((buffer
                                         (generate-new-buffer
                                          " *anki-error-response*")))
                                    (push buffer buffers)
                                    (with-current-buffer
                                        buffer
                                      (insert
                                       "HTTP/1.1 200 OK\n\n")
                                      (insert
                                       (pop bodies)))
                                    buffer))))
                            (list
                             (anki-connect-request
                              "sync"
                              nil)
                             (anki-connect-request
                              "version"
                              nil)
                             (length buffers)
                             bodies))
                        (mapc
                         (lambda (buffer)
                           (when
                               (buffer-live-p buffer)
                             (kill-buffer buffer)))
                         buffers)))"##;
    let expect = expect!["OK (nil 42 2 nil)"];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn http_status_and_headers_are_ignored_when_body_contains_valid_protocol_json() {
    let elisp_form = r##"(let (response-buffer)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'url-retrieve-synchronously)
                                (lambda (_url)
                                  (setq response-buffer
                                        (generate-new-buffer
                                         " *anki-http-error-response*"))
                                  (with-current-buffer
                                      response-buffer
                                    (insert
                                     "HTTP/1.1 503 Service Unavailable\nRetry-After: 30\nX-Debug: deterministic\n\n{\"result\":\"body-wins\",\"error\":null}"))
                                  response-buffer)))
                            (anki-connect-request
                             "version"
                             nil))
                        (when
                            (buffer-live-p
                             response-buffer)
                          (kill-buffer
                           response-buffer))))"##;
    let expect = expect![[r#"OK "body-wins""#]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn malformed_header_boundary_and_invalid_json_preserve_exact_parser_signals() {
    let elisp_form = r##"(let (buffers)
                      (unwind-protect
                          (cl-labels
                              ((response
                                 (text)
                                 (let ((buffer
                                        (generate-new-buffer
                                         " *anki-malformed-response*")))
                                   (push buffer buffers)
                                   (with-current-buffer
                                       buffer
                                     (insert text))
                                   buffer)))
                            (list
                             (cl-letf
                                 (((symbol-function
                                    'url-retrieve-synchronously)
                                   (lambda (_url)
                                     (response
                                      "HTTP/1.1 200 OK\nContent-Type: application/json\n{\"result\":1,\"error\":null}"))))
                               (condition-case error-data
                                   (anki-connect-request
                                    "version"
                                    nil)
                                 (error error-data)))
                             (cl-letf
                                 (((symbol-function
                                    'url-retrieve-synchronously)
                                   (lambda (_url)
                                     (response
                                      "HTTP/1.1 200 OK\n\n{\"result\":"))))
                               (condition-case error-data
                                   (anki-connect-request
                                    "version"
                                    nil)
                                 (error error-data)))))
                        (mapc
                         (lambda (buffer)
                           (when
                               (buffer-live-p buffer)
                             (kill-buffer buffer)))
                         buffers)))"##;
    let expect = expect![[r#"OK ((search-failed "^$") (json-end-of-file))"#]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn unavailable_transport_and_connection_signal_propagate_without_network_fallback() {
    let elisp_form = r##"(list
                      (cl-letf
                          (((symbol-function
                             'url-retrieve-synchronously)
                            (lambda (_url)
                              nil)))
                        (condition-case error-data
                            (anki-connect-request
                             "version"
                             nil)
                          (error error-data)))
                      (cl-letf
                          (((symbol-function
                             'url-retrieve-synchronously)
                            (lambda (url)
                              (signal
                               'file-error
                               (list
                                "connection refused"
                                url)))))
                        (condition-case error-data
                            (anki-connect-request
                             "version"
                             nil)
                          (error error-data))))"##;
    let expect = expect![[
        r#"OK ((wrong-type-argument stringp nil) (file-error "connection refused" "http://127.0.0.1:8765"))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}
