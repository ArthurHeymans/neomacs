use expect_test::expect;

use super::assert_agent_recall_parity;

#[test]
fn agent_recall_agent_names_normalize_and_match_every_config_identity() {
    let elisp_form = r##"(let ((config
                           '(:identifier "claude-code"
                             :mode-line-name "Claude Code"
                             :buffer-name "Claude/Code Agent")))
                      (list
                       (mapcar
                        #'agent-recall--normalize-agent-name
                        '(nil
                          " Claude Code "
                          "CLAUDE-code"
                          "Gemini_CLI.v2"
                          42))
                       (mapcar
                        (lambda (name)
                          (cons
                           name
                           (and
                            (agent-recall--agent-config-matches-name-p
                             config name)
                            t)))
                        '(nil
                          "Claude Code"
                          "claude-code"
                          "Claude/Code Agent"
                          "claudecode"
                          "Claude"
                          "Gemini CLI"))))"##;
    let expect = expect![[
        r#"OK ((nil "claudecode" "claudecode" "geminicliv2" "42") ((nil) ("Claude Code" . t) ("claude-code" . t) ("Claude/Code Agent" . t) ("claudecode" . t) ("Claude") ("Gemini CLI")))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_selects_preferred_then_list_agent_config_from_real_headers() {
    let elisp_form = r##"(setq agent-shell-agent-configs
                           '((:identifier "gemini"
                              :buffer-name "Gemini CLI")
                             (:identifier "codex"
                              :mode-line-name "Codex")))
                      (let* ((root (expand-file-name
                                   "agent-config"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (claude (expand-file-name "claude.md" root))
                           (gemini (expand-file-name "gemini.org" root))
                           (unknown (expand-file-name "unknown.md" root))
                           (preferred
                            '(:identifier "claude-code"
                              :mode-line-name "Claude Code")))
                      (make-directory root t)
                      (with-temp-file claude
                        (insert "**Agent:** Claude Code\n"))
                      (with-temp-file gemini
                        (insert "#+PROPERTY: Agent Gemini CLI\n"))
                      (with-temp-file unknown
                        (insert "**Agent:** Other Agent\n"))
                      (cl-letf (((symbol-function
                                 'agent-shell--resolve-preferred-config)
                                (lambda () preferred)))
                        (prog1
                            (mapcar
                             (lambda (file)
                               (agent-recall--agent-config-for-transcript
                                file))
                             (list claude gemini unknown))
                          (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK ((:identifier "claude-code" :mode-line-name "Claude Code") (:identifier "gemini" :buffer-name "Gemini CLI") nil)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_find_session_buffer_matches_nested_id_or_resume_id_and_ignores_other_modes() {
    let elisp_form = r##"(let ((nested (generate-new-buffer
                                    " *agent-nested*"))
                          (resume (generate-new-buffer
                                   " *agent-resume*"))
                          (other (generate-new-buffer
                                  " *agent-other*")))
                      (unwind-protect
                          (progn
                            (put 'agent-shell-mode
                                 'derived-mode-parent
                                 'special-mode)
                            (with-current-buffer nested
                              (setq major-mode 'agent-shell-mode
                                    agent-shell--state
                                    '(:session
                                      (:id "session-nested"))))
                            (with-current-buffer resume
                              (setq major-mode 'agent-shell-mode
                                    agent-shell--state
                                    '(:resume-session-id
                                      "session-resume")))
                            (with-current-buffer other
                              (setq major-mode 'fundamental-mode
                                    agent-shell--state
                                    '(:session
                                      (:id "session-other"))))
                            (mapcar
                             (lambda (id)
                               (when-let ((buffer
                                           (agent-recall--find-session-buffer
                                            id)))
                                 (buffer-name buffer)))
                             '("session-nested"
                               "session-resume"
                               "session-other"
                               "missing")))
                        (kill-buffer nested)
                        (kill-buffer resume)
                        (kill-buffer other)))"##;
    let expect = expect![[r#"OK (nil nil " *agent-nested*" nil)"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_display_buffer_respects_viewport_preference_boundary() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer
                                    " *agent-display*"))
                          events)
                      (unwind-protect
                          (cl-letf (((symbol-function
                                     'agent-shell-viewport--show-buffer)
                                    (lambda (&rest args)
                                      (push (cons 'viewport args)
                                            events)))
                                   ((symbol-function 'pop-to-buffer)
                                    (lambda (target &rest _)
                                      (push
                                       (list 'pop
                                             (buffer-name target))
                                       events))))
                            (let ((agent-shell-prefer-viewport-interaction
                                   t))
                              (agent-recall--display-buffer buffer))
                            (let ((agent-shell-prefer-viewport-interaction
                                   nil))
                              (agent-recall--display-buffer buffer))
                            (nreverse
                             (mapcar
                              (lambda (event)
                                (if (eq (car event) 'viewport)
                                    (list 'viewport
                                          (cadr event)
                                          (buffer-name (caddr event)))
                                  event))
                              events)))
                        (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ((pop " *agent-display*") (pop " *agent-display*"))"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_resume_current_switches_existing_starts_missing_and_rejects_no_session() {
    let elisp_form = r##"(let ((transcript (generate-new-buffer
                                        " *recall-transcript*"))
                          (existing (generate-new-buffer
                                     "agent-existing"))
                          events)
                      (unwind-protect
                          (with-current-buffer transcript
                            (setq buffer-file-name
                                  "/virtual/project/session.md")
                            (cl-letf (((symbol-function
                                       'agent-recall--find-session-buffer)
                                      (lambda (id)
                                        (and (equal id "existing")
                                             existing)))
                                     ((symbol-function
                                       'agent-recall--display-buffer)
                                      (lambda (buffer)
                                        (push
                                         (list 'display
                                               (buffer-name buffer))
                                         events)))
                                     ((symbol-function
                                       'agent-recall--start-resume)
                                      (lambda (id &optional file)
                                        (push
                                         (list 'start id file)
                                         events)))
                                     ((symbol-function 'message)
                                      (lambda (format-string &rest args)
                                        (push
                                         (cons 'message
                                               (apply #'format
                                                      format-string
                                                      args))
                                         events))))
                              (setq-local
                               agent-recall--transcript-session-id
                               "existing")
                              (agent-recall-resume-current)
                              (setq-local
                               agent-recall--transcript-session-id
                               "new-session")
                              (agent-recall-resume-current)
                              (setq-local
                               agent-recall--transcript-session-id nil)
                              (let ((error-result
                                     (condition-case error-data
                                         (agent-recall-resume-current)
                                       (error
                                        (list (car error-data)
                                              (cadr error-data))))))
                                (list (nreverse events)
                                      error-result))))
                        (kill-buffer transcript)
                        (kill-buffer existing)))"##;
    let expect = expect![[
        r#"OK (((message . "Switching to existing buffer: agent-existing") (display "agent-existing") (start "new-session" "/virtual/project/session.md")) (user-error "This transcript has no resumable session ID"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_force_resume_always_starts_new_session_and_validates_local_id() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer
                                    " *force-resume*"))
                          events)
                      (unwind-protect
                          (with-current-buffer buffer
                            (setq buffer-file-name
                                  "/virtual/project/session.md")
                            (cl-letf (((symbol-function
                                       'agent-recall--start-resume)
                                      (lambda (id &optional file)
                                        (push
                                         (list id file)
                                         events))))
                              (setq-local
                               agent-recall--transcript-session-id
                               "force-id")
                              (agent-recall-force-resume-current)
                              (setq-local
                               agent-recall--transcript-session-id nil)
                              (list
                               (nreverse events)
                               (condition-case error-data
                                   (agent-recall-force-resume-current)
                                 (error
                                  (list (car error-data)
                                        (cadr error-data)))))))
                        (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ((("force-id" "/virtual/project/session.md")) (user-error "This transcript has no resumable session ID"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_start_resume_preserves_project_agent_session_strategy_and_transcript_continuity() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "start-resume"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (work (expand-file-name "project" root))
                           (file (expand-file-name "session.md" root))
                           (config '(:identifier "claude-code"))
                           (shell (generate-new-buffer
                                   " *started-agent*"))
                           (agent-recall-resume-continue-transcript t)
                           started displayed observed-default)
                      (make-directory work t)
                      (with-temp-file file
                        (insert "**Working Directory:** " work "\n"
                                "**Agent:** Claude Code\n"))
                      (unwind-protect
                          (cl-letf (((symbol-function
                                     'agent-recall--agent-config-for-transcript)
                                    (lambda (_file) config))
                                   ((symbol-function
                                     'agent-shell--start)
                                    (lambda (&rest args)
                                      (setq started args
                                            observed-default
                                            default-directory)
                                      shell))
                                   ((symbol-function
                                     'agent-recall--display-buffer)
                                    (lambda (buffer)
                                      (setq displayed
                                            (buffer-name buffer)))))
                            (agent-recall--start-resume
                             "session-42" file)
                            (list started
                                  (file-name-nondirectory
                                   (directory-file-name
                                    observed-default))
                                  displayed
                                  (with-current-buffer shell
                                    agent-shell--transcript-file)))
                        (kill-buffer shell)
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK ((:config (:identifier "claude-code") :session-id "session-42" :session-strategy new :no-focus t :new-session t) "project" " *started-agent*" "[ORACLE-SANDBOX]/start-resume/session.md")"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_start_resume_fallback_prompts_for_named_agent_but_prefers_default_without_header() {
    let elisp_form = r##"(let ((shell (generate-new-buffer
                                   " *fallback-shell*"))
                          events)
                      (unwind-protect
                          (cl-letf (((symbol-function
                                     'agent-recall--read-agent-name)
                                    (lambda (file)
                                      (and (string-match-p
                                            "named" file)
                                           "Unknown Agent")))
                                   ((symbol-function
                                     'agent-recall--read-working-directory)
                                    (lambda (_file) nil))
                                   ((symbol-function
                                     'agent-recall--agent-config-for-transcript)
                                    (lambda (_file) nil))
                                   ((symbol-function
                                     'agent-shell--resolve-preferred-config)
                                    (lambda ()
                                      (push 'preferred events)
                                      '(:identifier "preferred")))
                                   ((symbol-function
                                     'agent-shell-select-config)
                                    (lambda (&rest args)
                                      (push
                                       (list 'select args)
                                       events)
                                      '(:identifier "selected")))
                                   ((symbol-function
                                     'agent-shell--start)
                                    (lambda (&rest args)
                                      (push
                                       (list 'start
                                             (plist-get args :config))
                                       events)
                                      shell))
                                   ((symbol-function
                                     'agent-recall--display-buffer)
                                    (lambda (_buffer))))
                            (agent-recall--start-resume
                             "one" "/virtual/named.md")
                            (agent-recall--start-resume
                             "two" "/virtual/anonymous.md")
                            (nreverse events))
                        (kill-buffer shell)))"##;
    let expect = expect![[
        r#"OK ((select (:prompt "Resume Unknown Agent session with agent: ")) (start (:identifier "selected")) preferred (start (:identifier "preferred")))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_resume_filters_index_to_real_resolvable_sessions_and_preserves_preview_metadata() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "resume-list"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (a (expand-file-name "a.md" root))
                           (b (expand-file-name "b.md" root))
                           (gone (expand-file-name "gone.md" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t)
                           captured started)
                      (make-directory root t)
                      (with-temp-file a (insert "a"))
                      (with-temp-file b (insert "b"))
                      (puthash a
                               '(:project "alpha"
                                 :timestamp "2026-01"
                                 :session-id "embedded-a"
                                 :preview "First task")
                               agent-recall--index)
                      (puthash b
                               '(:project "beta"
                                 :timestamp "2026-02"
                                 :session-id nil
                                 :preview "Second task")
                               agent-recall--index)
                      (puthash gone
                               '(:project "gone"
                                 :timestamp "2026-03"
                                 :session-id "gone")
                               agent-recall--index)
                      (cl-letf (((symbol-function
                                 'agent-recall--resolve-session-id)
                                (lambda (file)
                                  (and (equal file b)
                                       "resolved-b")))
                               ((symbol-function 'completing-read)
                                (lambda (prompt collection &rest _)
                                  (let* ((metadata
                                          (funcall collection
                                                   "" nil 'metadata))
                                         (annotation
                                          (cdr
                                           (assq
                                            'annotation-function
                                            (cdr metadata))))
                                         (options
                                          (funcall collection
                                                   "" nil t)))
                                    (setq captured
                                          (list
                                           prompt
                                           (mapcar
                                            #'substring-no-properties
                                            options)
                                           (funcall
                                            annotation
                                            "[beta] 2026-02"))))
                                  "[beta] 2026-02"))
                               ((symbol-function
                                 'agent-recall--start-resume)
                                (lambda (id &optional file)
                                  (setq started
                                        (list id
                                              (file-name-nondirectory
                                               file))))))
                        (agent-recall-resume))
                      (prog1
                          (list captured started)
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("Resume session: " ("[beta] 2026-02" "[alpha] 2026-01") "  Second task") ("resolved-b" "b.md"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_iso8601_parser_handles_offsets_fractional_seconds_and_invalid_input() {
    let elisp_form = r##"(mapcar
                      (lambda (input)
                        (let ((time
                               (agent-recall--parse-iso8601-timestamp
                                input)))
                          (list input
                                (and time
                                     (format-time-string
                                      "%Y-%m-%dT%H:%M:%S%z"
                                      time t)))))
                      '(nil
                        ""
                        "not-a-time"
                        "2026-07-10T17:07:31Z"
                        "2026-07-10T13:07:31-04:00"
                        "2026-07-10T17:07:31.987Z"))"##;
    let expect = expect![[
        r#"OK ((nil nil) ("" nil) ("not-a-time" nil) ("2026-07-10T17:07:31Z" "2026-07-10T17:07:31+0000") ("2026-07-10T13:07:31-04:00" "2026-07-10T17:07:31+0000") ("2026-07-10T17:07:31.987Z" "2026-07-10T17:07:31+0000"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_claude_project_directory_mangles_real_path_characters_and_checks_existence() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "claude-config"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (agent-recall-claude-config-dir root)
                           (project "/work/My Project/foo_bar.v2")
                           (mangled
                            (replace-regexp-in-string
                             "[/. _]" "-" project))
                           (expected
                            (expand-file-name
                             (concat "projects/" mangled) root)))
                      (make-directory expected t)
                      (prog1
                          (list
                           (file-name-nondirectory
                            (agent-recall--claude-project-dir project))
                           (agent-recall--claude-project-dir
                            "/work/missing")
                           (agent-recall--claude-project-dir nil))
                        (delete-directory root t)))"##;
    let expect = expect![[r#"OK ("-work-My-Project-foo-bar-v2" nil nil)"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_load_sessions_index_parses_valid_entries_and_skips_broken_timestamps() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "sessions-index"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (file (expand-file-name
                                  "sessions-index.json" root)))
                      (make-directory root t)
                      (with-temp-file file
                        (insert
                         "{\"entries\":["
                         "{\"sessionId\":\"one\",\"created\":\"2026-07-10T17:07:00Z\"},"
                         "{\"sessionId\":\"bad\",\"created\":\"not-a-date\"},"
                         "{\"created\":\"2026-07-10T18:00:00Z\"},"
                         "{\"sessionId\":\"two\",\"created\":\"2026-07-10T13:08:00-04:00\"}"
                         "]}"))
                      (prog1
                          (mapcar
                           (lambda (entry)
                             (list
                              (car entry)
                              (format-time-string
                               "%Y-%m-%dT%H:%M:%SZ"
                               (cdr entry) t)))
                           (sort
                            (agent-recall--load-sessions-index root)
                            (lambda (a b)
                              (string< (car a) (car b)))))
                        (delete-directory root t)))"##;
    let expect = expect![[r#"OK (("one" "2026-07-10T17:07:00Z") ("two" "2026-07-10T17:08:00Z"))"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_jsonl_scanner_emits_one_user_activity_per_minute_per_session() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "jsonl-scan"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (one (expand-file-name "one.jsonl" root))
                           (two (expand-file-name "two.jsonl" root)))
                      (make-directory root t)
                      (with-temp-file one
                        (insert
                         "{\"type\":\"assistant\",\"timestamp\":\"2026-07-10T17:00:00Z\"}\n"
                         "{\"type\":\"user\",\"timestamp\":\"2026-07-10T17:01:01Z\"}\n"
                         "{\"type\":\"user\",\"timestamp\":\"2026-07-10T17:01:59Z\"}\n"
                         "{\"type\":\"user\",\"timestamp\":\"2026-07-10T17:02:00Z\"}\n"
                         "invalid json\n"))
                      (with-temp-file two
                        (insert
                         "{\"type\":\"user\",\"timestamp\":\"2026-07-11T09:30:00Z\"}\n"
                         "{\"type\":\"user\"}\n"))
                      (prog1
                          (mapcar
                           (lambda (entry)
                             (list
                              (car entry)
                              (format-time-string
                               "%Y-%m-%dT%H:%M:%SZ"
                               (cdr entry) t)))
                           (sort
                            (agent-recall--scan-jsonl-timestamps root)
                            (lambda (a b)
                              (or (string< (car a) (car b))
                                  (and (equal (car a) (car b))
                                       (time-less-p
                                        (cdr a) (cdr b)))))))
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("one" "2026-07-10T17:01:01Z") ("one" "2026-07-10T17:02:00Z") ("two" "2026-07-11T09:30:00Z"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_jsonl_first_message_skips_commands_and_supports_string_and_block_content() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "jsonl-message"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (string-file
                            (expand-file-name "string.jsonl" root))
                           (blocks-file
                            (expand-file-name "blocks.jsonl" root))
                           (none-file
                            (expand-file-name "none.jsonl" root)))
                      (make-directory root t)
                      (with-temp-file string-file
                        (insert
                         "{\"type\":\"user\",\"message\":{\"content\":\"  <command>init</command>  \"}}\n"
                         "{\"type\":\"assistant\",\"message\":{\"content\":\"ignore\"}}\n"
                         "{\"type\":\"user\",\"message\":{\"content\":\"  Build an atomic index  \"}}\n"))
                      (with-temp-file blocks-file
                        (insert
                         "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"image\",\"url\":\"x\"},{\"type\":\"text\",\"text\":\"Compare transcripts\"}]}}\n"))
                      (with-temp-file none-file
                        (insert "invalid\n{\"type\":\"assistant\"}\n"))
                      (prog1
                          (mapcar
                           #'agent-recall--jsonl-first-message
                           (list string-file blocks-file none-file))
                        (delete-directory root t)))"##;
    let expect = expect![[r#"OK ("Build an atomic index" "Compare transcripts" nil)"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_message_normalization_and_hybrid_matching_confirm_content_then_fallback_time() {
    let elisp_form = r###"(let* ((root (expand-file-name
                                   "hybrid-match"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (transcript
                            (expand-file-name "session.md" root))
                           (sessions
                            (list
                             (cons "closest"
                                   (encode-time
                                    10 0 12 10 7 2026 t))
                             (cons "confirmed"
                                   (encode-time
                                    40 0 12 10 7 2026 t))
                             (cons "far"
                                   (encode-time
                                    0 10 12 10 7 2026 t))))
                           (agent-recall-session-match-window 120))
                      (make-directory root t)
                      (with-temp-file transcript
                        (insert "## User\nBuild   an ATOMIC\nindex\n\n"
                                "## Agent\nOkay\n"))
                      (with-temp-file
                          (expand-file-name "closest.jsonl" root)
                        (insert
                         "{\"type\":\"user\",\"message\":{\"content\":\"Different request\"}}\n"))
                      (with-temp-file
                          (expand-file-name "confirmed.jsonl" root)
                        (insert
                         "{\"type\":\"user\",\"message\":{\"content\":\" build an atomic index \"}}\n"))
                      (prog1
                          (list
                           (agent-recall--normalize-message
                            "  Mixed\n\t WHITESPACE  ")
                           (agent-recall--match-session
                            (encode-time 0 0 12 10 7 2026 t)
                            transcript sessions root)
                           (agent-recall--match-session
                            (encode-time 0 0 12 10 7 2026 t)
                            transcript
                            (list (car sessions))
                            root)
                           (agent-recall--match-session
                            (encode-time 0 0 10 10 7 2026 t)
                            transcript sessions root)
                           (agent-recall--match-session
                            nil transcript sessions root))
                        (delete-directory root t)))"###;
    let expect = expect![[r#"OK ("mixed\n whitespace" "closest" "closest" nil nil)"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_resolve_session_id_prefers_cache_then_embedded_then_matching_and_caches_failures() {
    let elisp_form = r##"(let ((agent-recall--session-id-cache
                           (make-hash-table :test 'equal))
                          events)
                      (puthash "cached.md" "cached-id"
                               agent-recall--session-id-cache)
                      (puthash "none.md" 'none
                               agent-recall--session-id-cache)
                      (cl-letf (((symbol-function
                                 'agent-recall--read-embedded-session-id)
                                (lambda (file)
                                  (push (list 'embedded file) events)
                                  (and (equal file "embedded.md")
                                       "embedded-id")))
                               ((symbol-function
                                 'agent-recall--project-root-for-session)
                                (lambda (file)
                                  (push (list 'root file) events)
                                  "/project"))
                               ((symbol-function
                                 'agent-recall--claude-project-dir)
                                (lambda (_root) "/claude"))
                               ((symbol-function
                                 'agent-recall--parse-transcript-timestamp)
                                (lambda (_file) '(1 0 0 0)))
                               ((symbol-function
                                 'agent-recall--load-sessions-index)
                                (lambda (_dir)
                                  '(("match-id" . (1 0 0 1)))))
                               ((symbol-function
                                 'agent-recall--scan-jsonl-timestamps)
                                (lambda (_dir) nil))
                               ((symbol-function
                                 'agent-recall--match-session)
                                (lambda (_time file _sessions _dir)
                                  (push (list 'match file) events)
                                  (and (equal file "matched.md")
                                       "match-id"))))
                        (let ((first
                               (mapcar
                                #'agent-recall--resolve-session-id
                                '("cached.md" "none.md"
                                  "embedded.md" "matched.md"
                                  "missing.md"))))
                          (let ((second
                                 (mapcar
                                  #'agent-recall--resolve-session-id
                                  '("embedded.md" "matched.md"
                                    "missing.md"))))
                            (list first second
                                  (nreverse events)
                                  (mapcar
                                   (lambda (file)
                                     (gethash
                                      file
                                      agent-recall--session-id-cache))
                                   '("embedded.md"
                                     "matched.md"
                                     "missing.md")))))))"##;
    let expect = expect![[
        r#"OK (("cached-id" nil "embedded-id" "match-id" nil) ("embedded-id" "match-id" nil) ((embedded "embedded.md") (embedded "matched.md") (root "matched.md") (match "matched.md") (embedded "missing.md") (root "missing.md") (match "missing.md")) ("embedded-id" "match-id" none))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}
