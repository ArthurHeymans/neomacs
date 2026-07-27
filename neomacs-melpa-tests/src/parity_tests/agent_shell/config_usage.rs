use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn normalizes_a_real_multi_option_acp_configuration() {
    let elisp_form = r##"
(agent-shell--normalize-config-options
 [((id . "provider")
    (name . "Provider")
    (description . "Select the account backend")
    (category . "model")
    (type . "select")
    (currentValue . "openai")
    (options . [((value . "openai") (name . "OpenAI") (description . "Subscription"))
                ((value . "local") (name . "Local") (description . nil))]))
   ((id . "model")
    (name . "Model")
    (category . "model")
    (type . "select")
    (currentValue . "gpt-5.5")
    (options . [((value . "gpt-5.5") (name . "GPT-5.5")
                 (description . "Deep coding model"))]))
   ((id . "thought_level")
    (name . "Reasoning")
    (category . "thought_level")
    (type . "select")
    (currentValue . "high")
    (options . [((value . "medium") (name . "Medium"))
                ((value . "high") (name . "High"))]))])
"##;
    let expect = expect![[
        r#"OK (((:id . "provider") (:name . "Provider") (:description . "Select the account backend") (:category . "model") (:type . "select") (:current-value . "openai") (:options ((:value . "openai") (:name . "OpenAI") (:description . "Subscription")) ((:value . "local") (:name . "Local") (:description)))) ((:id . "model") (:name . "Model") (:description) (:category . "model") (:type . "select") (:current-value . "gpt-5.5") (:options ((:value . "gpt-5.5") (:name . "GPT-5.5") (:description . "Deep coding model")))) ((:id . "thought_level") (:name . "Reasoning") (:description) (:category . "thought_level") (:type . "select") (:current-value . "high") (:options ((:value . "medium") (:name . "Medium") (:description)) ((:value . "high") (:name . "High") (:description)))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn resolves_duplicate_categories_to_the_semantic_option() {
    let elisp_form = r##"
(let* ((options
        (agent-shell--normalize-config-options
         [((id . "provider") (name . "Provider") (category . "model")
            (type . "select") (currentValue . "openai")
            (options . [((value . "openai") (name . "OpenAI"))]))
           ((id . "model") (name . "Model") (category . "model")
            (type . "select") (currentValue . "gpt-5.5")
            (options . [((value . "gpt-5.5") (name . "GPT-5.5"))
                        ((value . "sonnet") (name . "Sonnet"))]))
           ((id . "approval") (name . "Approval") (category . "mode")
            (type . "select") (currentValue . "ask")
            (options . [((value . "ask") (name . "Ask"))]))]))
       (state `((:config-options . ,options))))
  (list
   (map-elt (agent-shell--config-option-by-category state "model") :id)
   (map-elt (agent-shell--config-option-by-category state "mode") :id)
   (agent-shell--current-model-id state)
   (agent-shell--current-mode-id state)
   (agent-shell--get-available-models state)))
"##;
    let expect = expect![[
        r#"OK ("model" "approval" "gpt-5.5" "ask" (((:model-id . "gpt-5.5") (:name . "GPT-5.5") (:description)) ((:model-id . "sonnet") (:name . "Sonnet") (:description))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn saves_and_updates_config_in_session_state_without_losing_other_fields() {
    let elisp_form = r##"
(let ((state '((:session . ((:id . "session-42") (:title . "Parity review")))
               (:config-options . nil)
               (:usage . ((:total-tokens . 12))))))
  (agent-shell--save-config-options
   :state state
   :acp-config-options
   [((id . "mode") (name . "Mode") (category . "mode")
      (type . "select") (currentValue . "ask")
      (options . [((value . "ask") (name . "Ask"))
                  ((value . "code") (name . "Code"))]))])
  (agent-shell--config-option-set-value
   :state state :config-id "mode" :value "code")
  state)
"##;
    let expect = expect![[
        r#"OK ((:session (:config-options . #1=(((:id . "mode") (:name . "Mode") (:description) (:category . "mode") (:type . "select") (:current-value . "code") (:options ((:value . "ask") (:name . "Ask") (:description)) ((:value . "code") (:name . "Code") (:description)))))) (:id . "session-42") (:title . "Parity review")) (:config-options . #1#) (:usage (:total-tokens . 12)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn converts_config_values_for_legacy_model_and_mode_consumers() {
    let elisp_form = r##"
(let ((option
       '((:id . "model")
         (:current-value . "sonnet")
         (:options . (((:value . "sonnet") (:name . "Sonnet")
                       (:description . "Fast daily work"))
                      ((:value . "opus") (:name . "Opus")
                       (:description . "Deep analysis")))))))
  (list
   (agent-shell--config-option-as-models option)
   (agent-shell--config-option-as-modes option)
   (agent-shell--config-option-value-name option "opus")
   (agent-shell--config-option-value-name option "unknown")))
"##;
    let expect = expect![[
        r#"OK ((((:model-id . "sonnet") (:name . "Sonnet") (:description . "Fast daily work")) ((:model-id . "opus") (:name . "Opus") (:description . "Deep analysis"))) (((:id . "sonnet") (:name . "Sonnet") (:description . "Fast daily work")) ((:id . "opus") (:name . "Opus") (:description . "Deep analysis"))) "Opus" "unknown")"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn formats_selectable_config_for_a_real_shell_header() {
    let elisp_form = r##"
(let* ((options
        (agent-shell--normalize-config-options
         [((id . "mode") (name . "Approval mode")
            (description . "Controls whether edits need confirmation")
            (category . "mode") (type . "select") (currentValue . "ask")
            (options . [((value . "ask") (name . "Ask first"))
                        ((value . "auto") (name . "Automatic"))]))
           ((id . "budget") (name . "Budget") (type . "number")
            (currentValue . 120000))]))
       (rendered
        (agent-shell--format-available-config-options
         (agent-shell--select-config-options
          `((:config-options . ,options))))))
  (list (substring-no-properties rendered)
        (get-text-property 0 'font-lock-face rendered)
        (get-text-property
         (string-match "current:" rendered)
         'font-lock-face rendered)))
"##;
    let expect = expect![[
        r#"OK ("Approval mode (id: mode)\ncurrent: Ask first\nControls whether edits need confirmation" agent-shell-list-name agent-shell-list-value)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn accumulates_prompt_and_notification_usage_into_one_session_record() {
    let elisp_form = r##"
(let ((state '((:usage . ((:total-tokens . 0)
                          (:input-tokens . 0)
                          (:output-tokens . 0))))))
  (agent-shell--save-usage
   :state state
   :acp-usage '((totalTokens . 15342)
                (inputTokens . 12000)
                (outputTokens . 2410)
                (thoughtTokens . 932)
                (cachedReadTokens . 8000)
                (cachedWriteTokens . 400)))
  (agent-shell--update-usage-from-notification
   :state state
   :acp-update '((used . 62500)
                 (size . 200000)
                 (cost . ((amount . 1.375) (currency . "USD")))))
  (map-elt state :usage))
"##;
    let expect = expect![""];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn formats_usage_for_inline_and_multiline_real_ui_surfaces() {
    let elisp_form = r##"
(let ((usage '((:total-tokens . 15342)
               (:input-tokens . 12000)
               (:output-tokens . 2410)
               (:thought-tokens . 932)
               (:cached-read-tokens . 8000)
               (:context-used . 62500)
               (:context-size . 200000)
               (:cost-amount . 1.375)
               (:cost-currency . "USD"))))
  (list
   (substring-no-properties (agent-shell--format-usage usage))
   (substring-no-properties (agent-shell--format-usage usage t))
   (mapcar #'agent-shell--format-number-compact
           '(0 999 1000 15342 1500000 2500000000))
   (agent-shell--usage-has-data-p usage)))
"##;
    let expect = expect![[
        r#"OK ("Context: 62k/200k (31.2%) Tokens: 12k in · 2k out · 932 thought · 8k cached (15k total) Cost: USD1.38" " Context: 62k/200k (31.2%)\n  Tokens: 12k in · 2k out · 932 thought · 8k cached (15k total)\n    Cost: USD1.38" ("0" "999" "1k" "15k" "2m" "2b") t)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn context_indicators_cover_thresholds_and_preserve_help_text() {
    let elisp_form = r##"
(mapcar
 (lambda (used)
   (let* ((usage `((:context-used . ,used) (:context-size . 200000)
                   (:input-tokens . 1000) (:output-tokens . 500)))
          (bar (agent-shell--context-usage-indicator-bar
                usage used 200000))
          (detail (agent-shell--context-usage-indicator-detailed
                   usage used 200000)))
     (list (and bar (substring-no-properties bar))
           (and bar (get-text-property 0 'face bar))
           (substring-no-properties detail)
           (get-text-property 0 'face detail)
           (get-text-property 0 'help-echo detail))))
 '(0 50000 120000 170000 200000))
"##;
    let expect = expect![[
        r#"OK ((nil nil "0/200k (0%%)" agent-shell-success #("Context: 0/200k (0.0%) Tokens: 1k in · 500 out Cost: $0.00" 0 9 (font-lock-face agent-shell-secondary face agent-shell-secondary) 23 31 (font-lock-face agent-shell-secondary face agent-shell-secondary) 47 53 (font-lock-face agent-shell-secondary face agent-shell-secondary))) ("▂" agent-shell-success "50k/200k (25%%)" agent-shell-success #("Context: 50k/200k (25.0%) Tokens: 1k in · 500 out Cost: $0.00" 0 9 (font-lock-face agent-shell-secondary face agent-shell-secondary) 26 34 (font-lock-face agent-shell-secondary face agent-shell-secondary) 50 56 (font-lock-face agent-shell-secondary face agent-shell-secondary))) ("▄" agent-shell-warning "120k/200k (60%%)" agent-shell-warning #("Context: 120k/200k (60.0%) Tokens: 1k in · 500 out Cost: $0.00" 0 9 (font-lock-face agent-shell-secondary face agent-shell-secondary) 27 35 (font-lock-face agent-shell-secondary face agent-shell-secondary) 51 57 (font-lock-face agent-shell-secondary face agent-shell-secondary))) ("▆" agent-shell-error "170k/200k (85%%)" agent-shell-error #("Context: 170k/200k (85.0%) Tokens: 1k in · 500 out Cost: $0.00" 0 9 (font-lock-face agent-shell-secondary face agent-shell-secondary) 27 35 (font-lock-face agent-shell-secondary face agent-shell-secondary) 51 57 (font-lock-face agent-shell-secondary face agent-shell-secondary))) ("█" agent-shell-error "200k/200k (100%%)" agent-shell-error #("Context: 200k/200k (100.0%) Tokens: 1k in · 500 out Cost: $0.00" 0 9 (font-lock-face agent-shell-secondary face agent-shell-secondary) 28 36 (font-lock-face agent-shell-secondary face agent-shell-secondary) 52 58 (font-lock-face agent-shell-secondary face agent-shell-secondary))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn status_styles_keep_text_and_semantic_faces_in_sync() {
    let elisp_form = r##"
(mapcar
 (lambda (status)
   (let ((unicode
          (agent-shell--unicode-icons-status-kind-label status "execute"))
         (plain
          (agent-shell--plain-colored-status-kind-label status "search")))
     (list status
           (agent-shell--status-config status)
           (substring-no-properties unicode)
           (get-text-property 0 'font-lock-face unicode)
           (substring-no-properties plain)
           (get-text-property 0 'font-lock-face plain))))
 '("pending" "in_progress" "completed" "failed" "mystery"))
"##;
    let expect = expect![[
        r#"OK (("pending" ((:label . "wait") (:icon . "…") (:face . agent-shell-pending)) "… run" agent-shell-pending "[wait][find]" agent-shell-pending) ("in_progress" ((:label . "busy") (:icon . "…") (:face . agent-shell-warning)) "… run" agent-shell-warning "[busy][find]" agent-shell-warning) ("completed" ((:label . "done") (:icon . "✓") (:face . agent-shell-success)) "✓ run" agent-shell-success "[done][find]" agent-shell-success) ("failed" ((:label . "error") (:icon . "✗") (:face . agent-shell-error)) "✗ run" agent-shell-error "[error][find]" agent-shell-error) ("mystery" ((:label . "unknown") (:icon . "?") (:face . agent-shell-warning)) "? run" agent-shell-warning "[unknown][find]" agent-shell-warning))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}
