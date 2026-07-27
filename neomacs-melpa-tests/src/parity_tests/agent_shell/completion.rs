use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn completion_bounds_distinguish_file_mentions_commands_and_paths() {
    let elisp_form = r##"
(mapcar
 (lambda (case)
   (with-temp-buffer
     (insert (car case))
     (goto-char (point-max))
     (list (agent-shell--completion-bounds (cadr case) (caddr case))
           (point))))
 '(("@src/parity_tests/agent-shell.rs" "[:alnum:]/_.-" 64)
   ("please /resume-session" "[:alnum:]_-" 47)
   ("path/to/file" "[:alnum:]_-" 47)
   ("email@example.org" "[:alnum:]/_.-" 64)
   ("@hyphenated_file.md" "[:alnum:]/_.-" 64)))
"##;
    let expect = expect![
        "OK ((((:start . 2) (:end . 33)) 33) (((:start . 9) (:end . 23)) 23) (nil 13) (nil 18) (((:start . 2) (:end . 20)) 20))"
    ];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn file_completion_uses_a_session_cache_and_preserves_candidate_kinds() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "@src/")
  (let ((calls 0))
    (cl-letf (((symbol-function 'agent-shell--project-files)
               (lambda ()
                 (setq calls (1+ calls))
                 '("src/lib.rs" "src/parity_tests/" "README.md"))))
      (let* ((first (agent-shell--file-completion-at-point))
             (second (agent-shell--file-completion-at-point))
             (kind (plist-get (nthcdr 3 first) :company-kind)))
        (list (seq-take first 3)
              (seq-take second 3)
              calls
              (mapcar kind '("src/lib.rs" "src/parity_tests/")))))))
"##;
    let expect = expect![[
        r#"OK ((2 6 #1=("src/lib.rs" "src/parity_tests/" "README.md")) (2 6 #1#) 1 (file folder))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn command_completion_reads_live_session_commands_and_annotations() {
    let elisp_form = r##"
(let ((shell (generate-new-buffer " *agent-shell-command-source*")))
  (unwind-protect
      (progn
        (with-current-buffer shell
          (setq-local agent-shell--state
                      '((:available-commands
                         . (((name . "review") (description . "Review current changes"))
                            ((name . "resume") (description . "Resume a session")))))))
        (with-temp-buffer
          (setq-local agent-shell-completion--shell-buffer shell)
          (insert "/re")
          (let* ((capf (agent-shell--command-completion-at-point))
                 (annotate (plist-get (nthcdr 3 capf) :annotation-function)))
            (list (seq-take capf 3)
                  (funcall annotate "review")
                  (funcall annotate "resume")
                  (plist-get (nthcdr 3 capf) :exclusive)))))
    (kill-buffer shell)))
"##;
    let expect = expect![[
        r#"OK ((2 4 ("review" "resume")) "  Review current changes" "  Resume a session" t)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn minibuffer_completion_setup_and_cleanup_are_symmetric() {
    let elisp_form = r##"
(let ((shell (generate-new-buffer " *agent-shell-minibuffer-source*")))
  (unwind-protect
      (progn
        (with-current-buffer shell
          (agent-shell-completion-mode 1))
        (with-temp-buffer
          (agent-shell-completion--setup-minibuffer shell)
          (let ((installed
                 (list (eq agent-shell-completion--shell-buffer shell)
                       (memq #'agent-shell--file-completion-at-point
                             completion-at-point-functions)
                       (memq #'agent-shell--command-completion-at-point
                             completion-at-point-functions)
                       (memq #'agent-shell--trigger-completion-at-point
                             post-self-insert-hook))))
            (agent-shell-completion--cleanup-minibuffer)
            (list installed
                  (local-variable-p 'agent-shell-completion--shell-buffer)
                  completion-at-point-functions
                  post-self-insert-hook))))
    (kill-buffer shell)))
"##;
    let expect = expect![
        "OK ((t #1=(agent-shell--file-completion-at-point t) (agent-shell--command-completion-at-point . #1#) (agent-shell--trigger-completion-at-point t)) nil (tags-completion-at-point-function) (electric-indent-post-self-insert-function blink-paren-post-self-insert-function))"
    ];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn completion_exit_appends_exactly_one_space_to_the_draft() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "ask @src/lib.rs")
  (agent-shell--capf-exit-with-space "src/lib.rs" 'finished)
  (agent-shell--capf-exit-with-space "src/lib.rs" 'sole)
  (list (buffer-string) (point)))
"##;
    let expect = expect![[r#"OK ("ask @src/lib.rs  " 18)"#]];
    assert_agent_shell_parity(elisp_form, expect);
}
