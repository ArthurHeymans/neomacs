use expect_test::expect;

use super::assert_agent_recall_parity;

#[test]
fn agent_recall_list_transcripts_applies_every_sort_to_real_index_entries() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "browse-sorts"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (a (expand-file-name "a.md" root))
                           (b (expand-file-name "b.md" root))
                           (c (expand-file-name "c.org" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t)
                           results)
                      (make-directory root t)
                      (dolist (file (list a b c))
                        (with-temp-file file (insert file)))
                      (set-file-times a (encode-time 0 0 1 1 1 2026 t))
                      (set-file-times b (encode-time 0 0 2 1 1 2026 t))
                      (set-file-times c (encode-time 0 0 3 1 1 2026 t))
                      (puthash a
                               '(:project "zeta"
                                 :timestamp "2026-01-03-00-00-00")
                               agent-recall--index)
                      (puthash b
                               '(:project "alpha"
                                 :timestamp "2026-01-01-00-00-00")
                               agent-recall--index)
                      (puthash c
                               '(:project "middle"
                                 :timestamp "2026-01-02-00-00-00")
                               agent-recall--index)
                      (dolist (sort
                               '(date-desc date-asc modified-desc
                                 modified-asc project))
                        (let ((agent-recall-browse-sort sort))
                          (push
                           (cons
                            sort
                            (mapcar
                             (lambda (entry)
                               (list (car entry)
                                     (file-name-nondirectory
                                      (cdr entry))))
                             (agent-recall--list-transcripts)))
                           results)))
                      (delete-directory root t)
                      (nreverse results))"##;
    let expect = expect![[
        r#"OK ((date-desc ("[zeta] 2026-01-03-00-00-00" "a.md") ("[middle] 2026-01-02-00-00-00" "c.org") ("[alpha] 2026-01-01-00-00-00" "b.md")) (date-asc ("[alpha] 2026-01-01-00-00-00" "b.md") ("[middle] 2026-01-02-00-00-00" "c.org") ("[zeta] 2026-01-03-00-00-00" "a.md")) (modified-desc ("[middle] 2026-01-02-00-00-00" "c.org") ("[alpha] 2026-01-01-00-00-00" "b.md") ("[zeta] 2026-01-03-00-00-00" "a.md")) (modified-asc ("[zeta] 2026-01-03-00-00-00" "a.md") ("[alpha] 2026-01-01-00-00-00" "b.md") ("[middle] 2026-01-02-00-00-00" "c.org")) (project ("[alpha] 2026-01-01-00-00-00" "b.md") ("[middle] 2026-01-02-00-00-00" "c.org") ("[zeta] 2026-01-03-00-00-00" "a.md")))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_browse_candidates_preserve_file_property_and_preview_annotation() {
    let elisp_form = r##"(let* ((file "/virtual/project/session.md")
                           (candidate
                            (propertize "[project] session"
                                        'agent-recall-file file))
                           (agent-recall--index
                            (let ((table
                                   (make-hash-table :test 'equal)))
                              (puthash file
                                       '(:preview
                                         "Design an atomic index")
                                       table)
                              table))
                           (annotation
                            (lambda (value)
                              (when-let* ((path
                                           (agent-recall--candidate-file
                                            value))
                                          (entry
                                           (gethash
                                            path agent-recall--index))
                                          (preview
                                           (plist-get entry :preview)))
                                (unless (string-empty-p preview)
                                  (concat "  " preview))))))
                      (list
                       (agent-recall--candidate-file candidate)
                       (funcall annotation candidate)
                       (agent-recall--candidate-file
                        (substring-no-properties candidate))
                       (text-properties-at 0 candidate)))"##;
    let expect = expect![[
        r#"OK ("/virtual/project/session.md" "  Design an atomic index" nil (agent-recall-file "/virtual/project/session.md"))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_default_browser_exposes_completion_metadata_order_annotation_and_history() {
    let elisp_form = r##"(let* ((one (propertize "[alpha] one"
                                             'agent-recall-file
                                             "/alpha/one.md"))
                           (two (propertize "[beta] two"
                                             'agent-recall-file
                                             "/beta/two.md"))
                           (candidates (list one two))
                           (agent-recall--browse-history
                            '("[beta] previous"))
                           captured)
                      (cl-letf (((symbol-function 'completing-read)
                                 (lambda (prompt collection predicate
                                                 require-match initial-input
                                                 history default)
                                   (setq captured
                                         (list
                                          prompt
                                          predicate require-match
                                          initial-input history default
                                          (funcall collection
                                                   "" nil 'metadata)
                                          (funcall collection
                                                   "[a" nil t)))
                                   two)))
                        (list
                         (agent-recall--browse-default
                          candidates
                          (lambda (candidate)
                            (concat " note:"
                                    (substring-no-properties
                                     candidate))))
                         captured)))"##;
    let expect = expect![[
        r#"OK (#("[beta] two" 0 10 (agent-recall-file "/beta/two.md")) ("Transcript: " nil t nil agent-recall--browse-history "[beta] previous" (metadata (category . agent-recall-transcript) (display-sort-function . identity) (cycle-sort-function . identity) (annotation-function . #[(candidate) ((concat " note:" (substring-no-properties candidate))) (t)])) (#("[alpha] one" 0 11 (agent-recall-file "/alpha/one.md")))))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_browse_orchestrates_candidates_annotations_selection_and_opening() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "browse-command"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (file (expand-file-name "chosen.md" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t)
                           (agent-recall-browse-preview nil)
                           captured)
                      (make-directory root t)
                      (with-temp-file file (insert "chosen"))
                      (puthash file
                               '(:project "project"
                                 :timestamp "2026-07-10"
                                 :preview "Chosen transcript")
                               agent-recall--index)
                      (cl-letf (((symbol-function
                                 'agent-recall--browse-default)
                                (lambda (candidates annotate)
                                  (setq captured
                                        (mapcar
                                         (lambda (candidate)
                                           (list
                                            (substring-no-properties
                                             candidate)
                                            (file-name-nondirectory
                                             (agent-recall--candidate-file
                                              candidate))
                                            (funcall annotate candidate)))
                                         candidates))
                                  (car candidates)))
                               ((symbol-function
                                 'agent-recall--open-transcript)
                                (lambda (chosen &optional other)
                                  (setq captured
                                        (append captured
                                                (list
                                                 (list
                                                  'opened
                                                  (file-name-nondirectory
                                                   chosen)
                                                  other)))))))
                        (agent-recall-browse))
                      (prog1
                          captured
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("[project] 2026-07-10" "chosen.md" "  Chosen transcript") (opened "chosen.md" nil))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_open_transcript_uses_requested_window_resets_point_and_toggles_mode() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "open-transcript"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (file (expand-file-name "session.md" root))
                           (agent-recall-auto-transcript-mode t)
                           calls)
                      (make-directory root t)
                      (with-temp-file file
                        (insert "line one\nline two\n"))
                      (cl-letf (((symbol-function 'find-file)
                                 (lambda (path)
                                   (push (list 'same path) calls)
                                   (switch-to-buffer
                                    (find-file-noselect path))
                                   (goto-char (point-max))))
                                ((symbol-function
                                  'find-file-other-window)
                                 (lambda (path)
                                   (push (list 'other path) calls)
                                   (switch-to-buffer
                                    (find-file-noselect path))
                                   (goto-char (point-max))))
                                ((symbol-function
                                  'agent-recall-transcript-mode)
                                 (lambda (arg)
                                   (push
                                    (list 'mode arg (point))
                                    calls))))
                        (agent-recall--open-transcript file)
                        (agent-recall--open-transcript file t))
                      (prog1
                          (mapcar
                           (lambda (call)
                             (if (memq (car call) '(same other))
                                 (list (car call)
                                       (file-name-nondirectory
                                        (cadr call)))
                               call))
                           (nreverse calls))
                        (when-let ((buffer (get-file-buffer file)))
                          (kill-buffer buffer))
                        (delete-directory root t)))"##;
    let expect =
        expect![[r#"OK ((same "session.md") (mode 1 1) (other "session.md") (mode 1 1))"#]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_browse_builds_lookup_preview_state_and_returns_exact_candidate() {
    let elisp_form = r##"(let* ((one (propertize "[alpha] one"
                                             'agent-recall-file
                                             "/alpha/one.md"))
                           (two (propertize "[beta] two"
                                             'agent-recall-file
                                             "/beta/two.md"))
                           (agent-recall--browse-history nil)
                           captured)
                      (cl-letf (((symbol-function
                                 'agent-recall--browse-preview-state)
                                (lambda (lookup)
                                  (setq captured
                                        (list
                                         (gethash "[alpha] one" lookup)
                                         (gethash "[beta] two" lookup)))
                                  'preview-state))
                               ((symbol-function 'consult--read)
                                (lambda (candidates &rest args)
                                  (setq captured
                                        (append
                                         captured
                                         (list
                                          (mapcar
                                           #'substring-no-properties
                                           candidates)
                                          (plist-get args :prompt)
                                          (plist-get args :state)
                                          (funcall
                                           (plist-get args :lookup)
                                           "[beta] two"
                                           candidates))))
                                  two)))
                        (list
                         (agent-recall--browse-consult
                          (list one two)
                          (lambda (_candidate) " note"))
                         captured)))"##;
    let expect = expect![[
        r#"OK (#("[beta] two" 0 10 (agent-recall-file #1="/beta/two.md")) ("/alpha/one.md" "/beta/two.md" ("[alpha] one" "[beta] two") "Transcript: " preview-state #("[beta] two" 0 10 (agent-recall-file #1#))))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_browse_preview_state_opens_mapped_files_previews_buffers_and_cleans_up() {
    let elisp_form = r##"(let ((lookup
                           (let ((table
                                  (make-hash-table :test 'equal)))
                             (puthash "[alpha] one"
                                      "/virtual/one.md" table)
                             table))
                          events
                          preview-buffer)
                      (cl-letf (((symbol-function
                                 'consult--temporary-files)
                                (lambda ()
                                  (lambda (&optional file)
                                    (if file
                                        (progn
                                          (push (list 'open file)
                                                events)
                                          (setq preview-buffer
                                                (get-buffer-create
                                                 " *browse-preview*")))
                                      (push '(cleanup) events)))))
                               ((symbol-function
                                 'consult--buffer-preview)
                                (lambda ()
                                  (lambda (action buffer-name)
                                    (push
                                     (list 'preview
                                           action buffer-name)
                                     events)))))
                        (let ((state
                               (agent-recall--browse-preview-state
                                lookup)))
                          (funcall state
                                   'preview "[alpha] one")
                          (funcall state
                                   'return "[alpha] one")
                          (funcall state 'exit nil)
                          (prog1 (nreverse events)
                            (when preview-buffer
                              (kill-buffer preview-buffer))))))"##;
    let expect = expect![[
        r#"OK ((open "/virtual/one.md") (preview preview " *browse-preview*") (preview return nil) (cleanup) (preview exit nil))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_ivy_browse_wires_caller_preview_unwind_history_and_selected_candidate() {
    let elisp_form = r##"(setq ivy-update-fns-alist nil
                           ivy-unwind-fns-alist nil)
                      (let* ((one (propertize "[alpha] one"
                                             'agent-recall-file
                                             "/alpha/one.md"))
                           (two (propertize "[beta] two"
                                             'agent-recall-file
                                             "/beta/two.md"))
                           (agent-recall--browse-history
                            '("[alpha] previous"))
                           captured
                           cleanup-count)
                      (cl-letf (((symbol-function 'ivy-read)
                                 (lambda (prompt candidates &rest args)
                                   (setq captured
                                         (list
                                          prompt
                                          (mapcar
                                           #'substring-no-properties
                                           candidates)
                                          (plist-get args :caller)
                                          (plist-get args :require-match)
                                          (plist-get args :preselect)
                                          (plist-get args :history)
                                          (funcall
                                           (plist-get args :action)
                                           two)
                                          ivy-update-fns-alist
                                          ivy-unwind-fns-alist))
                                   two))
                                ((symbol-function
                                  'agent-recall--ivy-browse-unwind)
                                 (lambda ()
                                   (setq cleanup-count
                                         (1+ (or cleanup-count 0))))))
                        (list
                         (agent-recall--browse-ivy
                          (list one two)
                          (lambda (_candidate) nil))
                         captured cleanup-count)))"##;
    let expect = expect![[
        r#"OK (#("[beta] two" 0 10 (agent-recall-file #1="/beta/two.md")) ("Transcript: " ("[alpha] one" "[beta] two") agent-recall-browse t "[alpha] previous" agent-recall--browse-history #("[beta] two" 0 10 (agent-recall-file #1#)) ((agent-recall-browse . agent-recall--ivy-browse-update-fn)) ((agent-recall-browse . agent-recall--ivy-browse-unwind))) 1)"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_ivy_unwind_kills_only_preview_buffers_and_resets_tracking() {
    let elisp_form = r##"(let ((kept (generate-new-buffer
                                  " *recall-kept*"))
                          (preview-one
                           (generate-new-buffer
                            " *recall-preview-one*"))
                          (preview-two
                           (generate-new-buffer
                            " *recall-preview-two*")))
                      (setq agent-recall--ivy-temporary-buffers
                            (list preview-one preview-two))
                      (agent-recall--ivy-browse-unwind)
                      (prog1
                          (list
                           (buffer-live-p kept)
                           (buffer-live-p preview-one)
                           (buffer-live-p preview-two)
                           agent-recall--ivy-temporary-buffers)
                        (kill-buffer kept)))"##;
    let expect = expect!["OK (t nil nil nil)"];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_stats_computes_project_counts_sizes_and_sorted_report() {
    let elisp_form = r##"(let* ((root (expand-file-name
                                   "stats"
                                   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                           (a (expand-file-name "a.md" root))
                           (b (expand-file-name "b.md" root))
                           (c (expand-file-name "c.org" root))
                           (gone (expand-file-name "gone.md" root))
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall--index-loaded-p t)
                           report)
                      (make-directory root t)
                      (with-temp-file a (insert (make-string 10 ?a)))
                      (with-temp-file b (insert (make-string 20 ?b)))
                      (with-temp-file c (insert (make-string 5 ?c)))
                      (puthash a '(:project "alpha")
                               agent-recall--index)
                      (puthash b '(:project "alpha")
                               agent-recall--index)
                      (puthash c '(:project "beta")
                               agent-recall--index)
                      (puthash gone '(:project "gone")
                               agent-recall--index)
                      (cl-letf (((symbol-function 'pop-to-buffer)
                                 (lambda (buffer &rest _)
                                   (setq report
                                         (with-current-buffer buffer
                                           (buffer-string))))))
                        (agent-recall-stats))
                      (prog1 report
                        (when-let ((buffer
                                    (get-buffer
                                     "*agent-recall-stats*")))
                          (kill-buffer buffer))
                        (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK #("Agent Recall -- Transcript Statistics\n════════════════════════════════════════\n\n  Transcripts: 3\n  Projects:    2\n  Total size:  0.0 MB\n\nBy Project:\n────────────────────────────────────────\n  alpha                             2 files  (0.0 MB)\n  beta                              1 files  (0.0 MB)\n" 0 38 (face info-title-1) 137 149 (face bold))"#
    ]];
    assert_agent_recall_parity(elisp_form, expect);
}

#[test]
fn agent_recall_browse_from_transcript_quits_then_reopens_browser_in_order() {
    let elisp_form = r##"(let (events)
                      (cl-letf (((symbol-function 'quit-window)
                                 (lambda (&rest _)
                                   (push 'quit events)))
                                ((symbol-function 'agent-recall-browse)
                                 (lambda ()
                                   (push 'browse events))))
                        (agent-recall-browse-from-transcript))
                      (nreverse events))"##;
    let expect = expect!["OK (quit browse)"];
    assert_agent_recall_parity(elisp_form, expect);
}
