use expect_test::expect;

use super::assert_asx_batch;

#[test]
fn request_public_surface_batch() {
    assert_asx_batch(&[
        (
            "asx_user_agent_selection_passes_complete_configured_pool_to_random_selector",
            r##"(let ((asx--user-agents
                '("Agent A"
                  "Agent B"
                  "Agent C"))
               observed)
         (cl-letf
             (((symbol-function
                'seq-random-elt)
               (lambda (sequence)
                 (setq observed
                       (copy-sequence sequence))
                 (nth 1 sequence))))
           (list
            (asx--get-user-agent)
            observed
            asx--user-agents)))"##,
            true,
            expect![[
        r#"OK ("Agent B" ("Agent A" "Agent B" "Agent C") ("Agent A" "Agent B" "Agent C"))"#
    ]],
        ),
        (
            "asx_request_configures_user_agent_parses_html_and_forwards_success_dom",
            r##"(let ((asx--user-agents
                '("Fixture Agent"))
               callback-values
               request-observation)
         (cl-letf
             (((symbol-function 'request)
               (lambda
                 (url &rest properties)
                 (let* ((parser
                         (plist-get
                          properties
                          :parser))
                        (success
                         (plist-get
                          properties
                          :success))
                        (parsed
                         (with-temp-buffer
                           (insert
                            "<html><body><h1>Result</h1><p>Body</p></body></html>")
                           (funcall parser))))
                   (setq request-observation
                         (list
                          url
                          request-curl-options
                          (and
                           (plist-member
                            properties
                            :error)
                           (functionp
                            (plist-get
                             properties
                             :error)))
                          (and
                           (plist-member
                            properties
                            :success)
                           (functionp success))
                          parsed))
                   (funcall
                    success
                    :data parsed)
                   :request-return))))
           (list
            (asx--request
             "https://search.invalid/query"
             (lambda (data)
               (push data callback-values)))
            request-observation
            callback-values)))"##,
            true,
            expect![[
        r#"OK (:request-return ("https://search.invalid/query" ("-A Fixture Agent") t t #1=(html nil (body nil (h1 nil "Result") (p nil "Body")))) (#1#))"#
    ]],
        ),
        (
            "asx_request_custom_error_callback_receives_original_url_and_suppresses_signal",
            r##"(let ((asx--user-agents
                '("Fixture Agent"))
               events)
         (cl-letf
             (((symbol-function 'request)
               (lambda
                 (url &rest properties)
                 (push
                  (list
                   :requested
                   url
                   request-curl-options)
                  events)
                 (funcall
                  (plist-get
                   properties
                   :error)
                  :error-thrown
                  '(file-error
                    "offline"))
                 :handled)))
           (list
            (asx--request
             "https://search.invalid/fail"
             (lambda (_)
               (push :unexpected-success events))
             (lambda (url)
               (push
                (list
                 :fallback
                 url)
                events)))
            (nreverse events))))"##,
            true,
            expect![[
        r#"OK (:handled ((:requested "https://search.invalid/fail" ("-A Fixture Agent")) (:fallback "https://search.invalid/fail")))"#
    ]],
        ),
        (
            "asx_request_default_error_handler_signals_stringified_request_error",
            r##"(let ((asx--user-agents
                '("Fixture Agent")))
         (cl-letf
         (((symbol-function 'request)
           (lambda
             (_url &rest properties)
             (funcall
              (plist-get
               properties
               :error)
              :error-thrown
              '(error
                "network exploded")))))
         (condition-case error
             (asx--request
              "https://search.invalid/fail"
              #'ignore)
           (error
            (list
             (car error)
             (cdr error))))))"##,
            true,
            expect![[r#"OK (error ("(error network exploded)"))"#]],
        ),
        (
            "asx_request_post_announces_title_and_dispatches_url_insert_and_retry_callbacks",
            r##"(let (messages requests)
         (cl-letf
             (((symbol-function 'message)
               (lambda
                 (format-string &rest arguments)
                 (push
                  (apply
                   #'format
                   format-string
                   arguments)
                  messages)))
              ((symbol-function 'asx--request)
               (lambda
                 (url callback
                      &optional error-callback)
                 (push
                  (list
                   url
                   callback
                   error-callback)
                  requests)
                 :queued)))
           (list
            (asx--request-post
             '("A useful answer"
               .
               "https://stackoverflow.com/questions/7/useful"))
            (nreverse messages)
            (nreverse requests))))"##,
            true,
            expect![[
        r#"OK (:queued ("Loading: A useful answer") (("https://stackoverflow.com/questions/7/useful" asx--insert-post-dom asx--remove-and-next)))"#
    ]],
        ),
        (
            "asx_request_parser_handles_entities_nested_elements_and_malformed_html_practically",
            r##"(let ((asx--user-agents
                '("Fixture Agent"))
               parsed)
         (cl-letf
             (((symbol-function 'request)
               (lambda
                 (_url &rest properties)
                 (setq parsed
                       (with-temp-buffer
                         (insert
                          "<html><body><p>A &amp; B <strong>bold<p>second")
                         (funcall
                          (plist-get
                           properties
                           :parser))))
                 (funcall
                  (plist-get
                   properties
                   :success)
                  :data parsed))))
           (asx--request
            "https://fixture.invalid"
            #'ignore)
           (list
            (dom-texts parsed)
            (mapcar
             #'dom-texts
             (dom-by-tag
              parsed
              'p))
            (length
             (dom-by-tag
              parsed
              'strong)))))"##,
            true,
            expect![[r#"OK ("A & B  bold second" ("A & B  bold second" "second") 1)"#]],
        ),
    ]);
}
