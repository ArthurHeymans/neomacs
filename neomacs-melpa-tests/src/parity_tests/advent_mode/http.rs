use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_http_status_prefers_bound_response_variable_then_parses_status_line() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "HTTP/1.1 200 OK\r\n\r\nbody")
           (setq-local url-http-response-status 418)
           (advent--http--status))
         (with-temp-buffer
           (insert "HTTP/2 204 No Content\r\n\r\n")
           (makunbound 'url-http-response-status)
           (advent--http--status))
         (with-temp-buffer
           (insert "prefix\nHTTP/1.1 200 OK\r\n\r\n")
           (makunbound 'url-http-response-status)
           (advent--http--status))
         (with-temp-buffer
           (insert "GARBAGE\r\n\r\nbody")
           (makunbound 'url-http-response-status)
           (advent--http--status)))"##;
    let expect = expect!["OK (418 204 nil nil)"];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_http_body_returns_exact_payload_for_success_statuses() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (pcase-let ((`(,response ,required) case))
             (with-temp-buffer
               (insert response)
               (makunbound 'url-http-response-status)
               (list case
                     (advent--http--body required)
                     (point)))))
         '(("HTTP/1.1 200 OK\r\nX-Test: yes\r\n\r\nhello\n" nil)
           ("HTTP/1.1 201 Created\nHeader: x\n\n body " t)
           ("HTTP/1.1 204 No Content\r\n\r\n" nil)))"##;
    let expect = expect![[
        r#"OK ((("HTTP/1.1 200 OK\15\nX-Test: yes\15\n\15\nhello\n" nil) "hello\n" 33) (("HTTP/1.1 201 Created\nHeader: x\n\n body " t) " body " 33) (("HTTP/1.1 204 No Content\15\n\15\n" nil) "" 28))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_http_body_errors_cover_status_separator_empty_and_snippet_rules() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (pcase-let ((`(,response ,status ,required) case))
             (with-temp-buffer
               (insert response)
               (if status
                   (setq-local url-http-response-status status)
                 (makunbound 'url-http-response-status))
               (condition-case error-data
                   (advent--http--body required)
                 (error
                  (list 'signal
                        (car error-data)
                        (cdr error-data)))))))
         '(("GARBAGE\r\n\r\nbody" nil nil)
           ("HTTP/1.1 200 OK\r\nHeader: x" 200 nil)
           ("HTTP/1.1 404 Not Found\r\n\r\nnope" 404 nil)
           ("HTTP/1.1 500 Error\r\n\r\n   " 500 nil)
           ("HTTP/1.1 200 OK\r\n\r\n  \n" 200 t)
           ("HTTP/1.1 200 OK\r\n\r\nx" 200 t)))"##;
    let expect = expect![[
        r#"OK ((signal error ("Malformed HTTP response (no status)")) (signal error ("Malformed HTTP response (no header/body separator)")) (signal error ("HTTP 404: nope")) (signal error ("HTTP 500")) (signal error ("Empty HTTP response body")) "x")"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_http_request_binds_transport_arguments_returns_body_and_kills_buffer() {
    let elisp_form = r##"(let (calls response-buffer)
         (cl-letf (((symbol-function 'url-retrieve-synchronously)
                    (lambda (&rest arguments)
                      (setq response-buffer
                            (generate-new-buffer " *advent-http*"))
                      (push
                       (list arguments
                             url-request-method
                             url-request-extra-headers
                             url-request-data)
                       calls)
                      (with-current-buffer response-buffer
                        (insert
                         "HTTP/1.1 200 OK\r\nX-Test: yes\r\n\r\nresponse"))
                      response-buffer)))
           (list
            (advent--http-request
             "https://example.test/get")
            (buffer-live-p response-buffer)
            (advent--http-request
             "https://example.test/post"
             "POST"
             "x=1"
             t)
            (buffer-live-p response-buffer)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("response" nil "response" nil ((("https://example.test/get" t nil 30) "GET" nil nil) (("https://example.test/post" t nil 30) "POST" (("Content-Type" . "application/x-www-form-urlencoded")) "x=1")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_http_request_nil_transport_and_http_post_adapter_match() {
    let elisp_form = r##"(let (calls)
         (list
          (cl-letf (((symbol-function 'url-retrieve-synchronously)
                     (lambda (&rest _arguments) nil)))
            (condition-case error-data
                (advent--http-request
                 "https://example.test/unavailable"
                 "POST"
                 "x=1")
              (error
               (list 'signal
                     (car error-data)
                     (cdr error-data)))))
          (cl-letf (((symbol-function 'advent--http-request)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       "posted")))
            (list
             (advent--http-post
              "https://example.test/answer"
              "level=1&answer=42")
             (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((signal error ("Failed to POST https://example.test/unavailable")) ("posted" (("https://example.test/answer" "POST" "level=1&answer=42"))))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}
