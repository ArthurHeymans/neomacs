use expect_test::expect;

use super::assert_anx_api_parity;

#[test]
fn anx_api_parse_response_reads_json_from_declared_header_boundary() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer " *anx-http*")))
         (unwind-protect
             (with-current-buffer buffer
               (insert "HTTP/1.1 200 OK\r\nHeader: value\r\n\r\n"
                       "{\"response\":{\"status\":\"OK\",\"count\":2},"
                       "\"items\":[1,null,true]}")
               (setq-local url-http-end-of-headers
                           (save-excursion
                             (goto-char (point-min))
                             (search-forward "\r\n\r\n")
                             (point)))
               (goto-char 3)
               (let ((result (anx--parse-response buffer)))
                 (list result
                       (hash-table-p result)
                       (alist-get 'response result)
                       (alist-get 'items result)
                       (point)
                       (buffer-live-p buffer))))
           (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (((response . #1=((status . "OK") (count . 2))) (items . #2=[1 nil t])) nil #1# #2# 3 t)"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_parse_response_without_header_marker_returns_nil_and_preserves_buffer() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer " *anx-no-header*")))
         (unwind-protect
             (with-current-buffer buffer
               (insert "{\"ok\":true}")
               (when (boundp 'url-http-end-of-headers)
                 (makunbound 'url-http-end-of-headers))
               (goto-char 4)
               (list (anx--parse-response buffer)
                     (buffer-string)
                     (point)
                     (buffer-live-p buffer)))
           (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK (nil "{\"ok\":true}" 4 t)"#]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_send_request_encodes_payload_and_binds_complete_url_contract() {
    let elisp_form = r##"(let ((*anx-current-url* "https://api.example.test")
               calls)
         (cl-letf (((symbol-function 'url-retrieve-synchronously)
                    (lambda (url)
                      (push (list url url-request-method
                                  url-request-extra-headers
                                  url-request-data)
                            calls)
                      'response-buffer))
                   ((symbol-function 'anx--parse-response)
                    (lambda (buffer)
                      (push (list 'parse buffer) calls)
                      'parsed)))
           (list
            (anx--send-request
             "POST" "member/42"
             '(:member (:name "Ada" :active t :tags ["a" "b"])))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (parsed (("https://api.example.test/member/42" "POST" (("Content-Type" . "application/x-www-form-urlencoded")) "{\"member\":{\"name\":\"Ada\",\"active\":true,\"tags\":[\"a\",\"b\"]}}") (parse response-buffer)))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_send_request_without_payload_uses_empty_data_and_preserves_path_slashes() {
    let elisp_form = r##"(let ((*anx-current-url* "http://sand.example/")
               calls)
         (cl-letf (((symbol-function 'url-retrieve-synchronously)
                    (lambda (url)
                      (push (list url url-request-method
                                  url-request-extra-headers
                                  url-request-data)
                            calls)
                      'response))
                   ((symbol-function 'anx--parse-response)
                    (lambda (buffer)
                      (list 'parsed buffer))))
           (list (anx--send-request "GET" "/user?current")
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((parsed response) (("http://sand.example///user?current" "GET" (("Content-Type" . "application/x-www-form-urlencoded")) "")))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_authenticate_sends_live_credentials_and_opens_lisp_response() {
    let elisp_form = r##"(let ((anx-username "alice")
               (anx-password "p@ss")
               (*anx-current-url* "https://sandbox.example")
               calls)
         (cl-letf (((symbol-function 'anx--send-request)
                    (lambda (&rest args)
                      (push (cons 'send args) calls)
                      '(:response (:status "OK"))))
                   ((symbol-function 'anx--pop-up-buffer)
                    (lambda (&rest args)
                      (push (cons 'popup args) calls)
                      'shown)))
           (list (anx-authenticate)
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (shown ((send "POST" "auth" (:auth (:username "alice" :password "p@ss"))) (popup "https://sandbox.example/auth" (:response (:status "OK")) emacs-lisp-mode)))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_raw_get_preserves_literal_http_response_and_request_bindings() {
    let elisp_form = r##"(let ((response (generate-new-buffer " *anx-raw*"))
               calls)
         (unwind-protect
             (progn
               (with-current-buffer response
                 (insert "HTTP/1.1 200 OK\r\n\r\nraw,csv\n1,2\n"))
               (cl-letf (((symbol-function 'url-retrieve-synchronously)
                          (lambda (url)
                            (push (list 'retrieve url
                                        url-request-method
                                        url-request-extra-headers)
                                  calls)
                            response))
                         ((symbol-function 'anx--pop-up-buffer)
                          (lambda (&rest args)
                            (push (cons 'popup args) calls)
                            'shown)))
                 (list (anx-raw-get
                        "https://report.example/download?id=7")
                       (nreverse calls))))
           (kill-buffer response)))"##;
    let expect = expect![[
        r#"OK (shown ((retrieve "https://report.example/download?id=7" nil nil) (popup "https://report.example/download?id=7" "HTTP/1.1 200 OK\15\n\15\nraw,csv\n1,2\n" fundamental-mode)))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_get_delete_switch_user_and_whoami_route_exact_requests() {
    let elisp_form = r##"(let ((*anx-current-url* "https://api.example")
               calls)
         (cl-letf (((symbol-function 'anx--send-request)
                    (lambda (&rest args)
                      (push (cons 'send args) calls)
                      (list 'response args)))
                   ((symbol-function 'anx--pop-up-buffer)
                    (lambda (&rest args)
                      (push (cons 'popup args) calls)
                      'shown)))
           (list (anx-get "member/7?stats=true")
                 (anx-delete "campaign/9")
                 (anx-switch-users "42")
                 (anx-who-am-i)
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (shown shown shown shown ((send . #1=("GET" "member/7?stats=true")) (popup "https://api.example/member/7?stats=true" (response #1#) emacs-lisp-mode) (send . #2=("DELETE" "campaign/9")) (popup "https://api.example/campaign/9" (response #2#) emacs-lisp-mode) (send . #3=("POST" "auth" (:auth (:switch_to_user "42")))) (popup "*anx-switch-users*" (response #3#) emacs-lisp-mode) (send . #4=("GET" "user?current")) (popup "*anx-who-am-i*" (response #4#) emacs-lisp-mode)))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}
