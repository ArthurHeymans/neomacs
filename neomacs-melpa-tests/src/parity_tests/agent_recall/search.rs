use expect_test::expect;

use super::assert_agent_recall_parity;

#[test]
fn agent_recall_search_pattern_rendering_quotes_spaces_metacharacters_and_legacy_string() {
    let elisp_form = r##"(list
                      (let ((agent-recall-file-patterns
                             '("*.md" "*.org" "space name.*"
                               "literal'quote")))
                        (list
                         (agent-recall--file-patterns-as-includes)
                         (agent-recall--file-patterns-as-globs)))
                      (let ((agent-recall-file-patterns "*.txt"))
                        (list
                         (agent-recall--file-patterns-as-includes)
                         (agent-recall--file-patterns-as-globs))))"##;
    let expect = expect![[
        r#"OK (("--include=\\*.md --include=\\*.org --include=space\\ name.\\* --include=literal\\'quote" "--glob \\*.md --glob \\*.org --glob space\\ name.\\* --glob literal\\'quote") ("--include=\\*.txt" "--glob \\*.txt"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_grep_backend_builds_exact_command_and_marks_real_result_buffer() {
    let elisp_form = r##"(let ((agent-recall-search-context-lines 4)
                          (agent-recall-file-patterns
                           '("*.md" "*.org"))
                          (agent-recall-auto-transcript-mode t)
                          (find-file-hook nil)
                          command)
                      (unwind-protect
                          (progn
                            (get-buffer-create "*grep*")
                            (cl-letf (((symbol-function 'grep)
                                       (lambda (value)
                                         (setq command value))))
                              (agent-recall--search-via-grep
                               "needle 'quoted'"
                               '("/work/alpha transcripts"
                                 "/work/beta")))
                            (list
                             command
                             (memq
                              #'agent-recall--maybe-enable-from-search
                              find-file-hook)
                             (buffer-local-value
                              'agent-recall--search-buffer-p
                              (get-buffer "*grep*"))))
                        (when-let ((buffer (get-buffer "*grep*")))
                          (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ("grep -rnH -C 4 --include=\\*.md --include=\\*.org -- needle\\ \\'quoted\\' /work/alpha\\ transcripts /work/beta" (agent-recall--maybe-enable-from-search) t)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_deadgrep_backend_reuses_index_symlink_root_and_marks_search_context() {
    let elisp_form = r##"(let ((agent-recall-auto-transcript-mode t)
                          (find-file-hook nil)
                          deadgrep-extra-arguments
                          calls)
                      (cl-letf (((symbol-function
                                 'agent-recall--ensure-symlink-dir)
                                (lambda () "/sandbox/search-root"))
                               ((symbol-function 'deadgrep)
                                (lambda (query directory)
                                  (push
                                   (list query directory
                                         deadgrep-extra-arguments)
                                   calls))))
                        (agent-recall--search-via-deadgrep
                         "atomic index" '("/ignored/one")))
                      (list
                       (nreverse calls)
                       agent-recall--search-buffer-p
                       (memq #'agent-recall--maybe-enable-from-search
                             find-file-hook)))"##;
    let expect = expect![[
        r#"OK ((("atomic index" "/sandbox/search-root" nil)) t (agent-recall--maybe-enable-from-search))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_counsel_backend_builds_following_multi_glob_command_and_dispatches_query() {
    let elisp_form = r##"(let ((agent-recall-file-patterns
                           '("*.md" "*.org"))
                          (agent-recall-auto-transcript-mode nil)
                          counsel-rg-base-command
                          call)
                      (cl-letf (((symbol-function
                                 'agent-recall--ensure-symlink-dir)
                                (lambda () "/sandbox/recall-search"))
                               ((symbol-function 'counsel-rg)
                                (lambda (&rest args)
                                  (setq call
                                        (list args
                                              counsel-rg-base-command)))))
                        (agent-recall--search-via-counsel-rg
                         "resume session" nil))
                      call)"##;
    let expect = expect![[r#"OK (("resume session" "/sandbox/recall-search" "" "Recall: ") nil)"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_ripgrep_backend_extends_arguments_and_enables_opened_transcript() {
    let elisp_form = r##"(setq consult-ripgrep-args
                           "rg --null --line-buffered")
                      (let ((agent-recall-file-patterns
                           '("*.md" "*.org"))
                          (agent-recall-auto-transcript-mode t)
                          (mode-enabled nil)
                          call)
                      (cl-letf (((symbol-function
                                 'agent-recall--ensure-symlink-dir)
                                (lambda () "/sandbox/search"))
                               ((symbol-function 'require)
                                (let ((real-require
                                       (symbol-function 'require)))
                                  (lambda (feature &optional filename noerror)
                                    (if (eq feature 'consult)
                                        t
                                      (funcall real-require
                                               feature filename noerror)))))
                               ((symbol-function 'consult-ripgrep)
                                (lambda (directory query)
                                  (setq call
                                        (list directory query
                                              consult-ripgrep-args))))
                               ((symbol-function 'buffer-file-name)
                                (lambda (&optional _buffer)
                                  "/work/project/.agent-shell/transcripts/a.md"))
                               ((symbol-function
                                 'agent-recall-transcript-mode)
                                (lambda (arg)
                                  (setq mode-enabled arg))))
                        (agent-recall--search-via-consult-ripgrep
                         "cache" nil))
                      (list call mode-enabled))"##;
    let expect = expect![[
        r#"OK (("/sandbox/search" "cache" "rg --null --line-buffered --follow --glob \\*.md --glob \\*.org") 1)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_search_dispatches_every_configured_backend_with_real_index_directories() {
    let elisp_form = r##"(let ((agent-recall--index
                           (let ((table
                                  (make-hash-table :test 'equal)))
                             (puthash "a"
                                      '(:dir "/transcripts/alpha")
                                      table)
                             table))
                          (agent-recall--index-loaded-p t)
                          calls)
                      (cl-letf (((symbol-function
                                 'agent-recall--search-via-grep)
                                (lambda (query dirs)
                                  (push (list 'grep query dirs) calls)))
                               ((symbol-function
                                 'agent-recall--search-via-deadgrep)
                                (lambda (query dirs)
                                  (push (list 'deadgrep query dirs) calls)))
                               ((symbol-function
                                 'agent-recall--search-via-counsel-rg)
                                (lambda (query dirs)
                                  (push (list 'counsel query dirs) calls)))
                               ((symbol-function
                                 'agent-recall--search-via-consult-ripgrep)
                                (lambda (query dirs)
                                  (push (list 'consult query dirs) calls))))
                        (dolist (backend
                                 '(grep deadgrep counsel-rg
                                   consult-ripgrep unknown))
                          (let ((agent-recall-search-function backend))
                            (agent-recall-search
                             (format "query-%s" backend)))))
                      (nreverse calls))"##;
    let expect = expect![[
        r#"OK ((grep "query-grep" ("/transcripts/alpha")) (deadgrep "query-deadgrep" ("/transcripts/alpha")) (counsel "query-counsel-rg" ("/transcripts/alpha")) (consult "query-consult-ripgrep" ("/transcripts/alpha")) (grep "query-unknown" ("/transcripts/alpha")))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_search_live_respects_backend_then_selects_available_fallbacks() {
    let elisp_form = r##"(let ((agent-recall--index
                           (let ((table
                                  (make-hash-table :test 'equal)))
                             (puthash "a"
                                      '(:dir "/transcripts/alpha")
                                      table)
                             table))
                          (agent-recall--index-loaded-p t)
                          calls)
                      (cl-letf (((symbol-function
                                 'agent-recall--search-via-counsel-rg)
                                (lambda (query dirs)
                                  (push (list 'counsel query dirs) calls)))
                               ((symbol-function
                                 'agent-recall--search-via-consult-ripgrep)
                                (lambda (query dirs)
                                  (push (list 'consult query dirs) calls)))
                               ((symbol-function 'counsel-rg)
                                (lambda (&rest _)))
                               ((symbol-function 'consult-ripgrep)
                                (lambda (&rest _)))
                               ((symbol-function 'call-interactively)
                                (lambda (command)
                                  (push (list 'interactive command) calls))))
                        (let ((agent-recall-search-function 'counsel-rg))
                          (agent-recall-search-live))
                        (let ((agent-recall-search-function
                               'consult-ripgrep))
                          (agent-recall-search-live))
                        (let ((agent-recall-search-function 'grep))
                          (agent-recall-search-live))
                        (cl-letf (((symbol-function 'counsel-rg) nil))
                          (let ((agent-recall-search-function 'grep))
                            (agent-recall-search-live)))
                        (cl-letf (((symbol-function 'counsel-rg) nil)
                                  ((symbol-function 'consult-ripgrep) nil))
                          (let ((agent-recall-search-function 'grep))
                            (agent-recall-search-live))))
                      (nreverse calls))"##;
    let expect = expect![[
        r#"OK ((counsel "" ("/transcripts/alpha")) (consult "" ("/transcripts/alpha")) (counsel "" ("/transcripts/alpha")) (consult "" ("/transcripts/alpha")) (interactive agent-recall-search))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_search_open_hook_enables_only_real_transcripts_when_search_buffer_exists() {
    let elisp_form = r##"(let ((search-buffer
                           (generate-new-buffer " *recall-search*"))
                          (transcript-buffer
                           (generate-new-buffer " *recall-transcript*"))
                          (plain-buffer
                           (generate-new-buffer " *recall-plain*"))
                          (agent-recall-auto-transcript-mode t)
                          enabled)
                      (unwind-protect
                          (progn
                            (with-current-buffer search-buffer
                              (setq-local
                               agent-recall--search-buffer-p t))
                            (cl-letf (((symbol-function 'buffer-file-name)
                                       (lambda (&optional buffer)
                                         (if (eq (or buffer
                                                     (current-buffer))
                                                 transcript-buffer)
                                             "/work/.agent-shell/transcripts/a.md"
                                           "/work/README.md")))
                                      ((symbol-function
                                        'agent-recall-transcript-mode)
                                       (lambda (arg)
                                         (push
                                          (list (buffer-name) arg)
                                          enabled))))
                              (with-current-buffer transcript-buffer
                                (agent-recall--maybe-enable-from-search))
                              (with-current-buffer plain-buffer
                                (agent-recall--maybe-enable-from-search)))
                            (nreverse enabled))
                        (kill-buffer search-buffer)
                        (kill-buffer transcript-buffer)
                        (kill-buffer plain-buffer)))"##;
    let expect = expect![[r#"OK ((" *recall-transcript*" 1))"#]];
    assert_agent_recall_parity(elisp_form, expect);
}
