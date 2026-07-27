use expect_test::expect;

use super::assert_aidev_mode_parity;

#[test]
fn aidev_mode_start_chat_builds_real_text_buffer_local_history_and_first_response() {
    let elisp_form = r##"(let ((origin (current-buffer))
               (aidev-provider 'claude)
               (aidev-chat-buffer-name
                "*Aidev Frozen Chat*")
               generated
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'aidev---claude)
                   (lambda
                     (messages system model)
                     (push
                      (list messages system model)
                      calls)
                     "Use `mapcar` with the named function.")))
               (let ((result
                      (aidev-start-chat
                       "How should I transform each row?")))
                 (setq generated
                       (current-buffer))
                 (list
                  result
                  (buffer-name generated)
                  major-mode
                  aidev-chat-mode
                  aidev-mode
                  aidev-chat-system-prompt-used
                  aidev-chat-messages
                  (buffer-string)
                  (point)
                  (= (point) (point-max))
                  (nreverse calls))))
           (when (buffer-live-p generated)
             (kill-buffer generated))
           (when (buffer-live-p origin)
             (set-buffer origin))))"##;
    let expect = expect![[
        r#"OK (84 "*Aidev Frozen Chat*" text-mode t nil "You are a helpful assistant. Respond concisely and helpfully to the user's messages." ((("role" . "assistant") ("content" . "Use `mapcar` with the named function.")) #1=(("role" . "user") ("content" . "How should I transform each row?"))) "User: How should I transform each row?\n\nAI: Use `mapcar` with the named function.\n\n" 84 t (((#1#) "You are a helpful assistant. Respond concisely and helpfully to the user's messages." "deepseek-coder-v2:latest")))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_chat_preserves_complete_multi_turn_conversation_for_provider() {
    let elisp_form = r##"(let ((origin (current-buffer))
               (aidev-provider 'claude)
               (aidev-chat-buffer-name
                "*Aidev Multi Turn*")
               (responses
                '("First answer."
                  "Second answer with context."))
               generated
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'aidev---claude)
                   (lambda
                     (messages system model)
                     (push
                      (list messages system model)
                      calls)
                     (prog1
                         (car responses)
                       (setq responses
                             (cdr responses))))))
               (aidev-start-chat
                "First question")
               (setq generated
                     (current-buffer))
               (goto-char (point-max))
               (insert
                aidev-chat-user-prompt-prefix
                "Follow-up question"
                aidev-chat-separator)
               (aidev-chat-send-message
                "Follow-up question")
               (list
                (buffer-string)
                aidev-chat-messages
                aidev-chat-system-prompt-used
                (nreverse calls)
                responses))
           (when (buffer-live-p generated)
             (kill-buffer generated))
           (when (buffer-live-p origin)
             (set-buffer origin))))"##;
    let expect = expect![[
        r#"OK ("User: First question\n\nAI: First answer.\n\nUser: Follow-up question\n\nAI: Second answer with context.\n\n" ((#1=("role" . "assistant") ("content" . "Second answer with context.")) #5=(#2=("role" . "user") ("content" . "Follow-up question")) #4=(#1# ("content" . "First answer.")) #3=(#2# ("content" . "First question"))) "You are a helpful assistant. Respond concisely and helpfully to the user's messages." (((#3#) "You are a helpful assistant. Respond concisely and helpfully to the user's messages." "deepseek-coder-v2:latest") ((#3# #4# #5#) "You are a helpful assistant. Respond concisely and helpfully to the user's messages." "deepseek-coder-v2:latest")) nil)"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_chat_region_send_appends_selected_text_and_response_at_buffer_end() {
    let elisp_form = r##"(let ((aidev-provider 'openai)
               calls)
         (cl-letf
             (((symbol-function
                'aidev---openai)
               (lambda
                 (messages system model)
                 (push
                  (list messages system model)
                  calls)
                 "Selected code is already concise.")))
           (with-temp-buffer
             (text-mode)
             (aidev-chat-mode 1)
             (insert
              "Notes before selection\n"
              "(mapcar #'1+ values)\n"
              "Notes after selection\n")
             (let ((transient-mark-mode t))
               (goto-char (point-min))
               (search-forward "(mapcar")
               (goto-char (match-beginning 0))
               (push-mark
                (progn
                  (search-forward "values)")
                  (point))
                t t)
               (let ((result
                      (aidev-chat-send-message
                       (buffer-substring-no-properties
                        (region-beginning)
                        (region-end)))))
                 (list
                  result
                  (buffer-string)
                  aidev-chat-messages
                  (= (point) (point-max))
                  (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (114 "Notes before selection\n(mapcar #'1+ values)\nNotes after selection\nUser: \n\nAI: Selected code is already concise.\n\n" ((("role" . "assistant") ("content" . "Selected code is already concise.")) #1=(("role" . "user") ("content" . ""))) t (((#1#) "You are a helpful assistant. Respond concisely and helpfully to the user's messages." "deepseek-coder-v2:latest")))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_send_buffer_contents_uses_only_text_before_point_then_updates_history() {
    let elisp_form = r##"(let ((aidev-provider 'ollama)
               calls)
         (cl-letf
             (((symbol-function
                'aidev---ollama)
               (lambda
                 (messages system model)
                 (push
                  (list messages system model)
                  calls)
                 "Buffer prefix received.")))
           (with-temp-buffer
             (text-mode)
             (aidev-chat-mode 1)
             (insert
              "Architecture decision\n"
              "- preserve API\n"
              "- add tests\n"
              "DO NOT SEND THIS LINE")
             (goto-char (point-min))
             (search-forward "- add tests\n")
             (let ((sent-point
                    (point))
                   (result
                    (aidev-chat-send-buffer-contents)))
               (list
                result
                sent-point
                (buffer-string)
                aidev-chat-messages
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (102 50 "Architecture decision\n- preserve API\n- add tests\nDO NOT SEND THIS LINE\n\nAI: Buffer prefix received.\n\n" ((("role" . "assistant") ("content" . "Buffer prefix received.")) #1=(("role" . "user") ("content" . "Architecture decision\n- preserve API\n- add tests\n"))) (((#1#) "You are a helpful assistant. Respond concisely and helpfully to the user's messages." "deepseek-coder-v2:latest")))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_chat_inserts_missing_custom_separator_but_does_not_duplicate_existing_one() {
    let elisp_form = r##"(let ((aidev-provider 'claude)
               (aidev-chat-user-prompt-prefix "Developer> ")
               (aidev-chat-ai-response-prefix "Assistant> ")
               (aidev-chat-separator " <END> ")
               (responses
                '("first" "second")))
         (cl-letf
             (((symbol-function
                'aidev---claude)
               (lambda (&rest _)
                 (prog1
                     (car responses)
                   (setq responses
                         (cdr responses))))))
           (with-temp-buffer
             (text-mode)
             (insert "Developer> first")
             (aidev-chat-send-message "first")
             (let ((after-missing
                    (buffer-string)))
               (insert
                "Developer> second"
                aidev-chat-separator)
               (aidev-chat-send-message "second")
               (list
                after-missing
                (buffer-string)
                aidev-chat-messages
                responses)))))"##;
    let expect = expect![[
        r#"OK ("Developer> first <END> Assistant> first <END> " "Developer> first <END> Assistant> first <END> Developer> second <END> Assistant> second <END> " ((#1=("role" . "assistant") ("content" . "second")) (#2=("role" . "user") ("content" . "second")) (#1# ("content" . "first")) (#2# ("content" . "first"))) nil)"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_chat_rejects_non_text_buffers_before_contacting_provider() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'aidev---claude)
               (lambda (&rest arguments)
                 (push arguments calls)
                 "unexpected")))
           (with-temp-buffer
             (emacs-lisp-mode)
             (list
              (condition-case error-data
                  (aidev-chat-send-message
                   "Should fail")
                (error error-data))
              major-mode
              aidev-chat-messages
              (buffer-string)
              calls))))"##;
    let expect = expect![[
        r#"OK ((error "Can only send messages from the chat buffer") emacs-lisp-mode nil "" nil)"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}
