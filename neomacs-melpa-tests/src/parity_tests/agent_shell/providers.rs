use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn default_agent_catalog_resolves_to_complete_user_facing_configs() {
    let elisp_form = r##"
(mapcar
 (lambda (config)
   (list
    (map-elt config :identifier)
    (map-elt config :mode-line-name)
    (map-elt config :buffer-name)
    (map-elt config :shell-prompt)
    (map-elt config :shell-prompt-regexp)
    (map-elt config :icon-name)
    (and (map-elt config :welcome-function) t)
    (and (map-elt config :client-maker) t)
    (map-elt config :needs-authentication)
    (and (map-elt config :authenticate-request-maker) t)
    (and (map-elt config :notification-adapter) t)
    (map-elt config :session-meta)
    (not (string-empty-p
          (or (map-elt config :install-instructions) "")))))
 (agent-shell--resolved-agent-configs))
"##;
    let expect = expect![[
        r#"OK ((auggie "Auggie" "Auggie" "Auggie> " "Auggie> " nil t t nil nil nil nil t) (claude-code "Claude" "Claude" "Claude> " "Claude> " "claudecode.png" t t nil nil nil ((claudeCode (options (thinking (type . "adaptive") (display . "summarized"))))) t) (codebuddy "CodeBuddy" "CodeBuddy" "CodeBuddy> " "CodeBuddy> " "codebuddy.png" t t nil nil nil nil t) (cline "Cline" "Cline" "Cline> " "Cline> " "cline.png" t t nil nil nil nil t) (codex "Codex" "Codex" "Codex> " "Codex> " "openai.png" t t nil nil nil nil t) (cursor "Cursor" "Cursor" "Cursor> " "Cursor> " "cursor.png" t t nil t t nil t) (droid "Droid" "Droid" "Droid> " "Droid> " "https://avatars.githubusercontent.com/u/131064358" t t nil nil nil nil t) (copilot "Copilot" "Copilot" "Copilot> " "Copilot> " "githubcopilot.png" t t nil nil nil nil t) (gemini-cli "Gemini" "Gemini" "Gemini> " "Gemini> " "gemini.png" t t t t nil nil t) (goose "Goose" "Goose" "Goose> " "Goose> " "goose.png" t t nil nil nil nil t) (hermes "Hermes" "Hermes" "Hermes> " "Hermes> " "hermesagent.png" t t nil nil nil nil t) (kimi "Kimi" "Kimi" "Kimi> " "Kimi> " "kimi.png" t t nil nil nil nil t) (kiro "Kiro" "Kiro" "Kiro> " "Kiro> " "kiro.png" t t nil nil nil nil t) (mistral-vibe "Mistral Vibe" "Mistral Vibe" "Vibe> " "Vibe> " "mistral.png" t t nil nil nil nil t) (omp "OMP" "OMP" "OMP> " "OMP> " nil t t nil nil nil nil t) (opencode "OpenCode" "OpenCode" "OpenCode> " "OpenCode> " "opencode.png" t t nil nil nil nil t) (pi "Pi" "Pi" "Pi> " "Pi> " "pi.png" t t nil nil nil nil t) (qwen-code "Qwen Code" "Qwen Code" "qwen> " "qwen> " "qwen.png" t t t t nil nil t) (grok-build "Grok" "Grok" "Grok> " "Grok> " "xai.png" t t t t nil nil t))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn authentication_builders_accept_each_supported_real_login_strategy() {
    let elisp_form = r##"
(list
 (agent-shell-anthropic-make-authentication :login t)
 (agent-shell-anthropic-make-authentication :api-key "anthropic")
 (agent-shell-anthropic-make-authentication :oauth "oauth")
 (agent-shell-cursor-make-authentication :none t)
 (agent-shell-cursor-make-authentication :login t)
 (agent-shell-cursor-make-authentication :api-key "cursor")
 (agent-shell-cursor-make-authentication :auth-token "token")
 (agent-shell-droid-make-authentication :none t)
 (agent-shell-droid-make-authentication :api-key "droid")
 (agent-shell-google-make-authentication :login t)
 (agent-shell-google-make-authentication :api-key "google")
 (agent-shell-google-make-authentication :vertex-ai t)
 (agent-shell-google-make-authentication :none t)
 (agent-shell-make-goose-authentication :none t)
 (agent-shell-make-goose-authentication :openai-api-key "goose")
 (agent-shell-mistral-make-authentication :api-key "mistral")
 (agent-shell-openai-make-authentication :login t)
 (agent-shell-openai-make-authentication :api-key "openai")
 (agent-shell-openai-make-authentication :codex-api-key "codex")
 (agent-shell-opencode-make-authentication :none t)
 (agent-shell-opencode-make-authentication :api-key "opencode")
 (agent-shell-qwen-make-authentication :login t)
 (agent-shell-qwen-make-authentication :openai-api-key "qwen"))
"##;
    let expect = expect![[
        r#"OK (((:login . t)) ((:api-key . "anthropic")) ((:oauth . "oauth")) ((:none . t)) ((:login . t)) ((:api-key . "cursor")) ((:auth-token . "token")) ((:none . t)) ((:api-key . "droid")) ((:login . t)) ((:api-key . "google")) ((:vertex-ai . t)) ((:none . t)) ((:none . t)) ((:openai-api-key . "goose")) ((:api-key . "mistral")) ((:login . t)) ((:api-key . "openai")) ((:codex-api-key . "codex")) ((:none . t)) ((:api-key . "opencode")) ((:login . t)) ((:openai-api-key . "qwen")))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn authentication_builders_reject_ambiguous_and_missing_credentials() {
    let elisp_form = r##"
(mapcar
 (lambda (thunk)
   (condition-case error
       (funcall thunk)
     (error (list (car error) (error-message-string error)))))
 (list
  (lambda () (agent-shell-anthropic-make-authentication))
  (lambda () (agent-shell-anthropic-make-authentication
              :login t :api-key "both"))
  (lambda () (agent-shell-cursor-make-authentication))
  (lambda () (agent-shell-cursor-make-authentication
              :api-key "one" :auth-token "two"))
  (lambda () (agent-shell-google-make-authentication))
  (lambda () (agent-shell-google-make-authentication
              :login t :vertex-ai t))
  (lambda () (agent-shell-openai-make-authentication))
  (lambda () (agent-shell-openai-make-authentication
              :login t :codex-api-key "both"))
  (lambda () (agent-shell-qwen-make-authentication))
  (lambda () (agent-shell-qwen-make-authentication
              :login t :openai-api-key "both"))))
"##;
    let expect = expect![[
        r#"OK ((error "Must specify either :api-key, :login, or :oauth") (error "Cannot specify both :api-key and :login - choose one") (error "Must specify one of :api-key, :auth-token, :login, or :none") (error "Cannot specify multiple authentication methods - choose one") (error "Must specify one of :api-key, :login, or :vertex-ai") (error "Cannot specify multiple authentication methods - choose one") (error "Must specify one of :api-key, :codex-api-key, :login") (error "Cannot specify multiple authentication methods - choose one") (error "Must specify either :login or :openai-api-key") (error "Cannot specify both :login and :openai-api-key - choose one"))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn codex_auth_request_matches_each_cli_authentication_contract() {
    let elisp_form = r##"
(mapcar
 (lambda (authentication)
   (let ((agent-shell-openai-authentication authentication))
     (json-parse-string
      (agent-shell-openai--codex-default-auth-request)
      :object-type 'alist
      :array-type 'list)))
 (list
  (agent-shell-openai-make-authentication :login t)
  (agent-shell-openai-make-authentication :api-key "openai-secret")
  (agent-shell-openai-make-authentication :codex-api-key "codex-secret")))
"##;
    let expect = expect![[
        r#"OK (((methodId . "chat-gpt")) ((methodId . "api-key") (_meta (api-key (apiKey . "openai-secret")))) ((methodId . "api-key") (_meta (api-key (apiKey . "codex-secret")))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn claude_client_translates_login_api_key_and_oauth_into_process_environment() {
    let elisp_form = r##"
(cl-letf (((symbol-function 'agent-shell--make-acp-client)
           (lambda (&rest arguments) arguments)))
  (mapcar
   (lambda (authentication)
     (let ((agent-shell-anthropic-authentication authentication)
           (agent-shell-anthropic-claude-acp-command
            '("claude-agent-acp" "--debug"))
           (agent-shell-anthropic-claude-environment
            '("HTTP_PROXY=http://proxy.test")))
       (agent-shell-anthropic-make-claude-client
        :buffer (current-buffer))))
   (list
    (agent-shell-anthropic-make-authentication :login t)
    (agent-shell-anthropic-make-authentication :api-key "key-value")
    (agent-shell-anthropic-make-authentication :oauth "oauth-value"))))
"##;
    let expect = expect![[
        r#"OK ((:command "claude-agent-acp" :command-params #1=("--debug") :environment-variables ("ANTHROPIC_API_KEY=" . #2=("HTTP_PROXY=http://proxy.test")) :context-buffer (:buffer #3="*scratch*")) (:command "claude-agent-acp" :command-params #1# :environment-variables ("ANTHROPIC_API_KEY=key-value" . #2#) :context-buffer (:buffer #3#)) (:command "claude-agent-acp" :command-params #1# :environment-variables ("CLAUDE_CODE_OAUTH_TOKEN=oauth-value" . #2#) :context-buffer (:buffer #3#)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn cursor_client_resolves_string_function_and_external_authentication() {
    let elisp_form = r##"
(cl-letf (((symbol-function 'agent-shell--make-acp-client)
           (lambda (&rest arguments) arguments)))
  (mapcar
   (lambda (authentication)
     (let ((agent-shell-cursor-authentication authentication)
           (agent-shell-cursor-acp-command '("agent" "acp" "--stdio"))
           (agent-shell-cursor-environment '("CURSOR_LOG=debug")))
       (agent-shell-cursor-make-client :buffer (current-buffer))))
   (list
    (agent-shell-cursor-make-authentication :none t)
    (agent-shell-cursor-make-authentication :login t)
    (agent-shell-cursor-make-authentication :api-key "cursor-key")
    (agent-shell-cursor-make-authentication
     :auth-token (lambda () "resolved-token")))))
"##;
    let expect = expect![[
        r#"OK ((:command "agent" :command-params #1=("acp" "--stdio") :environment-variables #2=("CURSOR_LOG=debug") :context-buffer (:buffer #3="*scratch*")) (:command "agent" :command-params #1# :environment-variables #2# :context-buffer (:buffer #3#)) (:command "agent" :command-params #1# :environment-variables ("CURSOR_API_KEY=cursor-key" . #2#) :context-buffer (:buffer #3#)) (:command "agent" :command-params #1# :environment-variables ("CURSOR_AUTH_TOKEN=resolved-token" . #2#) :context-buffer (:buffer #3#)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn gemini_and_qwen_clients_build_provider_specific_environment() {
    let elisp_form = r##"
(cl-letf (((symbol-function 'agent-shell--make-acp-client)
           (lambda (&rest arguments) arguments)))
  (list
   (let ((agent-shell-google-authentication
          (agent-shell-google-make-authentication :api-key
                                                   (lambda () "gemini-key")))
         (agent-shell-google-gemini-acp-command
          '("gemini" "--experimental-acp"))
         (agent-shell-google-gemini-environment
          '("GOOGLE_CLOUD_PROJECT=parity")))
     (agent-shell-google-make-gemini-client :buffer (current-buffer)))
   (let ((agent-shell-google-authentication
          (agent-shell-google-make-authentication :vertex-ai t))
         (agent-shell-google-gemini-acp-command
          '("gemini" "--experimental-acp"))
         (agent-shell-google-gemini-environment
          '("GOOGLE_CLOUD_LOCATION=us-central1")))
     (agent-shell-google-make-gemini-client :buffer (current-buffer)))
   (let ((agent-shell-qwen-authentication
          (agent-shell-qwen-make-authentication
           :openai-api-key (lambda () "qwen-key")))
         (agent-shell-qwen-acp-command '("qwen" "--experimental-acp"))
         (agent-shell-qwen-environment
          '("OPENAI_BASE_URL=https://provider.test/v1"
            "OPENAI_MODEL=model-x")))
     (agent-shell-qwen-make-client :buffer (current-buffer)))))
"##;
    let expect = expect![[
        r#"OK ((:command "gemini" :command-params ("--experimental-acp") :environment-variables ("GEMINI_API_KEY=gemini-key" "GOOGLE_CLOUD_PROJECT=parity") :context-buffer (:buffer #1="*scratch*")) (:command "gemini" :command-params ("--experimental-acp") :environment-variables ("GOOGLE_CLOUD_LOCATION=us-central1") :context-buffer (:buffer #1#)) (:command "qwen" :command-params ("--experimental-acp") :environment-variables ("OPENAI_API_KEY=qwen-key" "OPENAI_BASE_URL=https://provider.test/v1" "OPENAI_MODEL=model-x") :context-buffer (:buffer #1#)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn cursor_adapts_real_command_search_and_file_outputs_into_visible_content() {
    let elisp_form = r##"
(mapcar
 (lambda (raw-output)
   (let ((notification
          `((method . "session/update")
            (params
             (update
              (sessionUpdate . "tool_call_update")
              (status . "completed")
              (rawOutput . ,raw-output))))))
     (agent-shell-cursor--notification-adapter
      :acp-notification notification)
     (map-nested-elt notification '(params update content))))
 '(((stdout . "compiled 42 crates")
    (stderr . "one warning")
    (exitCode . 0))
   ((error . "permission denied"))
   ((content . "file contents\nsecond line"))
   ((totalMatches . 42) (truncated . t))
   ((resultCount . 7))
   ((unknown . "leave absent"))))
"##;
    let expect = expect![[
        r#"OK (((#1=(type . "content") (content #2=(type . "text") (text . "```\nExit code: 0\n\ncompiled 42 crates\n\none warning\n```")))) ((#1# (content #2# (text . "permission denied")))) ((#1# (content #2# (text . "file contents\nsecond line")))) ((#1# (content #2# (text . "42 matches (truncated)")))) ((#1# (content #2# (text . "7 results")))) nil)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn cursor_preserves_server_content_instead_of_replacing_it_with_raw_output() {
    let elisp_form = r##"
(let ((notification
       '((method . "session/update")
         (params
          (update
           (sessionUpdate . "tool_call_update")
           (status . "completed")
           (content . (((type . "content")
                        (content (type . "text")
                                 (text . "authoritative server text")))))
           (rawOutput (stdout . "inferior fallback")))))))
  (agent-shell-cursor--notification-adapter
   :acp-notification notification)
  notification)
"##;
    let expect = expect![[
        r#"OK ((method . "session/update") (params (update (sessionUpdate . "tool_call_update") (status . "completed") (content ((type . "content") (content (type . "text") (text . "authoritative server text")))) (rawOutput (stdout . "inferior fallback")))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn agent_catalog_rebuilds_maker_entries_but_preserves_concrete_custom_configs() {
    let elisp_form = r##"
(unwind-protect
    (progn
      (setq agent-shell-test-config-maker-calls 0)
      (let ((agent-shell-agent-configs
             (list
              (lambda ()
                (setq agent-shell-test-config-maker-calls
                      (1+ agent-shell-test-config-maker-calls))
                (agent-shell-make-agent-config
                 :identifier 'dynamic
                 :mode-line-name
                 (format "Dynamic %d"
                         agent-shell-test-config-maker-calls)
                 :buffer-name "Dynamic"
                 :shell-prompt "Dynamic> "
                 :shell-prompt-regexp "Dynamic> "
                 :install-instructions "local"))
              '((:identifier . concrete)
                (:mode-line-name . "Concrete")))))
        (list
         (mapcar (lambda (config)
                   (list (map-elt config :identifier)
                         (map-elt config :mode-line-name)))
                 (agent-shell--resolved-agent-configs))
         (mapcar (lambda (config)
                   (list (map-elt config :identifier)
                         (map-elt config :mode-line-name)))
                 (agent-shell--resolved-agent-configs))
         agent-shell-test-config-maker-calls)))
  (makunbound 'agent-shell-test-config-maker-calls))
"##;
    let expect = expect![[
        r#"OK (((dynamic "Dynamic 1") (concrete "Concrete")) ((dynamic "Dynamic 2") (concrete "Concrete")) 2)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}
