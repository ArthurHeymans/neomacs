use expect_test::expect;

use super::assert_agent_recall_parity;

#[test]
fn agent_recall_backfill_dry_run_reports_embedded_matched_and_unmatched_real_transcripts() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "backfill-dry"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (embedded
                            (expand-file-name "embedded.md" root))
                           (matched
                            (expand-file-name "matched.md" root))
                           (missing
                            (expand-file-name "missing.org" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t)
                           report)
                      (make-directory root t)
                      (with-temp-file embedded
                        (insert
                         "**Session:** deadbeef-1234-5678-9abc-def012345678\n"))
                      (with-temp-file matched (insert "matched"))
                      (with-temp-file missing (insert "missing"))
                      (puthash embedded '(:project "alpha")
                               agent-recall--index)
                      (puthash matched '(:project "beta")
                               agent-recall--index)
                      (puthash missing '(:project "gamma")
                               agent-recall--index)
                      (cl-letf (((symbol-function
                                 'agent-recall--project-root-for-session)
                                (lambda (_file) "/virtual/project"))
                               ((symbol-function
                                 'agent-recall--claude-project-dir)
                                (lambda (_root) "/virtual/claude"))
                               ((symbol-function
                                 'agent-recall--parse-transcript-timestamp)
                                (lambda (_file) '(1 0 0 0)))
                               ((symbol-function
                                 'agent-recall--load-sessions-index)
                                (lambda (_dir)
                                  '(("matched-session" . (1 0 0 0)))))
                               ((symbol-function
                                 'agent-recall--scan-jsonl-timestamps)
                                (lambda (_dir) nil))
                               ((symbol-function
                                 'agent-recall--match-session)
                                (lambda (_time file _sessions _dir)
                                  (and
                                   (string-suffix-p
                                    "matched.md" file)
                                   "matched-session")))
                               ((symbol-function 'pop-to-buffer)
                                (lambda (buffer &rest _)
                                  (setq report
                                        (with-current-buffer buffer
                                          (buffer-string))))))
                        (agent-recall-backfill nil))
                      (prog1 report
                        (when-let ((buffer
                                    (get-buffer
                                     "*agent-recall-backfill*")))
                          (kill-buffer buffer))
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK #("Agent Recall -- Backfill (DRY RUN)\n══════════════════════════════════════════════════\n\n  SKIP:     [alpha] embedded.md (has deadbeef)\n  MATCH:    [beta] matched.md → matched-\n  NO MATCH: [gamma] missing.org\n\n──────────────────────────────────────────────────\nSummary:\n  Total:      3\n  Matched:    1\n  Skipped:    1 (already have session ID)\n  No match:   1\n\n  To write, run: C-u C-u M-x agent-recall-backfill\n" 0 35 (face info-title-1) 259 268 (face bold))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_backfill_write_mode_updates_only_matches_and_writes_auditable_undo_log() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "backfill-write"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (matched
                            (expand-file-name "matched.md" root))
                           (unmatched
                            (expand-file-name "unmatched.md" root))
                           (agent-recall-index-file
                            (expand-file-name "state/index.el" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t)
                           report)
                      (make-directory root t)
                      (make-directory
                       (file-name-directory agent-recall-index-file)
                       t)
                      (with-temp-file matched
                        (insert "# Transcript\n\n---\n\n## User\nMatch\n"))
                      (with-temp-file unmatched
                        (insert "# Transcript\n\n---\n\n## User\nNo\n"))
                      (puthash matched '(:project "alpha")
                               agent-recall--index)
                      (puthash unmatched '(:project "beta")
                               agent-recall--index)
                      (cl-letf (((symbol-function
                                 'agent-recall--project-root-for-session)
                                (lambda (_file) "/virtual/project"))
                               ((symbol-function
                                 'agent-recall--claude-project-dir)
                                (lambda (_root) "/virtual/claude"))
                               ((symbol-function
                                 'agent-recall--parse-transcript-timestamp)
                                (lambda (_file) '(1 0 0 0)))
                               ((symbol-function
                                 'agent-recall--load-sessions-index)
                                (lambda (_dir)
                                  '(("deadbeef-1234-5678-9abc-def012345678"
                                     . (1 0 0 0)))))
                               ((symbol-function
                                 'agent-recall--scan-jsonl-timestamps)
                                (lambda (_dir) nil))
                               ((symbol-function
                                 'agent-recall--match-session)
                                (lambda (_time file _sessions _dir)
                                  (and
                                   (string-suffix-p
                                    "matched.md" file)
                                   (not
                                    (string-suffix-p
                                     "unmatched.md" file))
                                   "deadbeef-1234-5678-9abc-def012345678")))
                               ((symbol-function 'format-time-string)
                                (lambda (&rest _)
                                  "2026-07-10 17:07:00"))
                               ((symbol-function 'pop-to-buffer)
                                (lambda (buffer &rest _)
                                  (setq report
                                        (with-current-buffer buffer
                                          (buffer-string))))))
                        (agent-recall-backfill t))
                      (let ((log
                             (expand-file-name "state/backfill-log.el"
                                               root)))
                        (prog1
                            (list
                             (replace-regexp-in-string
                              (regexp-quote root) "[ROOT]" report t t)
                             (agent-recall--read-embedded-session-id
                              matched)
                             (agent-recall--read-embedded-session-id
                              unmatched)
                             (file-exists-p log)
                             (and
                              (file-exists-p log)
                              (replace-regexp-in-string
                               (regexp-quote root)
                               "[ROOT]"
                               (with-temp-buffer
                                 (insert-file-contents log)
                                 (buffer-string))
                               t t)))
                          (when-let ((buffer
                                      (get-buffer
                                       "*agent-recall-backfill*")))
                            (kill-buffer buffer))
                          (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (#("Agent Recall -- Backfill (WRITING)\n══════════════════════════════════════════════════\n\n  MATCH:    [alpha] matched.md → deadbeef\n  NO MATCH: [beta] unmatched.md\n\n──────────────────────────────────────────────────\nSummary:\n  Total:      2\n  Matched:    1\n  Skipped:    0 (already have session ID)\n  No match:   1\n\n  Wrote session IDs to 1 files.\n  Undo log: [ROOT]/state/backfill-log.el\n" 0 35 (face info-title-1) 213 222 (face bold)) "deadbeef-1234-5678-9abc-def012345678" nil t ";; agent-recall backfill undo log\n;; Written: 2026-07-10 17:07:00\n;; Files modified: 1\n\n;; To undo, evaluate this buffer (removes **Session:** lines):\n(dolist (file '(\n  \"[ROOT]/matched.md\"\n))\n  (when (file-exists-p file)\n    (with-temp-buffer\n      (insert-file-contents file)\n      (goto-char (point-min))\n      (when (re-search-forward \"^\\\\*\\\\*Session:\\\\*\\\\*.*\\n\\n?\" nil t)\n        (replace-match \"\")\n        (write-region (point-min) (point-max) file nil 'no-message)))))\n")"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_embark_setup_and_actions_open_resume_switch_force_and_error_correctly() {
    let elisp_form = r##"(let* ((candidate
                            (propertize "[project] session"
                                        'agent-recall-file
                                        "/virtual/session.md"))
                           (existing
                            (generate-new-buffer
                             "agent-existing"))
                           (embark-keymap-alist nil)
                           events
                           resolution)
                      (unwind-protect
                          (cl-letf (((symbol-function
                                     'agent-recall--open-transcript)
                                    (lambda (file &optional other)
                                      (push
                                       (list 'open file other)
                                       events)))
                                   ((symbol-function
                                     'agent-recall--resolve-session-id)
                                    (lambda (_file) resolution))
                                   ((symbol-function
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
                                       events))))
                            (agent-recall--setup-embark)
                            (agent-recall--setup-embark)
                            (agent-recall-embark-open-other-window
                             candidate)
                            (setq resolution "existing")
                            (agent-recall-embark-resume candidate)
                            (setq resolution "new")
                            (agent-recall-embark-resume candidate)
                            (agent-recall-embark-force-resume
                             candidate)
                            (setq resolution nil)
                            (let ((errors
                                   (list
                                    (condition-case error-data
                                        (agent-recall-embark-resume
                                         candidate)
                                      (error
                                       (list (car error-data)
                                             (cadr error-data))))
                                    (condition-case error-data
                                        (agent-recall-embark-force-resume
                                         candidate)
                                      (error
                                       (list (car error-data)
                                             (cadr error-data)))))))
                              (list embark-keymap-alist
                                    (nreverse events)
                                    errors)))
                        (kill-buffer existing)))"##;
    let expect = expect![[
        r#"OK (nil ((open "/virtual/session.md" t) (display "agent-existing") (start "new" "/virtual/session.md") (start "new" "/virtual/session.md")) ((user-error "This transcript has no resumable session ID") (user-error "This transcript has no resumable session ID")))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}
