use expect_test::expect;

use super::assert_agent_recall_parity;

#[test]
fn agent_recall_org_detection_and_property_reader_port_upstream_matrix_strictly() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "org-properties"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (org (expand-file-name "session.org" root)))
                      (make-directory root t)
                      (with-temp-file org
                        (insert "#+TITLE: Transcript\n"
                                "#+PROPERTY: Working_Directory   /work/project  \n"
                                "#+PROPERTY: Model claude-opus\n"
                                "#+PROPERTY: Empty    \n"))
                      (prog1
                          (list
                           (mapcar #'agent-recall--org-file-p
                                   (list org
                                         (expand-file-name "session.md" root)
                                         (expand-file-name "SESSION.ORG" root)
                                         nil))
                           (mapcar
                            (lambda (property)
                              (cons property
                                    (agent-recall--org-read-property
                                     org property)))
                            '("Working_Directory" "Model" "Empty" "Missing"))
                           (agent-recall--org-read-property
                            (expand-file-name "missing.org" root)
                            "Model"))
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK ((t nil nil nil) (("Working_Directory" . "/work/project") ("Model" . "claude-opus") ("Empty" . "") ("Missing")) nil)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_transcript_preview_ports_all_upstream_org_and_markdown_cases() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "preview-matrix"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (cases
                            '(("plain.org"
                               . "#+TITLE: Test\n\n** User\nHello world, this is my question\nsecond line\n\n** Assistant\nAnswer\n")
                              ("quote.org"
                               . "#+TITLE: Test\n\n** User\n#+begin_quote\nActual quoted message\nwith detail\n#+end_quote\n\n** Assistant\nResponse\n")
                              ("empty.org"
                               . "#+TITLE: Test\n\nJust random text\n")
                              ("plain.md"
                               . "# Transcript\n\n## User\n> What is Emacs?\n\n## Assistant\nEmacs is...\n")
                              ("unquoted.md"
                               . "# Transcript\n\n## User\nA practical message\n\n## Agent\nReply\n")
                              ("empty.md"
                               . "# Transcript\n\n## Agent\nNo user message\n")
                              ("long.org"
                               . "** User\nThis first line deliberately exceeds eighty columns: 01234567890123456789012345678901234567890123456789\n\n** Assistant\nx\n"))))
                      (make-directory root t)
                      (dolist (case cases)
                        (with-temp-file
                            (expand-file-name (car case) root)
                          (insert (cdr case))))
                      (prog1
                          (mapcar
                           (lambda (case)
                             (list (car case)
                                   (agent-recall--transcript-preview
                                    (expand-file-name (car case) root))))
                           cases)
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("plain.org" "Hello world, this is my question") ("quote.org" "Actual quoted message") ("empty.org" "(empty)") ("plain.md" "What is Emacs?") ("unquoted.md" "A practical message") ("empty.md" "(empty)") ("long.org" "This first line deliberately exceeds eighty columns: 012345678901234567890123456"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_first_message_ports_upstream_cases_and_preserves_multiline_content() {
    let elisp_form = r###"(let* ((root (expand-file-name
                                   "first-message"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (cases
                            '(("plain.org"
                               . "** User\nHow do I use org-roam?\nWith backlinks.\n\n** Assistant\nYou can...\n")
                              ("quote.org"
                               . "** User\n#+begin_quote\nWhat is the meaning of life?\nInclude evidence.\n#+end_quote\n\n** Assistant\n42\n")
                              ("none.org"
                               . "#+TITLE: Test\nNo headings here\n")
                              ("quoted.md"
                               . "## User\n> Explain atomic rename\n> in detail\n\n## Agent\nResponse\n")
                              ("plain.md"
                               . "## User\nFirst line\nSecond line\n\n## Agent\nResponse\n")
                              ("none.md"
                               . "## Agent\nOnly response\n"))))
                      (make-directory root t)
                      (dolist (case cases)
                        (with-temp-file
                            (expand-file-name (car case) root)
                          (insert (cdr case))))
                      (prog1
                          (mapcar
                           (lambda (case)
                             (list (car case)
                                   (agent-recall--transcript-first-message
                                    (expand-file-name (car case) root))))
                           cases)
                        (delete-directory root t)))"###;
    let expect = expect![[
        r#"OK (("plain.org" "How do I use org-roam?\nWith backlinks.") ("quote.org" "What is the meaning of life?\nInclude evidence.") ("none.org" nil) ("quoted.md" "Explain atomic rename\n> in detail") ("plain.md" "First line\nSecond line") ("none.md" nil))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_session_writer_ports_all_upstream_formats_and_is_idempotent() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "session-writer"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (uuid "deadbeef-1234-5678-9abc-def012345678")
                           (cases
                            '(("properties.org"
                               . "#+TITLE: Test\n#+PROPERTY: Working_Directory /work/project\n#+PROPERTY: Model opus\n\n** User\nHello\n")
                              ("title-only.org"
                               . "#+TITLE: Test\n\n** User\nHello\n")
                              ("bare.org"
                               . "** User\nHello\n")
                              ("session.md"
                               . "# Transcript\n\n**Started:** 2025-01-01\n\n---\n\n## User\nHi\n")
                              ("no-separator.md"
                               . "# Transcript\n\n## User\nHi\n"))))
                      (make-directory root t)
                      (dolist (case cases)
                        (with-temp-file
                            (expand-file-name (car case) root)
                          (insert (cdr case))))
                      (prog1
                          (mapcar
                           (lambda (case)
                             (let ((file
                                    (expand-file-name (car case) root)))
                               (agent-recall--write-session-id-to-file
                                file uuid)
                               (agent-recall--write-session-id-to-file
                                file "ffffffff-ffff-ffff-ffff-ffffffffffff")
                               (list
                                (car case)
                                (with-temp-buffer
                                  (insert-file-contents file)
                                  (buffer-string))
                                (agent-recall--read-embedded-session-id
                                 file))))
                           cases)
                        (delete-directory root t)))"##;
    let expect = expect![[
        r##"OK (("properties.org" "#+TITLE: Test\n#+PROPERTY: Working_Directory /work/project\n#+PROPERTY: Model opus\n#+PROPERTY: Session deadbeef-1234-5678-9abc-def012345678\n\n** User\nHello\n" "deadbeef-1234-5678-9abc-def012345678") ("title-only.org" "#+TITLE: Test\n#+PROPERTY: Session deadbeef-1234-5678-9abc-def012345678\n\n** User\nHello\n" "deadbeef-1234-5678-9abc-def012345678") ("bare.org" "\n#+PROPERTY: Session deadbeef-1234-5678-9abc-def012345678** User\nHello\n" "deadbeef-1234-5678-9abc-def012345678") ("session.md" "# Transcript\n\n**Started:** 2025-01-01\n\n**Session:** deadbeef-1234-5678-9abc-def012345678\n\n---\n\n## User\nHi\n" "deadbeef-1234-5678-9abc-def012345678") ("no-separator.md" "# Transcript\n\n## User\nHi\n" nil))"##
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_embedded_session_reader_rejects_malformed_and_misplaced_ids() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "embedded-reader"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (cases
                            '(("valid.org"
                               . "#+PROPERTY: Session deadbeef-1234-5678-9abc-def012345678\n")
                              ("valid.md"
                               . "**Session:** deadbeef-1234-5678-9abc-def012345678\n")
                              ("uppercase.org"
                               . "#+PROPERTY: Session DEADBEEF-1234-5678-9ABC-DEF012345678\n")
                              ("short.md"
                               . "**Session:** deadbeef-1234\n")
                              ("body.md"
                               . "Text **Session:** deadbeef-1234-5678-9abc-def012345678\n")
                              ("missing.org"
                               . "#+PROPERTY: Model opus\n"))))
                      (make-directory root t)
                      (dolist (case cases)
                        (with-temp-file
                            (expand-file-name (car case) root)
                          (insert (cdr case))))
                      (prog1
                          (mapcar
                           (lambda (case)
                             (list
                              (car case)
                              (agent-recall--read-embedded-session-id
                               (expand-file-name (car case) root))))
                           cases)
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("valid.org" "deadbeef-1234-5678-9abc-def012345678") ("valid.md" "deadbeef-1234-5678-9abc-def012345678") ("uppercase.org" "DEADBEEF-1234-5678-9ABC-DEF012345678") ("short.md" nil) ("body.md" nil) ("missing.org" nil))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_header_readers_cover_org_markdown_existing_and_missing_directories() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "header-readers"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (work (expand-file-name "actual-work" root))
                           (org (expand-file-name "session.org" root))
                           (markdown (expand-file-name "session.md" root))
                           (missing-dir (expand-file-name "gone" root)))
                      (make-directory work t)
                      (with-temp-file org
                        (insert "#+PROPERTY: Working_Directory "
                                work "\n"
                                "#+PROPERTY: Agent Claude Code\n"))
                      (with-temp-file markdown
                        (insert "**Working Directory:** " work "\n"
                                "**Agent:** Gemini CLI\n"))
                      (prog1
                          (list
                           (agent-recall--read-working-directory org)
                           (agent-recall--read-agent-name org)
                           (agent-recall--read-working-directory markdown)
                           (agent-recall--read-agent-name markdown)
                           (progn
                             (with-temp-file markdown
                               (insert "**Working Directory:** "
                                       missing-dir "\n"))
                             (agent-recall--read-working-directory
                              markdown))
                           (agent-recall--read-agent-name
                            (expand-file-name "missing.md" root)))
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/header-readers/actual-work" "Claude Code" "[ORACLE-SANDBOX]/header-readers/actual-work" "Gemini CLI" nil nil)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_transcript_timestamps_port_valid_upstream_org_and_markdown_cases_exactly() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "timestamp-reader"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (cases
                            '(("valid.org"
                               . "#+DATE: 2025-05-09 14:30:00\n")
                              ("valid.md"
                               . "**Started:** 2025-05-09 14:30:00\n")
                              ("timezone.md"
                               . "**Started:** 2025-05-09T14:30:00-04:00\n")
                              ("date-only.org"
                               . "#+DATE: 2025-05-09\n"))))
                      (make-directory root t)
                      (dolist (case cases)
                        (with-temp-file
                            (expand-file-name (car case) root)
                          (insert (cdr case))))
                      (prog1
                          (mapcar
                           (lambda (case)
                             (let ((time
                                    (agent-recall--parse-transcript-timestamp
                                     (expand-file-name (car case) root))))
                               (list
                                (car case)
                                (and time
                                     (format-time-string
                                      "%Y-%m-%dT%H:%M:%S%z"
                                      time t)))))
                           cases)
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("valid.org" "2025-05-09T14:30:00+0000") ("valid.md" "2025-05-09T14:30:00+0000") ("timezone.md" "2025-05-09T18:30:00+0000") ("date-only.org" "2025-05-09T00:00:00+0000"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_invalid_and_missing_transcript_timestamps_return_nil_instead_of_signaling() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "invalid-timestamp-reader"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (invalid (expand-file-name "invalid.md" root))
                           (missing (expand-file-name "missing.org" root)))
                      (make-directory root t)
                      (with-temp-file invalid
                        (insert "**Started:** not-a-date\n"))
                      (with-temp-file missing
                        (insert "#+TITLE: Test\n"))
                      (prog1
                          (list
                           (agent-recall--parse-transcript-timestamp
                            invalid)
                           (agent-recall--parse-transcript-timestamp
                            missing))
                        (delete-directory root t)))"##;
    let expect = expect![[r#"OK (nil nil)"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_project_name_from_org_ports_upstream_fallback_and_metadata_cases() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "project-from-file/fallback"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (metadata (expand-file-name "metadata.org" root))
                           (fallback (expand-file-name "fallback.org" root))
                           (empty (expand-file-name "empty.org" root)))
                      (make-directory root t)
                      (with-temp-file metadata
                        (insert "#+PROPERTY: Working_Directory /work/my-project\n"))
                      (with-temp-file fallback
                        (insert "#+TITLE: Test\n"))
                      (with-temp-file empty
                        (insert "#+PROPERTY: Working_Directory    \n"))
                      (prog1
                          (mapcar #'agent-recall--project-name-from-file
                                  (list metadata fallback empty))
                        (delete-directory
                         (file-name-directory
                          (directory-file-name root))
                         t)))"##;
    let expect = expect![[r#"OK ("my-project" "fallback" "fallback")"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_transcript_file_predicate_covers_layout_symlink_extra_and_boundaries() {
    let elisp_form = r##"(let ((agent-recall-transcript-dir-name
                           ".agent-shell/transcripts")
                          (agent-recall--symlink-dir
                           "/sandbox/index/search")
                          (agent-recall-extra-transcript-dirs
                           '((:dir "/archive/org")
                             (:dir "/archive/other/"))))
                      (mapcar
                       (lambda (file)
                         (cons file
                               (agent-recall--transcript-file-p file)))
                       '(nil
                         "/work/project/.agent-shell/transcripts/a.md"
                         "/work/project/.agent-shell/transcripts-old/a.md"
                         "/sandbox/index/search/project/a.org"
                         "/sandbox/index/search-other/a.org"
                         "/archive/org/session.org"
                         "/archive/organization/session.org"
                         "/archive/other/deep/session.md"
                         "/elsewhere/session.org")))"##;
    let expect = expect![[
        r#"OK ((nil) ("/work/project/.agent-shell/transcripts/a.md" . 13) ("/work/project/.agent-shell/transcripts-old/a.md") ("/sandbox/index/search/project/a.org" . t) ("/sandbox/index/search-other/a.org" . t) ("/archive/org/session.org" . t) ("/archive/organization/session.org") ("/archive/other/deep/session.md" . t) ("/elsewhere/session.org"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_navigation_walks_real_user_turns_and_reports_both_boundaries() {
    let elisp_form = r###"(with-temp-buffer
                      (insert "# Transcript\n"
                              "## User 1\nFirst\n"
                              "## Agent 1\nAnswer\n"
                              "## User 2\nSecond\n"
                              "## Agent 2\nAnswer\n"
                              "## User 3\nThird\n")
                      (goto-char (point-min))
                      (let (events)
                        (cl-letf (((symbol-function 'message)
                                   (lambda (format-string &rest args)
                                     (push (apply #'format
                                                  format-string args)
                                           events))))
                          (agent-recall-next-user-message)
                          (let ((first (line-number-at-pos)))
                            (agent-recall-next-user-message)
                            (let ((second (line-number-at-pos)))
                              (agent-recall-next-user-message)
                              (let ((third (line-number-at-pos)))
                                (agent-recall-next-user-message)
                                (agent-recall-prev-user-message)
                                (let ((back (line-number-at-pos)))
                                  (goto-char (point-min))
                                  (agent-recall-prev-user-message)
                                  (list first second third back
                                        (nreverse events)))))))))"###;
    let expect = expect![[r#"OK (2 6 10 6 ("No more user messages" "No earlier user messages"))"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_clean_view_preserves_conversation_and_strips_tools_and_thoughts() {
    let elisp_form = r####"(let* ((root (expand-file-name
                                   "clean-view"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (source (expand-file-name
                                    "2026-07-10.md" root))
                           (temporary-file-directory
                            (file-name-as-directory root))
                           result)
                      (make-directory root t)
                      (with-temp-file source
                        (insert "# Agent transcript\n"
                                "**Agent:** Claude Code\n\n"
                                "## User 1\nBuild an index.\n\n"
                                "## Agent 1\nI will inspect it.\n"
                                "### Tool Call\nrg source\n"
                                "### Tool Result\nmany lines\n"
                                "## User 2\nMake it atomic.\n"
                                "## Agent 2\nUse rename-file.\n"
                                "## Thought\nprivate chain\n"))
                      (let ((source-buffer (find-file-noselect source)))
                        (unwind-protect
                            (with-current-buffer source-buffer
                              (cl-letf (((symbol-function 'pop-to-buffer)
                                         (lambda (buffer &rest _)
                                           (setq result
                                                 (with-current-buffer buffer
                                                   (buffer-string))))))
                                (agent-recall-clean-view)
                                (list result
                                      (file-exists-p
                                       (expand-file-name
                                        "2026-07-10-clean.md"
                                        root)))))
                          (kill-buffer source-buffer)
                          (when-let ((clean
                                      (get-file-buffer
                                       (expand-file-name
                                        "2026-07-10-clean.md"
                                        root))))
                            (kill-buffer clean))
                          (delete-directory root t))))"####;
    let expect = expect![[
        r##"OK ("# Agent transcript\n**Agent:** Claude Code\n\n## ## User 1\nBuild an index.\n\n## Agent 1\nI will inspect it.\n## User 2\nMake it atomic.\n## Agent 2\nUse rename-file.\n" t)"##
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_transcript_mode_lifecycle_sets_read_only_session_header_and_cleans_up() {
    let elisp_form = r###"(let* ((root (expand-file-name
                                   "transcript-mode"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (file (expand-file-name
                                  "project/.agent-shell/transcripts/session.md"
                                  root))
                           (buffer nil))
                      (make-directory (file-name-directory file) t)
                      (with-temp-file file
                        (insert "**Session:** deadbeef-1234-5678-9abc-def012345678\n"
                                "## User\nHello\n"))
                      (setq buffer (find-file-noselect file))
                      (unwind-protect
                          (with-current-buffer buffer
                            (agent-recall-transcript-mode 1)
                            (let ((enabled
                                   (list agent-recall-transcript-mode
                                         buffer-read-only
                                         agent-recall--transcript-session-id
                                         (eval
                                          (cadr
                                           header-line-format)))))
                              (agent-recall-transcript-mode -1)
                              (list enabled
                                    agent-recall-transcript-mode
                                    buffer-read-only
                                    (local-variable-p
                                     'agent-recall--transcript-session-id)
                                    (local-variable-p
                                     'header-line-format))))
                        (kill-buffer buffer)
                        (delete-directory root t)))"###;
    let expect = expect![[
        r#"OK ((t t "deadbeef-1234-5678-9abc-def012345678" #("  r Resume (deadbeef)  c Clean  b Browse  C-j/C-k Navigate  q Quit" 2 3 (face agent-recall-header-key) 4 21 (face agent-recall-header-label) 23 24 (face agent-recall-header-key) 25 30 (face agent-recall-header-label) 32 33 (face agent-recall-header-key) 34 40 (face agent-recall-header-label) 42 49 (face agent-recall-header-key) 50 58 (face agent-recall-header-label) 60 61 (face agent-recall-header-key) 62 66 (face agent-recall-header-label))) nil nil nil nil)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_header_line_reflects_resume_existing_force_and_nonresumable_states() {
    let elisp_form = r##"(cl-letf (((symbol-function
                                'agent-recall--find-session-buffer)
                               (lambda (session-id)
                                 (and (equal session-id "existing-session")
                                      (get-buffer-create
                                       "agent-existing")))))
                      (unwind-protect
                          (mapcar
                           (lambda (session-id)
                             (let ((line
                                    (agent-recall--header-line session-id)))
                               (list
                                session-id
                                (substring-no-properties line)
                                (get-text-property
                                 (string-match "r" line)
                                 'face line))))
                           '(nil
                             "short"
                             "12345678-1234"
                             "existing-session"))
                        (when-let ((buffer
                                    (get-buffer "agent-existing")))
                          (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((nil "  c Clean  b Browse  C-j/C-k Navigate  q Quit" agent-recall-header-label) ("short" "  r Resume (short)  c Clean  b Browse  C-j/C-k Navigate  q Quit" agent-recall-header-key) ("12345678-1234" "  r Resume (12345678)  c Clean  b Browse  C-j/C-k Navigate  q Quit" agent-recall-header-key) ("existing-session" "  r Resume (agent-existing)  R Force Resume  c Clean  b Browse  C-j/C-k Navigate  q Quit" agent-recall-header-key))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}
