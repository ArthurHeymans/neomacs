use expect_test::expect;

use super::assert_aidev_mode_parity;

#[test]
fn aidev_mode_chat_dispatches_all_providers_with_model_and_trims_response() {
    let elisp_form = r##"(let ((aidev-default-model
                "frozen-default-model")
               calls)
         (cl-letf
             (((symbol-function
                'aidev---ollama)
               (lambda
                 (messages system model)
                 (push
                  (list 'ollama
                        messages system model)
                  calls)
                 "  ollama response\n"))
              ((symbol-function
                'aidev---openai)
               (lambda
                 (messages system model)
                 (push
                  (list 'openai
                        messages system model)
                  calls)
                 "\nopenai response  "))
              ((symbol-function
                'aidev---claude)
               (lambda
                 (messages system model)
                 (push
                  (list 'claude
                        messages system model)
                  calls)
                 "\tclaude response\t")))
           (let ((messages
                  '((("role" . "user")
                     ("content" . "Implement it"))))
                 results)
             (dolist (provider
                      '(ollama openai claude))
               (let ((aidev-provider provider))
                 (push
                  (aidev--chat
                   "System policy"
                   messages)
                  results)))
             (let ((aidev-provider
                    'unsupported-provider))
               (list
                (nreverse results)
                (nreverse calls)
                (condition-case error-data
                    (aidev--chat
                     "System policy"
                     messages)
                  (error error-data)))))))"##;
    let expect = expect![[
        r#"OK (("ollama response" "openai response" "claude response") ((ollama #1=((("role" . "user") ("content" . "Implement it"))) "System policy" "frozen-default-model") (openai #1# "System policy" "frozen-default-model") (claude #1# "System policy" "frozen-default-model")) (error "Unknown AI provider: unsupported-provider"))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_ollama_availability_orchestrates_async_probe_and_handles_failures() {
    let elisp_form = r##"(let ((sentinel-event "open\n")
               fail-connect
               events)
         (cl-letf
             (((symbol-function
                'make-network-process)
               (lambda (&rest arguments)
                 (push
                  (cons 'make arguments)
                  events)
                 (if fail-connect
                     (error "connection refused")
                   'frozen-network-process)))
              ((symbol-function
                'set-process-sentinel)
               (lambda (process sentinel)
                 (push
                  (list 'sentinel process)
                  events)
                 (funcall
                  sentinel
                  process
                  sentinel-event)))
              ((symbol-function
                'sleep-for)
               (lambda (&rest duration)
                 (push
                  (cons 'sleep duration)
                  events)))
              ((symbol-function
                'delete-process)
               (lambda (process)
                 (push
                  (list 'delete process)
                  events))))
           (let ((opened
                  (aidev---ollama-available
                   "http://ollama.example:11434/")))
             (setq sentinel-event
                   "failed with code 111\n")
             (let ((closed
                    (aidev---ollama-available
                     "http://ollama.example:11435/")))
               (setq fail-connect t)
               (let ((failed
                      (aidev---ollama-available
                       "http://ollama.example:11436/")))
                 (list
                  opened
                  closed
                  failed
                  (aidev---ollama-available
                   "not-a-url")
                  (nreverse events)))))))"##;
    let expect = expect![[
        r#"OK ("http://ollama.example:11434/" nil nil nil ((make :name "ollama-test" :host "ollama.example" :service 11434 :nowait t) (sentinel frozen-network-process) (sleep 0.2) (delete frozen-network-process) (make :name "ollama-test" :host "ollama.example" :service 11435 :nowait t) (sentinel frozen-network-process) (sleep 0.2) (delete frozen-network-process) (make :name "ollama-test" :host "ollama.example" :service 11436 :nowait t)))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_ollama_request_builds_json_parses_response_and_kills_http_buffer() {
    let elisp_form = r##"(let ((aidev---ollama-default-url
                "http://ollama.example:11434/")
               response-buffer
               requests)
         (cl-letf
             (((symbol-function
                'url-retrieve-synchronously)
               (lambda (url)
                 (push
                  (list
                   url
                   url-request-method
                   url-request-extra-headers
                   (json-read-from-string
                    url-request-data))
                  requests)
                 (setq response-buffer
                       (generate-new-buffer
                        " *aidev-ollama-response*"))
                 (with-current-buffer
                     response-buffer
                   (insert
                    "HTTP/1.1 200 OK\n"
                    "Content-Type: application/json\n"
                    "\n"
                    "{\"response\":\"generated implementation\"}"))
                 response-buffer)))
           (let ((result
                  (aidev---ollama
                   '((("role" . "user")
                      ("content" . "Generate code")))
                   "System policy"
                   "deepseek-frozen")))
             (list
              result
              (buffer-live-p
               response-buffer)
              (nreverse requests)))))"##;
    let expect = expect![[
        r#"OK ("generated implementation" nil (("http://ollama.example:11434//api/generate" "POST" (("Content-Type" . "application/json")) ((prompt . "SYSTEM PROMPT: System policy MESSAGES: [{\"role\":\"user\",\"content\":\"Generate code\"}]") (stream . :json-false) (model . "deepseek-frozen")))))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_ollama_requires_resolved_url_before_any_http_request() {
    let elisp_form = r##"(let ((aidev---ollama-default-url nil)
               calls)
         (cl-letf
             (((symbol-function
                'url-retrieve-synchronously)
               (lambda (&rest arguments)
                 (push arguments calls)
                 nil)))
           (list
            (condition-case error-data
                (aidev---ollama
                 '((("role" . "user")
                    ("content" . "Hello")))
                 "System"
                 nil)
              (error error-data))
            calls)))"##;
    let expect = expect![[r#"OK ((error "Invalid error symbol" ollama-url-unset) nil)"#]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_openai_builds_supported_and_fallback_system_messages_and_auth_headers() {
    let elisp_form = r##"(let ((process-environment
                (copy-sequence
                 process-environment))
               response-buffers
               requests)
         (setenv "OPENAI_API_KEY"
                 "openai-test-key")
         (cl-letf
             (((symbol-function
                'url-retrieve-synchronously)
               (lambda (url)
                 (push
                  (list
                   url
                   url-request-method
                   url-request-extra-headers
                   (json-read-from-string
                    url-request-data))
                  requests)
                 (let ((buffer
                        (generate-new-buffer
                         " *aidev-openai-response*")))
                   (push buffer
                         response-buffers)
                   (with-current-buffer buffer
                     (insert
                      "HTTP/1.1 200 OK\n\n"
                      "{\"choices\":[{\"message\":{\"content\":\"alphaâbeta\"}}]}"))
                   buffer))))
           (let ((messages
                  '(((role . "user")
                     (content . "Implement")))))
             (let ((supported
                    (aidev---openai
                     messages
                     "System policy"
                     "gpt-4"))
                   (fallback
                    (aidev---openai
                     messages
                     "System policy"
                     "o3-mini"))
                   (without-system
                    (aidev---openai
                     messages nil nil)))
               (list
                supported
                fallback
                without-system
                (mapcar
                 #'buffer-live-p
                 response-buffers)
                (nreverse requests))))))"##;
    let expect = expect![[
        r#"OK ("alpha-beta" "alpha-beta" "alpha-beta" (nil nil nil) (("https://api.openai.com/v1/chat/completions" "POST" (#1=("Content-Type" . "application/json") ("Authorization" . "Bearer openai-test-key")) ((messages . [((role . "system") (content . "System policy")) ((role . "user") (content . "Implement"))]) (model . "gpt-4"))) ("https://api.openai.com/v1/chat/completions" "POST" (#1# ("Authorization" . "Bearer openai-test-key")) ((messages . [((role . "user") (content . "SYSTEM_PROMPT: System policy")) ((role . "user") (content . "Implement"))]) (model . "o3-mini"))) ("https://api.openai.com/v1/chat/completions" "POST" (#1# ("Authorization" . "Bearer openai-test-key")) ((messages . [((role . "user") (content . "Implement"))]) (model . "o3-mini")))))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_claude_builds_optional_system_payload_api_headers_and_parses_content() {
    let elisp_form = r##"(let ((process-environment
                (copy-sequence
                 process-environment))
               response-buffers
               requests)
         (setenv "ANTHROPIC_API_KEY"
                 "anthropic-test-key")
         (cl-letf
             (((symbol-function
                'url-retrieve-synchronously)
               (lambda (url)
                 (push
                  (list
                   url
                   url-request-method
                   url-request-extra-headers
                   (json-read-from-string
                    url-request-data))
                  requests)
                 (let ((buffer
                        (generate-new-buffer
                         " *aidev-claude-response*")))
                   (push buffer
                         response-buffers)
                   (with-current-buffer buffer
                     (insert
                      "HTTP/1.1 200 OK\n\n"
                      "{\"content\":[{\"text\":\"strict answer\"}]}"))
                   buffer))))
           (let ((messages
                  '((("role" . "user")
                     ("content" . "Review this")))))
             (let ((with-system
                    (aidev---claude
                     messages
                     "System policy"
                     "claude-frozen"))
                   (without-system
                    (aidev---claude
                     messages nil nil)))
               (list
                with-system
                without-system
                (mapcar
                 #'buffer-live-p
                 response-buffers)
                (nreverse requests))))))"##;
    let expect = expect![[
        r#"OK ("strict answer" "strict answer" (nil nil) (("https://api.anthropic.com/v1/messages" "POST" (#1=("Content-Type" . "application/json") ("X-Api-Key" . "anthropic-test-key") . #2=(("anthropic-version" . "2023-06-01"))) ((messages . [((role . "user") (content . "Review this"))]) (model . "claude-frozen") (max_tokens . 4096) (system . "System policy"))) ("https://api.anthropic.com/v1/messages" "POST" (#1# ("X-Api-Key" . "anthropic-test-key") . #2#) ((messages . [((role . "user") (content . "Review this"))]) (model . "claude-3-5-sonnet-20240620") (max_tokens . 4096)))))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_provider_http_cleanup_runs_when_response_body_is_malformed() {
    let elisp_form = r##"(let ((process-environment
                (copy-sequence
                 process-environment))
               (aidev---ollama-default-url
                "http://ollama.example:11434")
               buffers)
         (setenv "OPENAI_API_KEY" "key")
         (setenv "ANTHROPIC_API_KEY" "key")
         (cl-letf
             (((symbol-function
                'url-retrieve-synchronously)
               (lambda (_)
                 (let ((buffer
                        (generate-new-buffer
                         " *aidev-malformed-response*")))
                   (push buffer buffers)
                   (with-current-buffer buffer
                     (insert
                      "HTTP/1.1 200 OK\n\n"
                      "{not-json"))
                   buffer))))
           (let ((outcomes
                  (mapcar
                   (lambda (provider)
                     (condition-case error-data
                         (pcase provider
                           ('ollama
                            (aidev---ollama
                             nil nil nil))
                           ('openai
                            (aidev---openai
                             nil nil nil))
                           ('claude
                            (aidev---claude
                             nil nil nil)))
                       (error
                        (car error-data))))
                   '(ollama openai claude))))
             (list
              outcomes
              (mapcar
               #'buffer-live-p
               buffers)))))"##;
    let expect = expect!["OK ((json-end-of-file json-end-of-file json-end-of-file) (nil nil nil))"];
    assert_aidev_mode_parity(elisp_form, expect);
}
