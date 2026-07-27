use expect_test::expect;

use super::assert_agent_recall_parity;

#[test]
fn agent_recall_exact_pin_dependencies_and_features_match() {
    let elisp_form = r##"(let ((descriptor
                         (cadr (assq 'agent-recall package-alist))))
                     (list
                      (package-desc-name descriptor)
                      (package-version-join
                       (package-desc-version descriptor))
                      (package-desc-reqs descriptor)
                      (mapcar #'featurep
                              '(agent-recall agent-shell))))"##;
    let expect = expect![[
        r#"OK (agent-recall "20260710.1707" ((emacs (29 1)) (agent-shell (0 1 0))) (t t))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_complete_callable_surface_arglists_and_commands_match() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list function
                              (help-function-arglist function t)
                              (and (commandp function)
                                   (interactive-form function))))
                      '(agent-recall--file-patterns
                        agent-recall--index-load
                        agent-recall--index-save
                        agent-recall--index-add
                        agent-recall--index-ensure
                        agent-recall--index-dirs
                        agent-recall--index-files
                        agent-recall--project-name
                        agent-recall--project-root
                        agent-recall--org-file-p
                        agent-recall--org-read-property
                        agent-recall--project-name-from-file
                        agent-recall--transcript-dir-from-file
                        agent-recall--project-root-for-session
                        agent-recall-invalidate-cache
                        agent-recall-reindex
                        agent-recall--list-transcript-files
                        agent-recall--file-patterns-as-includes
                        agent-recall--file-patterns-as-globs
                        agent-recall--ensure-symlink-dir
                        agent-recall--install-transcript-hook
                        agent-recall--maybe-enable-from-search
                        agent-recall--search-via-grep
                        agent-recall--search-via-deadgrep
                        agent-recall--search-via-counsel-rg
                        agent-recall--search-via-consult-ripgrep
                        agent-recall-search
                        agent-recall-search-live
                        agent-recall--list-transcripts
                        agent-recall--transcript-preview
                        agent-recall--candidate-file
                        agent-recall--open-transcript
                        agent-recall--browse-preview-state
                        agent-recall--browse-consult
                        agent-recall--ivy-browse-update-fn
                        agent-recall--ivy-browse-unwind
                        agent-recall--browse-ivy
                        agent-recall--browse-default
                        agent-recall-browse
                        agent-recall-clean-view
                        agent-recall-next-user-message
                        agent-recall-prev-user-message
                        agent-recall-browse-from-transcript
                        agent-recall--header-entry
                        agent-recall--header-line
                        agent-recall-transcript-mode
                        agent-recall--transcript-file-p
                        agent-recall--maybe-enable-transcript-mode
                        global-agent-recall-transcript-mode
                        agent-recall--find-session-buffer
                        agent-recall--display-buffer
                        agent-recall-resume-current
                        agent-recall-force-resume-current
                        agent-recall--read-working-directory
                        agent-recall--read-agent-name
                        agent-recall--normalize-agent-name
                        agent-recall--agent-config-matches-name-p
                        agent-recall--agent-config-for-transcript
                        agent-recall--start-resume
                        agent-recall-resume
                        agent-recall-stats
                        agent-recall--write-session-id-to-file
                        agent-recall-track-sessions
                        agent-recall--read-embedded-session-id
                        agent-recall--parse-transcript-timestamp
                        agent-recall--parse-iso8601-timestamp
                        agent-recall--claude-project-dir
                        agent-recall--load-sessions-index
                        agent-recall--scan-jsonl-timestamps
                        agent-recall--transcript-first-message
                        agent-recall--jsonl-first-message
                        agent-recall--normalize-message
                        agent-recall--match-session
                        agent-recall--resolve-session-id
                        agent-recall-backfill
                        agent-recall-embark-open-other-window
                        agent-recall-embark-resume
                        agent-recall-embark-force-resume
                        agent-recall--setup-embark))"##;
    let expect = expect![[
        r#"OK ((agent-recall--file-patterns nil nil) (agent-recall--index-load nil nil) (agent-recall--index-save nil nil) (agent-recall--index-add (file &optional session-id) nil) (agent-recall--index-ensure nil nil) (agent-recall--index-dirs nil nil) (agent-recall--index-files nil nil) (agent-recall--project-name (transcript-dir) nil) (agent-recall--project-root (transcript-dir) nil) (agent-recall--org-file-p (file) nil) (agent-recall--org-read-property (file property) nil) (agent-recall--project-name-from-file (file) nil) (agent-recall--transcript-dir-from-file (file) nil) (agent-recall--project-root-for-session (file) nil) (agent-recall-invalidate-cache nil (interactive nil)) (agent-recall-reindex nil (interactive nil)) (agent-recall--list-transcript-files (dir) nil) (agent-recall--file-patterns-as-includes nil nil) (agent-recall--file-patterns-as-globs nil nil) (agent-recall--ensure-symlink-dir nil nil) (agent-recall--install-transcript-hook nil nil) (agent-recall--maybe-enable-from-search nil nil) (agent-recall--search-via-grep (query dirs) nil) (agent-recall--search-via-deadgrep (query _dirs) nil) (agent-recall--search-via-counsel-rg (query _dirs) nil) (agent-recall--search-via-consult-ripgrep (query _dirs) nil) (agent-recall-search (query) (interactive "sSearch transcripts: ")) (agent-recall-search-live nil (interactive nil)) (agent-recall--list-transcripts nil nil) (agent-recall--transcript-preview (file) nil) (agent-recall--candidate-file (candidate) nil) (agent-recall--open-transcript (file &optional other-window) nil) (agent-recall--browse-preview-state (file-lookup) nil) (agent-recall--browse-consult (candidates annotate-fn) nil) (agent-recall--ivy-browse-update-fn nil nil) (agent-recall--ivy-browse-unwind nil nil) (agent-recall--browse-ivy (candidates _annotate-fn) nil) (agent-recall--browse-default (candidates annotate-fn) nil) (agent-recall-browse nil (interactive nil)) (agent-recall-clean-view nil (interactive nil)) (agent-recall-next-user-message nil (interactive nil)) (agent-recall-prev-user-message nil (interactive nil)) (agent-recall-browse-from-transcript nil (interactive nil)) (agent-recall--header-entry (key label) nil) (agent-recall--header-line (&optional session-id) nil) (agent-recall-transcript-mode (&optional arg) (interactive #1=(list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle)))) (agent-recall--transcript-file-p (file) nil) (agent-recall--maybe-enable-transcript-mode nil nil) (global-agent-recall-transcript-mode (&optional arg) (interactive #1#)) (agent-recall--find-session-buffer (session-id) nil) (agent-recall--display-buffer (buffer) nil) (agent-recall-resume-current nil (interactive nil)) (agent-recall-force-resume-current nil (interactive nil)) (agent-recall--read-working-directory (file) nil) (agent-recall--read-agent-name (file) nil) (agent-recall--normalize-agent-name (name) nil) (agent-recall--agent-config-matches-name-p (config name) nil) (agent-recall--agent-config-for-transcript (file) nil) (agent-recall--start-resume (session-id &optional transcript-file) nil) (agent-recall-resume nil (interactive nil)) (agent-recall-stats nil (interactive nil)) (agent-recall--write-session-id-to-file (filepath session-id) nil) (agent-recall-track-sessions nil nil) (agent-recall--read-embedded-session-id (file) nil) (agent-recall--parse-transcript-timestamp (file) nil) (agent-recall--parse-iso8601-timestamp (iso-string) nil) (agent-recall--claude-project-dir (project-path) nil) (agent-recall--load-sessions-index (claude-dir) nil) (agent-recall--scan-jsonl-timestamps (claude-dir) nil) (agent-recall--transcript-first-message (file) nil) (agent-recall--jsonl-first-message (file) nil) (agent-recall--normalize-message (text) nil) (agent-recall--match-session (transcript-time transcript-file sessions claude-dir) nil) (agent-recall--resolve-session-id (file) nil) (agent-recall-backfill (&optional write-mode) (interactive "P")) (agent-recall-embark-open-other-window (candidate) nil) (agent-recall-embark-resume (candidate) nil) (agent-recall-embark-force-resume (candidate) nil) (agent-recall--setup-embark nil nil))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_customization_defaults_and_types_match() {
    let elisp_form = r##"(mapcar
                      (lambda (variable)
                        (list variable
                              (symbol-value variable)
                              (get variable 'custom-type)
                              (get variable 'standard-value)))
                      '(agent-recall-search-paths
                        agent-recall-max-depth
                        agent-recall-transcript-dir-name
                        agent-recall-file-patterns
                        agent-recall-extra-transcript-dirs
                        agent-recall-rg-executable
                        agent-recall-search-extra-args
                        agent-recall-search-context-lines
                        agent-recall-search-function
                        agent-recall-browse-sort
                        agent-recall-resume-continue-transcript
                        agent-recall-session-match-window
                        agent-recall-auto-transcript-mode
                        agent-recall-browse-preview))"##;
    let expect = expect![[
        r#"OK ((agent-recall-search-paths nil (repeat directory) ((funcall #'#[nil (nil) #1=(embark-keymap-alist ivy-unwind-fns-alist ivy-update-fns-alist ivy-last consult-ripgrep-args counsel-rg-base-command deadgrep-extra-arguments t)]))) (agent-recall-max-depth 6 integer ((funcall #'#[nil (6) #1#]))) (agent-recall-transcript-dir-name ".agent-shell/transcripts" string ((funcall #'#[nil (".agent-shell/transcripts") #1#]))) (agent-recall-file-patterns ("*.md" "*.org") (repeat string) ((funcall #'#[nil ('("*.md" "*.org")) #1#]))) (agent-recall-extra-transcript-dirs nil (repeat (plist :key-type symbol :value-type string)) ((funcall #'#[nil (nil) #1#]))) (agent-recall-rg-executable "rg" string ((funcall #'#[nil ("rg") #1#]))) (agent-recall-search-extra-args ("--follow" "--sort=modified") (repeat string) ((funcall #'#[nil ('("--follow" "--sort=modified")) #1#]))) (agent-recall-search-context-lines 2 integer ((funcall #'#[nil (2) #1#]))) (agent-recall-search-function grep (choice (const :tag "grep-mode (built-in)" grep) (const :tag "deadgrep" deadgrep) (const :tag "counsel-rg (ivy)" counsel-rg) (const :tag "consult-ripgrep (vertico)" consult-ripgrep)) ((funcall #'#[nil ('grep) #1#]))) (agent-recall-browse-sort date-desc (choice (const :tag "Newest first (created)" date-desc) (const :tag "Oldest first (created)" date-asc) (const :tag "Recently modified first" modified-desc) (const :tag "Least recently modified first" modified-asc) (const :tag "By project" project)) ((funcall #'#[nil ('date-desc) #1#]))) (agent-recall-resume-continue-transcript t boolean ((funcall #'#[nil (t) #1#]))) (agent-recall-session-match-window 120 integer ((funcall #'#[nil (120) #1#]))) (agent-recall-auto-transcript-mode t boolean ((funcall #'#[nil (t) #1#]))) (agent-recall-browse-preview t boolean ((funcall #'#[nil (t) #1#]))))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_modes_keymaps_faces_and_embark_actions_match() {
    let elisp_form = r##"(list
                      (mapcar
                       (lambda (key)
                         (cons key
                               (lookup-key agent-recall-transcript-mode-map
                                           (kbd key))))
                       '("r" "R" "c" "b" "C-c C-n" "C-c C-p"))
                      (mapcar
                       (lambda (key)
                         (cons key
                               (lookup-key agent-recall-transcript-embark-map
                                           (kbd key))))
                       '("o" "r" "R"))
                      (mapcar
                       (lambda (face)
                         (list face
                               (get face 'face-defface-spec)
                               (get face 'face-documentation)))
                       '(agent-recall-header-key
                         agent-recall-header-label))
                      (list
                       (get 'agent-recall-transcript-mode 'function-documentation)
                       (get 'global-agent-recall-transcript-mode
                            'function-documentation)))"##;
    let expect = expect![[
        r#"OK ((("r" . agent-recall-resume-current) ("R" . agent-recall-force-resume-current) ("c" . agent-recall-clean-view) ("b" . agent-recall-browse-from-transcript) ("C-c C-n" . agent-recall-next-user-message) ("C-c C-p" . agent-recall-prev-user-message)) (("o" . agent-recall-embark-open-other-window) ("r" . agent-recall-embark-resume) ("R" . agent-recall-embark-force-resume)) ((agent-recall-header-key ((t :inherit warning)) "Face for keybinding letters in the transcript header line.") (agent-recall-header-label ((t :inherit default)) "Face for labels in the transcript header line.")) (nil nil))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_obsolete_pattern_alias_and_normalization_match() {
    let elisp_form = r##"(let ((original agent-recall-file-patterns))
                      (unwind-protect
                          (list
                           (let ((agent-recall-file-patterns "*.md"))
                             (agent-recall--file-patterns))
                           (let ((agent-recall-file-patterns
                                  '("*.md" "*.org" "*.md")))
                             (agent-recall--file-patterns))
                           (progn
                             (setq agent-recall-file-pattern "*.txt")
                             (list agent-recall-file-pattern
                                   agent-recall-file-patterns
                                   (agent-recall--file-patterns)))
                           (get 'agent-recall-file-pattern
                                'obsolete-variable))
                        (setq agent-recall-file-patterns original)))"##;
    let expect =
        expect![[r#"OK (("*.md") ("*.md" "*.org" "*.md") ("*.txt" "*.txt" ("*.txt")) nil)"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_buffer_local_state_is_independent_across_shells_and_transcripts() {
    let elisp_form = r##"(let ((one (generate-new-buffer " *recall-one*"))
                          (two (generate-new-buffer " *recall-two*")))
                      (unwind-protect
                          (progn
                            (with-current-buffer one
                              (setq-local agent-recall--pending-session-id "one"
                                          agent-recall--session-id-written-p t
                                          agent-recall--transcript-session-id "session-one"
                                          agent-recall--search-buffer-p t))
                            (with-current-buffer two
                              (setq-local agent-recall--pending-session-id "two"))
                            (list
                             (with-current-buffer one
                               (list agent-recall--pending-session-id
                                     agent-recall--session-id-written-p
                                     agent-recall--transcript-session-id
                                     agent-recall--search-buffer-p))
                             (with-current-buffer two
                               (list agent-recall--pending-session-id
                                     agent-recall--session-id-written-p
                                     agent-recall--transcript-session-id
                                     agent-recall--search-buffer-p))))
                        (kill-buffer one)
                        (kill-buffer two)))"##;
    let expect = expect![[r#"OK (("one" t "session-one" t) ("two" nil nil nil))"#]];
    assert_agent_recall_parity(elisp_form, expect);
}
