use expect_test::expect;

use super::{assert_aidev_mode_autoload_parity, assert_aidev_mode_parity};

#[test]
fn aidev_mode_defaults_custom_metadata_and_dependency_activation_match() {
    let elisp_form = r##"(list
         (featurep 'aidev-mode)
         (featurep 'request)
         (featurep 'json)
         (featurep 'url)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value symbol)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (custom-variable-p symbol)))
          '(aidev-default-model
            aidev-provider
            aidev-ollama-url
            aidev-chat-system-prompt
            aidev-chat-buffer-name
            aidev-chat-user-prompt-prefix
            aidev-chat-ai-response-prefix
            aidev-chat-separator))
         aidev---ollama-default-url
         (getenv "AIDEV_OLLAMA_ADDRESS")
         (list
          (default-boundp
           'aidev-chat-messages)
          (default-value
           'aidev-chat-messages)
          (local-variable-if-set-p
           'aidev-chat-messages)
          (default-boundp
           'aidev-chat-system-prompt-used)
          (default-value
           'aidev-chat-system-prompt-used)
          (local-variable-if-set-p
           'aidev-chat-system-prompt-used)))"##;
    let expect = expect![[
        r#"OK (t t t t ((aidev-default-model "deepseek-coder-v2:latest" string nil ((funcall #'#[nil ("deepseek-coder-v2:latest") #1=(t)]))) (aidev-provider claude (choice (const :tag "Ollama" ollama) (const :tag "OpenAI" openai) (const :tag "Claude" claude)) nil ((funcall #'#[nil ('claude) #1#]))) (aidev-ollama-url nil (choice (string :tag "URL") (const :tag "Auto-detect" nil)) nil ((funcall #'#[nil (nil) #1#]))) (aidev-chat-system-prompt "You are a helpful assistant. Respond concisely and helpfully to the user's messages." string nil ((funcall #'#[nil ("You are a helpful assistant. Respond concisely and helpfully to the user's messages.") #1#]))) (aidev-chat-buffer-name "*AIdev Chat*" string nil ((funcall #'#[nil ("*AIdev Chat*") #1#]))) (aidev-chat-user-prompt-prefix "User: " string nil ((funcall #'#[nil ("User: ") #1#]))) (aidev-chat-ai-response-prefix "AI: " string nil ((funcall #'#[nil ("AI: ") #1#]))) (aidev-chat-separator "\n\n" string nil ((funcall #'#[nil ("\n\n") #1#])))) "http://frozen-ollama.invalid:11434" "http://frozen-ollama.invalid:11434" (t nil t t nil t))"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_complete_callable_surface_arglists_commands_and_autoload_status_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (macrop symbol)
            (autoloadp
             (symbol-function symbol))))
         '(aidev-chat-mode
           aidev-mode
           aidev-global-mode
           aidev-insert-chat
           aidev-refactor-region-with-chat
           aidev-refactor-buffer-with-chat
           aidev-new-buffer-from-chat
           aidev-start-chat
           aidev-chat-send-message
           aidev-chat-send-buffer-contents
           aidev--prepare-system-message
           aidev--prepare-prompt
           aidev--invert-markdown-code
           aidev--strip-markdown-code
           aidev--chat
           aidev---ollama-available
           aidev---ollama
           aidev---decode-utf8-string
           aidev---openai
           aidev---claude))"##;
    let expect = expect![
        "OK ((aidev-chat-mode (&optional arg) t nil nil) (aidev-mode (&optional arg) t nil nil) (aidev-global-mode (&optional arg) t nil nil) (aidev-insert-chat (prompt) t nil nil) (aidev-refactor-region-with-chat (prompt) t nil nil) (aidev-refactor-buffer-with-chat (prompt) t nil nil) (aidev-new-buffer-from-chat (prompt) t nil nil) (aidev-start-chat (prompt) t nil nil) (aidev-chat-send-message (message) t nil nil) (aidev-chat-send-buffer-contents nil t nil nil) (aidev--prepare-system-message (additional-instructions) nil nil nil) (aidev--prepare-prompt (prompt &optional include-region) nil nil nil) (aidev--invert-markdown-code (md-block) nil nil nil) (aidev--strip-markdown-code (md-block) nil nil nil) (aidev--chat (system messages) nil nil nil) (aidev---ollama-available (url) nil nil nil) (aidev---ollama (messages &optional system model) nil nil nil) (aidev---decode-utf8-string (str) nil nil nil) (aidev---openai (messages &optional system model) nil nil nil) (aidev---claude (messages &optional system model) nil nil nil))"
    ];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_keymaps_bind_every_documented_edit_and_chat_command() {
    let elisp_form = r##"(list
         (keymapp aidev-mode-map)
         (mapcar
          (lambda (key)
            (list
             key
             (lookup-key
              aidev-mode-map
              (kbd key))))
          '("C-c C-a i"
            "C-c C-a r"
            "C-c C-a b"
            "C-c C-a n"
            "C-c C-a c"))
         (keymapp aidev-chat-mode-map)
         (lookup-key
          aidev-chat-mode-map
          (kbd "C-c C-c"))
         (assq
          'aidev-mode
          minor-mode-alist)
         (assq
          'aidev-chat-mode
          minor-mode-alist)
         (eq
          (cdr
           (assq
            'aidev-mode
            minor-mode-map-alist))
          aidev-mode-map)
         (eq
          (cdr
           (assq
            'aidev-chat-mode
            minor-mode-map-alist))
          aidev-chat-mode-map))"##;
    let expect = expect![[
        r#"OK (t (("C-c C-a i" aidev-insert-chat) ("C-c C-a r" aidev-refactor-region-with-chat) ("C-c C-a b" aidev-refactor-buffer-with-chat) ("C-c C-a n" aidev-new-buffer-from-chat) ("C-c C-a c" aidev-start-chat)) t aidev-chat-send-buffer-contents (aidev-mode " AIdev") (aidev-chat-mode " AI-Chat") t t)"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_toggle_is_buffer_local_preserves_major_mode_and_reports_state() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (let ((before
                (list
                 major-mode
                 aidev-mode
                 aidev-chat-mode)))
           (let ((enabled
                  (aidev-mode 1))
                 (enabled-message
                  (current-message)))
             (let ((disabled
                    (aidev-mode -1))
                   (disabled-message
                    (current-message)))
               (list
                before
                enabled
                enabled-message
                disabled
                disabled-message
                major-mode
                aidev-mode
                (local-variable-p
                 'aidev-mode))))))"##;
    let expect = expect!["OK ((emacs-lisp-mode nil nil) t nil nil nil emacs-lisp-mode nil t)"];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_chat_mode_toggle_is_independent_from_general_aidev_mode() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (list
          (aidev-chat-mode 1)
          aidev-chat-mode
          aidev-mode
          (lookup-key
           (current-local-map)
           (kbd "C-c C-c"))
          (aidev-mode 1)
          aidev-chat-mode
          aidev-mode
          (aidev-chat-mode -1)
          aidev-chat-mode
          aidev-mode
          major-mode))"##;
    let expect = expect!["OK (t t nil 1 t t t nil nil t text-mode)"];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_global_mode_enables_and_disables_aidev_across_real_buffers() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 " *aidev-project-one*"))
               (second
                (generate-new-buffer
                 " *aidev-project-two*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (emacs-lisp-mode))
               (with-current-buffer second
                 (text-mode))
               (aidev-global-mode 1)
               (let ((enabled
                      (list
                       aidev-global-mode
                       (buffer-local-value
                        'aidev-mode first)
                       (buffer-local-value
                        'aidev-mode second))))
                 (aidev-global-mode -1)
                 (list
                  enabled
                  aidev-global-mode
                  (buffer-local-value
                   'aidev-mode first)
                  (buffer-local-value
                   'aidev-mode second))))
           (aidev-global-mode -1)
           (when (buffer-live-p first)
             (kill-buffer first))
           (when (buffer-live-p second)
             (kill-buffer second))))"##;
    let expect = expect!["OK ((t t t) nil nil nil)"];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_autoload_file_exposes_only_frozen_generated_contract() {
    let elisp_form = r##"(list
         (featurep 'aidev-mode)
         (boundp 'aidev-chat-mode-map)
         (and
          (boundp 'aidev-chat-mode-map)
          (keymapp
           aidev-chat-mode-map))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (and
              (fboundp symbol)
              (autoloadp
               (symbol-function symbol)))
             (commandp symbol)))
          '(aidev-chat-mode
            aidev-mode
            aidev-global-mode
            aidev-insert-chat
            aidev-start-chat))
         (and
          (boundp 'aidev-chat-mode-map)
          (lookup-key
           aidev-chat-mode-map
           (kbd "C-c C-c"))))"##;
    let expect = expect![
        "OK (nil t t ((aidev-chat-mode nil nil nil) (aidev-mode nil nil nil) (aidev-global-mode nil nil nil) (aidev-insert-chat nil nil nil) (aidev-start-chat nil nil nil)) aidev-chat-send-buffer-contents)"
    ];
    assert_aidev_mode_autoload_parity(elisp_form, expect);
}
