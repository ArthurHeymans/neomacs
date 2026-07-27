use expect_test::expect;

use super::assert_agent_recall_parity;

#[test]
fn agent_recall_track_sessions_runs_full_init_to_first_turn_write_and_unsubscribe_workflow() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "tracking"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (file (expand-file-name "session.md" root))
                           (shell (generate-new-buffer
                                   " *tracked-agent-shell*"))
                           init-callback
                           turn-callback
                           events)
                      (make-directory root t)
                      (with-temp-file file
                        (insert "# Transcript\n\n---\n\n## User\nHello\n"))
                      (unwind-protect
                          (with-current-buffer shell
                            (setq-local
                             agent-shell--state
                             '(:session
                               (:id
                                "deadbeef-1234-5678-9abc-def012345678"))
                             agent-shell--transcript-file file)
                            (cl-letf (((symbol-function
                                       'agent-shell-subscribe-to)
                                      (lambda (&rest args)
                                        (let ((event
                                               (plist-get args :event))
                                              (callback
                                               (plist-get args :on-event)))
                                          (push
                                           (list 'subscribe event)
                                           events)
                                          (if (eq event 'init-session)
                                              (setq init-callback callback)
                                            (setq turn-callback callback))
                                          (if (eq event 'init-session)
                                              'init-token
                                            'turn-token))))
                                     ((symbol-function
                                       'agent-shell-unsubscribe)
                                      (lambda (&rest args)
                                        (push
                                         (list 'unsubscribe
                                               (plist-get
                                                args :subscription))
                                         events)))
                                     ((symbol-function
                                       'agent-recall--index-add)
                                      (lambda (path &optional id)
                                        (push
                                         (list 'index
                                               (file-name-nondirectory
                                                path)
                                               id)
                                         events))))
                              (agent-recall-track-sessions)
                              (funcall init-callback 'init-event)
                              (let ((pending-before-turn
                                     agent-recall--pending-session-id))
                                (funcall turn-callback 'turn-event)
                                (funcall turn-callback 'second-turn)
                                (list
                                 pending-before-turn
                                 agent-recall--pending-session-id
                                 agent-recall--session-id-written-p
                                 (agent-recall--read-embedded-session-id
                                  file)
                                 (nreverse events)))))
                        (kill-buffer shell)
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK ("deadbeef-1234-5678-9abc-def012345678" nil t "deadbeef-1234-5678-9abc-def012345678" ((subscribe init-session) (subscribe turn-complete) (index "session.md" "deadbeef-1234-5678-9abc-def012345678") (unsubscribe turn-token) (unsubscribe turn-token)))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_track_sessions_waits_until_transcript_exists_before_successful_write() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "tracking-delayed"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (file (expand-file-name "late.md" root))
                           (shell (generate-new-buffer
                                   " *delayed-agent-shell*"))
                           init-callback
                           turn-callback
                           events)
                      (make-directory root t)
                      (unwind-protect
                          (with-current-buffer shell
                            (setq-local
                             agent-shell--state
                             '(:session
                               (:id
                                "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"))
                             agent-shell--transcript-file file)
                            (cl-letf (((symbol-function
                                       'agent-shell-subscribe-to)
                                      (lambda (&rest args)
                                        (let ((event
                                               (plist-get args :event)))
                                          (if (eq event 'init-session)
                                              (setq
                                               init-callback
                                               (plist-get
                                                args :on-event))
                                            (setq
                                             turn-callback
                                             (plist-get
                                              args :on-event))))
                                        (if (eq (plist-get args :event)
                                                'init-session)
                                            'init-token
                                          'turn-token)))
                                     ((symbol-function
                                       'agent-shell-unsubscribe)
                                      (lambda (&rest args)
                                        (push
                                         (plist-get
                                          args :subscription)
                                         events)))
                                     ((symbol-function
                                       'agent-recall--index-add)
                                      (lambda (&rest args)
                                        (push
                                         (cons 'index args)
                                         events))))
                              (agent-recall-track-sessions)
                              (funcall init-callback nil)
                              (funcall turn-callback nil)
                              (let ((after-missing
                                     (list
                                      agent-recall--pending-session-id
                                      agent-recall--session-id-written-p
                                      events)))
                                (with-temp-file file
                                  (insert "# Transcript\n---\n"))
                                (funcall turn-callback nil)
                                (list
                                 after-missing
                                 agent-recall--pending-session-id
                                 agent-recall--session-id-written-p
                                 (agent-recall--read-embedded-session-id
                                  file)
                                 (nreverse events)))))
                        (kill-buffer shell)
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee" nil nil) nil t "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee" ((index "[ORACLE-SANDBOX]/tracking-delayed/late.md" "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee") turn-token))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_install_transcript_hook_is_idempotent_and_preserves_existing_order() {
    let elisp_form = r##"(let ((find-file-hook
                           '(existing-first existing-second)))
                      (agent-recall--install-transcript-hook)
                      (agent-recall--install-transcript-hook)
                      (list
                       find-file-hook
                       (cl-count
                        #'agent-recall--maybe-enable-from-search
                        find-file-hook)))"##;
    let expect =
        expect!["OK ((agent-recall--maybe-enable-from-search existing-first existing-second) 1)"];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_global_mode_predicate_enables_only_matching_visited_transcripts() {
    let elisp_form = r##"(let ((agent-recall--symlink-dir nil)
                          (agent-recall-extra-transcript-dirs nil)
                          calls)
                      (cl-letf (((symbol-function 'buffer-file-name)
                                 (lambda (&optional _buffer)
                                   (car calls)))
                                ((symbol-function
                                  'agent-recall-transcript-mode)
                                 (lambda (arg)
                                   (setq calls
                                         (list (car calls)
                                               'enabled arg)))))
                        (setq calls
                              '("/work/project/.agent-shell/transcripts/a.md"))
                        (agent-recall--maybe-enable-transcript-mode)
                        (let ((matching calls))
                          (setq calls '("/work/project/README.md"))
                          (agent-recall--maybe-enable-transcript-mode)
                          (list matching calls))))"##;
    let expect = expect![[
        r#"OK (("/work/project/.agent-shell/transcripts/a.md" enabled 1) ("/work/project/README.md"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}
