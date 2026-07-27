use expect_test::expect;

use super::assert_acp_parity;

#[test]
fn acp_initialize_request_covers_client_info_and_all_capability_combinations() {
    let elisp_form = r##"(list
         (acp-make-initialize-request
          :protocol-version 1)
         (acp-make-initialize-request
          :protocol-version 2
          :client-info
          '((name . "neomacs")
            (title . "Neo")
            (version . "1.0"))
          :read-text-file-capability t
          :write-text-file-capability t)
         (acp-make-initialize-request
          :protocol-version 3
          :read-text-file-capability t)
         (acp-make-initialize-request
          :protocol-version 4
          :write-text-file-capability t))"##;
    let expect = expect![[
        r#"OK ((#1=(:method . "initialize") (:params (protocolVersion . 1) (clientCapabilities (fs (readTextFile . :false) (writeTextFile . :false))))) (#1# (:params (clientInfo (name . "neomacs") (title . "Neo") (version . "1.0")) (protocolVersion . 2) (clientCapabilities (fs (readTextFile . t) (writeTextFile . t))))) (#1# (:params (protocolVersion . 3) (clientCapabilities (fs (readTextFile . t) (writeTextFile . :false))))) (#1# (:params (protocolVersion . 4) (clientCapabilities (fs (readTextFile . :false) (writeTextFile . t))))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_authenticate_and_session_prompt_requests_cover_optional_and_vectorized_values() {
    let elisp_form = r##"(list
         (acp-make-authenticate-request
          :method-id "oauth")
         (acp-make-authenticate-request
          :method-id "token"
          :method
          '((name . "API")))
         (acp-make-session-prompt-request
          :session-id "s-1"
          :prompt "Hi ✓")
         (acp-make-session-prompt-request
          :session-id "s-empty"
          :prompt ""))"##;
    let expect = expect![[
        r#"OK ((#1=(:method . "authenticate") (:params (methodId . "oauth"))) (#1# (:params (methodId . "token") (authMethod (name . "API")))) (#2=(:method . "session/prompt") (:params (sessionId . "s-1") (prompt . [72 105 32 10003]))) (#2# (:params (sessionId . "s-empty") (prompt . []))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_session_new_resume_fork_load_and_list_requests_normalize_cwd_and_defaults() {
    let elisp_form = r##"(let ((default-directory
                "/fixture/base/"))
         (list
          (acp-make-session-new-request
           :cwd "./project/")
          (acp-make-session-new-request
           :cwd "/fixture/absolute/"
           :mcp-servers
           '(((name . "one")))
           :meta
           '((systemPrompt . "append")))
          (acp-make-session-resume-request
           :session-id "resume"
           :cwd "./resume/")
          (acp-make-session-resume-request
           :session-id "resume-full"
           :cwd "/fixture/r"
           :mcp-servers
           '(((name . "two")))
           :meta
           '((key . value)))
          (acp-make-session-fork-request
           :session-id "fork"
           :cwd "./fork/")
          (acp-make-session-fork-request
           :session-id "fork-full"
           :cwd "/fixture/f"
           :mcp-servers
           '(((name . "fork-server")))
           :meta
           '((forked . t)))
          (acp-make-session-list-request
           :cwd "./list/")
          (acp-make-session-load-request
           :session-id "load"
           :cwd "./load/")
          (acp-make-session-load-request
           :session-id "load-full"
           :cwd "/fixture/l"
           :mcp-servers
           '(((name . "three")))
           :meta
           '((x . 1)))))"##;
    let expect = expect![[
        r#"OK ((#1=(:method . "session/new") (:params (cwd . "/fixture/base/project") (mcpServers . []))) (#1# (:params (cwd . "/fixture/absolute") (mcpServers ((name . "one"))) (_meta (systemPrompt . "append")))) (#2=(:method . "session/resume") (:params (sessionId . "resume") (cwd . "/fixture/base/resume") (mcpServers . []))) (#2# (:params (sessionId . "resume-full") (cwd . "/fixture/r") (mcpServers ((name . "two"))) (_meta (key . value)))) (#3=(:method . "session/fork") (:params (sessionId . "fork") (cwd . "/fixture/base/fork") (mcpServers . []))) (#3# (:params (sessionId . "fork-full") (cwd . "/fixture/f") (mcpServers ((name . "fork-server"))) (_meta (forked . t)))) ((:method . "session/list") (:params (cwd . "/fixture/base/list"))) (#4=(:method . "session/load") (:params (sessionId . "load") (cwd . "/fixture/base/load") (mcpServers . []))) (#4# (:params (sessionId . "load-full") (cwd . "/fixture/l") (mcpServers ((name . "three"))) (_meta (x . 1)))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_session_mode_model_config_delete_and_cancel_requests_match_exact_shapes() {
    let elisp_form = r##"(list
         (acp-make-session-set-mode-request
          :session-id "session"
          :mode-id "plan")
         (acp-make-session-set-model-request
          :session-id "session"
          :model-id "haiku")
         (acp-make-session-set-config-option-request
          :session-id "session"
          :config-id "thinking"
          :value "high")
         (acp-make-session-delete-request
          :session-id "session")
         (acp-make-session-cancel-notification
          :session-id "session")
         (acp-make-session-cancel-notification
          :session-id "session"
          :reason "user"))"##;
    let expect = expect![[
        r#"OK (((:method . "session/set_mode") (:params (sessionId . "session") (modeId . "plan"))) ((:method . "session/set_model") (:params (sessionId . "session") (modelId . "haiku"))) ((:method . "session/set_config_option") (:params (sessionId . "session") (configId . "thinking") (value . "high"))) ((:method . "session/delete") (:params (sessionId . "session"))) (#1=(:method . "session/cancel") (:params (sessionId . "session"))) (#1# (:params (sessionId . "session") (reason . "user"))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_permission_and_filesystem_responses_cover_success_error_and_cancelled_shapes() {
    let elisp_form = r##"(list
         (acp-make-session-request-permission-response
          :request-id 7
          :option-id "allow")
         (acp-make-session-request-permission-response
          :request-id 8
          :cancelled t)
         (acp-make-fs-read-text-file-response
          :request-id 9
          :content "text")
         (acp-make-fs-read-text-file-response
          :request-id 10
          :error
          '((code . -1)
            (message . "no")))
         (acp-make-fs-write-text-file-response
          :request-id 11)
         (acp-make-fs-write-text-file-response
          :request-id 12
          :error
          '((code . -2))))"##;
    let expect = expect![[
        r#"OK (((:request-id . 7) (:result (outcome (outcome . "selected") (optionId . "allow")))) ((:request-id . 8) (:result (outcome (outcome . "cancelled")))) ((:request-id . 9) (:result (content . "text"))) ((:request-id . 10) (:error (code . -1) (message . "no"))) ((:request-id . 11) (:result)) ((:request-id . 12) (:error (code . -2))))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_error_and_internal_error_builders_preserve_code_message_data_and_order() {
    let elisp_form = r##"(list
         (acp-make-error
          :code 0
          :message "zero")
         (acp-make-error
          :code -32600
          :message "bad"
          :data
          '((detail . "x")))
         (acp--make-internal-error
          "local failure"))"##;
    let expect = expect![[
        r#"OK (((code . 0) (message . "zero")) ((code . -32600) (message . "bad") (data (detail . "x"))) ((code . -32603) (message . "local failure")))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_constructor_required_argument_and_exclusivity_errors_match_exactly() {
    let elisp_form = r##"(cl-labels
         ((capture
           (function)
           (condition-case error
               (progn
                 (funcall function)
                 'no-error)
             (error
              (list
               (car error)
               (cadr error))))))
         (mapcar
          #'capture
          (list
           (lambda ()
             (acp-make-initialize-request))
           (lambda ()
             (acp-make-authenticate-request))
           (lambda ()
             (acp-make-session-new-request))
           (lambda ()
             (acp-make-session-prompt-request
              :prompt "x"))
           (lambda ()
             (acp-make-session-prompt-request
              :session-id "s"))
           (lambda ()
             (acp-make-session-set-mode-request
              :mode-id "m"))
           (lambda ()
             (acp-make-session-set-mode-request
              :session-id "s"))
           (lambda ()
             (acp-make-session-set-model-request
              :model-id "m"))
           (lambda ()
             (acp-make-session-set-model-request
              :session-id "s"))
           (lambda ()
             (acp-make-session-set-config-option-request
              :config-id "c"
              :value "v"))
           (lambda ()
             (acp-make-session-set-config-option-request
              :session-id "s"
              :value "v"))
           (lambda ()
             (acp-make-session-set-config-option-request
              :session-id "s"
              :config-id "c"))
           (lambda ()
             (acp-make-session-resume-request
              :cwd "/x"))
           (lambda ()
             (acp-make-session-resume-request
              :session-id "s"))
           (lambda ()
             (acp-make-session-fork-request
              :cwd "/x"))
           (lambda ()
             (acp-make-session-fork-request
              :session-id "s"))
           (lambda ()
             (acp-make-session-list-request))
           (lambda ()
             (acp-make-session-load-request
              :cwd "/x"))
           (lambda ()
             (acp-make-session-load-request
              :session-id "s"))
           (lambda ()
             (acp-make-session-delete-request))
           (lambda ()
             (acp-make-session-request-permission-response
              :request-id 1
              :option-id "yes"
              :cancelled t))
           (lambda ()
             (acp-make-session-request-permission-response
              :request-id 1))
           (lambda ()
             (acp-make-session-request-permission-response
              :option-id "yes"))
           (lambda ()
             (acp-make-fs-read-text-file-response
              :content "x"))
           (lambda ()
             (acp-make-fs-read-text-file-response
              :request-id 1
              :content "x"
              :error "y"))
           (lambda ()
             (acp-make-fs-read-text-file-response
              :request-id 1
              :content ""))
           (lambda ()
             (acp-make-fs-read-text-file-response
              :request-id 1))
           (lambda ()
             (acp-make-fs-write-text-file-response))
           (lambda ()
             (acp-make-error
              :message "x"))
           (lambda ()
             (acp-make-error
              :code 1))
           (lambda ()
             (acp-make-session-cancel-notification)))))"##;
    let expect = expect![[
        r#"OK ((error ":protocol-version is required") (error ":method-id is required") (error ":cwd is required") (error ":session-id is required") (error ":prompt is required") (error ":session-id is required") (error ":mode-id is required") (error ":session-id is required") (error ":model-id is required") (error ":session-id is required") (error ":config-id is required") (error ":value is required") (error ":session-id is required") (error ":cwd is required") (error ":session-id is required") (error ":cwd is required") (error ":cwd is required") (error ":session-id is required") (error ":cwd is required") (error ":session-id is required") (error "Choose :option-id or :cancelled Not both") (error "Must specify either :option-id or :cancelled") (error ":request-id is required") (error ":request-id is required") (error "Either :content or :error but not both") no-error (error "Either :content or :error is required") (error ":request-id is required") (error ":code is required") (error ":message is required") (error ":session-id is required"))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}

#[test]
fn acp_message_request_resolution_and_instance_counter_boundaries_match() {
    let elisp_form = r##"(let ((client
                '((:pending-requests
                   (2
                    (:value . found)))))
               (acp-instance-count 40))
         (list
          (acp--make-message
           :json "{\"x\":1}"
           :object
           '((x . 1)))
          (acp--make-message)
          (acp--request-resolver
           :client client
           :id 2)
          (acp--request-resolver
           :client client
           :id 3)
          (acp--increment-instance-count)
          acp-instance-count
          (let ((acp-instance-count
                 most-positive-fixnum))
            (list
             (acp--increment-instance-count)
             acp-instance-count))))"##;
    let expect = expect![[
        r#"OK (((:object (x . 1)) (:json . "{\"x\":1}")) ((:object) (:json)) ((:value . found)) nil 41 41 (0 0))"#
    ]];
    assert_acp_parity(elisp_form, expect);
}
