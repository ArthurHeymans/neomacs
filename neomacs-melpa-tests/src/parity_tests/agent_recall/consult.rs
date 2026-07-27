use expect_test::expect;

use super::assert_agent_recall_consult_parity;

#[test]
fn agent_recall_consult_exact_pin_feature_default_and_callable_surface_match() {
    let elisp_form = r##"(list
                      (featurep 'agent-recall-consult)
                      agent-recall-consult-resumable-only
                      (mapcar
                       (lambda (function)
                         (list
                          function
                          (help-function-arglist function t)
                          (and (commandp function)
                               (interactive-form function))))
                       '(agent-recall-consult--ensure-consult
                         agent-recall-consult--ripgrep-args
                         agent-recall-consult--humanize-timestamp
                         agent-recall-consult--build-candidate
                         agent-recall-consult--search-fn
                         agent-recall-consult--position
                         agent-recall-consult--state
                         agent-recall-consult-search)))"##;
    let expect = expect![
        "OK (t t ((agent-recall-consult--ensure-consult nil nil) (agent-recall-consult--ripgrep-args nil nil) (agent-recall-consult--humanize-timestamp (basename) nil) (agent-recall-consult--build-candidate (file count line content proj-width count-width) nil) (agent-recall-consult--search-fn (input) nil) (agent-recall-consult--position (cand &optional find-file) nil) (agent-recall-consult--state nil nil) (agent-recall-consult-search nil (interactive nil))))"
    ];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_ensure_reports_missing_dependency_and_accepts_available_feature() {
    let elisp_form = r##"(let ((real-require
                           (symbol-function 'require)))
                      (list
                       (cl-letf (((symbol-function 'require)
                                  (lambda (feature &optional _file
                                                   _noerror)
                                    (and (eq feature 'consult)
                                         nil))))
                         (condition-case error-data
                             (agent-recall-consult--ensure-consult)
                           (error
                            (list (car error-data)
                                  (cadr error-data)))))
                       (cl-letf (((symbol-function 'require)
                                  (lambda (feature &optional file
                                                   noerror)
                                    (if (eq feature 'consult)
                                        t
                                      (funcall real-require
                                               feature file noerror)))))
                         (agent-recall-consult--ensure-consult))))"##;
    let expect = expect![[
        r#"OK ((user-error "Consult is not installed.  Install it to use ‘agent-recall-consult-search’") nil)"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_ripgrep_arguments_replace_executable_and_append_real_filters() {
    let elisp_form = r##"(setq consult-ripgrep-args
                           '("rg" "--null" "--line-number"))
                      (let ((agent-recall-rg-executable
                           "/opt/tools/rg-custom")
                          (agent-recall-search-extra-args
                           '("--follow" "--sort=modified"
                             "--hidden"))
                          (agent-recall-file-patterns
                           '("*.md" "*.org" "space name.*")))
                      (cl-letf (((symbol-function
                                 'consult--build-args)
                                (lambda (args)
                                  (append args
                                          '("--color=never")))))
                        (agent-recall-consult--ripgrep-args)))"##;
    let expect = expect![[
        r#"OK ("/opt/tools/rg-custom" "--null" "--line-number" "--color=never" "--follow" "--sort=modified" "--hidden" "--glob" "*.md" "--glob" "*.org" "--glob" "space name.*")"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_humanizes_both_timestamp_separators_and_preserves_invalid_basenames() {
    let elisp_form = r##"(mapcar
                      (lambda (basename)
                        (cons
                         basename
                         (agent-recall-consult--humanize-timestamp
                          basename)))
                      '("2026-04-30-15-32-21"
                        "2026-04-30T15-32-21"
                        "2026-12-01-00-05-09"
                        "2026-12-01-23-59-59"
                        "session-name"
                        "2026-01-02-03-04"
                        ""))"##;
    let expect = expect![[
        r#"OK (("2026-04-30-15-32-21" . "30 Apr 26 03:32 PM") ("2026-04-30T15-32-21" . "30 Apr 26 03:32 PM") ("2026-12-01-00-05-09" . "01 Dec 26 12:05 AM") ("2026-12-01-23-59-59" . "01 Dec 26 11:59 PM") ("session-name" . "session-name") ("2026-01-02-03-04" . "2026-01-02-03-04") ("" . ""))"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_candidate_aligns_columns_status_content_and_navigation_properties() {
    let elisp_form = r##"(let* ((agent-recall--index
                            (make-hash-table :test 'equal))
                           (resumable
                            "/virtual/alpha/2026-04-30-15-32-21.md")
                           (plain
                            "/virtual/long-project/2026-05-01T09-02-03.org"))
                      (puthash resumable
                               '(:project "alpha"
                                 :session-id "session-a")
                               agent-recall--index)
                      (puthash plain
                               '(:project "long-project"
                                 :session-id nil)
                               agent-recall--index)
                      (mapcar
                       (lambda (spec)
                         (pcase-let
                             ((`(,only ,file ,count ,line ,content)
                               spec))
                           (let* ((agent-recall-consult-resumable-only
                                   only)
                                  (candidate
                                   (agent-recall-consult--build-candidate
                                    file count line content 12 3)))
                             (list
                              (substring-no-properties candidate)
                              (get-text-property
                               0 'agent-recall-consult-file
                               candidate)
                              (get-text-property
                               0 'agent-recall-consult-line
                               candidate)
                              (mapcar
                               (lambda (position)
                                 (get-text-property
                                  position 'face candidate))
                               (list
                                (if only 0 2)
                                (string-match
                                 "\\[[0-9]+\\]" candidate)))))))
                       (list
                        (list t resumable 7 42
                              "first matching line")
                        (list nil resumable 105 9
                              "resumable content")
                        (list nil plain 2 3
                              "plain content"))))"##;
    let expect = expect![[
        r#"OK (("[alpha]        [7]   30 Apr 26 03:32 PM first matching line" "/virtual/alpha/2026-04-30-15-32-21.md" 42 (consult-file consult-line-number)) ("● [alpha]        [105] 30 Apr 26 03:32 PM resumable content" "/virtual/alpha/2026-04-30-15-32-21.md" 9 (consult-file consult-line-number)) ("○ [long-project] [2]   01 May 26 09:02 AM plain content" "/virtual/long-project/2026-05-01T09-02-03.org" 3 (consult-file consult-line-number)))"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_search_aggregates_real_rg_output_filters_sessions_truncates_and_highlights_once()
 {
    let elisp_form = r##"(setq consult--grep-match-regexp
                           "^\\([^:]+\\):\\([0-9]+\\):"
                           consult-grep-max-columns 12)
                      (let* ((one
                            "/virtual/alpha/2026-04-30-15-32-21.md")
                           (two
                            "/virtual/beta/2026-05-01-09-02-03.org")
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall-consult-resumable-only t)
                           highlights)
                      (puthash one
                               '(:project "alpha"
                                 :session-id "one")
                               agent-recall--index)
                      (puthash two
                               '(:project "beta"
                                 :session-id nil)
                               agent-recall--index)
                      (cl-letf (((symbol-function
                                 'agent-recall-consult--ensure-consult)
                                (lambda () t))
                               ((symbol-function
                                 'agent-recall--index-dirs)
                                (lambda ()
                                  '("/virtual/alpha"
                                    "/virtual/beta")))
                               ((symbol-function
                                 'agent-recall-consult--ripgrep-args)
                                (lambda () '("rg" "--null")))
                               ((symbol-function
                                 'consult--ripgrep-make-builder)
                                (lambda (dirs)
                                  (lambda (input)
                                    (cons
                                     (list "rg" input
                                           (mapconcat #'identity
                                                      dirs ","))
                                     (lambda (text)
                                       (push text highlights))))))
                               ((symbol-function 'call-process)
                                (lambda (&rest _)
                                  (insert
                                   one ":12:first result content is long\n"
                                   one ":20:second result\n"
                                   two ":3:hidden result\n")
                                  0)))
                        (let ((candidates
                               (agent-recall-consult--search-fn
                                "atomic")))
                          (list
                           (mapcar
                            (lambda (candidate)
                              (list
                               (substring-no-properties candidate)
                               (get-text-property
                                0 'agent-recall-consult-file
                                candidate)
                               (get-text-property
                                0 'agent-recall-consult-line
                                candidate)))
                            candidates)
                           (nreverse highlights)))))"##;
    let expect = expect![[
        r#"OK ((("[alpha] [2] 30 Apr 26 03:32 PM first result" "/virtual/alpha/2026-04-30-15-32-21.md" 12)) ("first result"))"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_search_all_mode_includes_nonresumable_with_status_and_count_alignment() {
    let elisp_form = r##"(setq consult--grep-match-regexp
                           "^\\([^:]+\\):\\([0-9]+\\):"
                           consult-grep-max-columns nil)
                      (let* ((one
                            "/virtual/a/2026-04-30-15-32-21.md")
                           (two
                            "/virtual/longer/2026-05-01-09-02-03.org")
                           (agent-recall--index
                            (make-hash-table :test 'equal))
                           (agent-recall-consult-resumable-only nil))
                      (puthash one
                               '(:project "a" :session-id "one")
                               agent-recall--index)
                      (puthash two
                               '(:project "longer" :session-id nil)
                               agent-recall--index)
                      (cl-letf (((symbol-function
                                 'agent-recall-consult--ensure-consult)
                                (lambda () t))
                               ((symbol-function
                                 'agent-recall--index-dirs)
                                (lambda () '("/virtual")))
                               ((symbol-function
                                 'agent-recall-consult--ripgrep-args)
                                (lambda () '("rg")))
                               ((symbol-function
                                 'consult--ripgrep-make-builder)
                                (lambda (_dirs)
                                  (lambda (_input)
                                    (cons '("rg") nil))))
                               ((symbol-function 'call-process)
                                (lambda (&rest _)
                                  (insert
                                   one ":1:one\n"
                                   two ":8:two\n"
                                   two ":9:again\n")
                                  0)))
                        (mapcar
                         #'substring-no-properties
                         (agent-recall-consult--search-fn "all"))))"##;
    let expect = expect![[
        r#"OK ("● [a]      [1] 30 Apr 26 03:32 PM one" "○ [longer] [2] 01 May 26 09:02 AM two")"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_position_opens_candidate_and_resolves_line_marker_or_nil() {
    let elisp_form = r##"(let* ((buffer (generate-new-buffer
                                     " *consult-position*"))
                           (candidate
                            (propertize "candidate"
                                        'agent-recall-consult-file
                                        "/virtual/session.md"
                                        'agent-recall-consult-line
                                        37))
                           calls)
                      (unwind-protect
                          (cl-letf (((symbol-function
                                     'consult--file-action)
                                    (lambda (file)
                                      (push (list 'default file)
                                            calls)
                                      buffer))
                                   ((symbol-function
                                     'consult--marker-from-line-column)
                                    (lambda (target line column)
                                      (push
                                       (list 'marker
                                             (buffer-name target)
                                             line column)
                                       calls)
                                      'marker-37)))
                            (list
                             (agent-recall-consult--position candidate)
                             (agent-recall-consult--position
                              candidate
                              (lambda (file)
                                (push (list 'custom file) calls)
                                buffer))
                             (agent-recall-consult--position
                              "no properties")
                             (nreverse calls)))
                        (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ((marker-37) (marker-37) nil ((default "/virtual/session.md") (marker " *consult-position*" 37 0) (custom "/virtual/session.md") (marker " *consult-position*" 37 0)))"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_state_coordinates_temporary_open_jump_preview_return_and_cleanup() {
    let elisp_form = r##"(let ((opened nil)
                          (events nil))
                      (cl-letf (((symbol-function
                                 'consult--temporary-files)
                                (lambda ()
                                  (lambda (&optional file)
                                    (if file
                                        (progn
                                          (push
                                           (list 'open file)
                                           events)
                                          (setq opened
                                                (get-buffer-create
                                                 " *preview*")))
                                      (push '(cleanup) events)))))
                               ((symbol-function 'consult--jump-state)
                                (lambda ()
                                  (lambda (action position)
                                    (push
                                     (list 'jump action position)
                                     events))))
                               ((symbol-function
                                 'agent-recall-consult--position)
                                (lambda (candidate
                                         &optional find-file)
                                  (list
                                   candidate
                                   (and find-file
                                        (buffer-name
                                         (funcall
                                          find-file
                                          "/virtual/session.md")))))))
                        (let ((state
                               (agent-recall-consult--state)))
                          (funcall state 'preview "candidate")
                          (funcall state 'return "candidate")
                          (funcall state 'exit nil)
                          (prog1 (nreverse events)
                            (when opened
                              (kill-buffer opened))))))"##;
    let expect = expect![[
        r#"OK ((open "/virtual/session.md") (jump preview ("candidate" " *preview*")) (jump return ("candidate" nil)) (cleanup) (open "/virtual/session.md") (jump exit (nil " *preview*")))"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}

#[test]
fn agent_recall_consult_search_wires_dynamic_collection_selection_and_transcript_mode() {
    let elisp_form = r##"(let ((agent-recall-auto-transcript-mode t)
                          captured
                          mode)
                      (cl-letf (((symbol-function
                                 'agent-recall-consult--ensure-consult)
                                (lambda () t))
                               ((symbol-function
                                 'agent-recall--index-dirs)
                                (lambda () '("/virtual/transcripts")))
                               ((symbol-function
                                 'consult--dynamic-collection)
                                (lambda (function)
                                  (list 'dynamic function)))
                               ((symbol-function
                                 'agent-recall-consult--state)
                                (lambda () 'state-function))
                               ((symbol-function 'consult--read)
                                (lambda (collection &rest args)
                                  (setq captured
                                        (list collection
                                              (plist-get args :prompt)
                                              (plist-get args :lookup)
                                              (plist-get args :state)
                                              (plist-get args :category)
                                              (plist-get args :history)
                                              (plist-get args :sort)))
                                  "selected"))
                               ((symbol-function 'buffer-file-name)
                                (lambda (&optional _buffer)
                                  "/work/.agent-shell/transcripts/a.md"))
                               ((symbol-function
                                 'agent-recall-transcript-mode)
                                (lambda (arg) (setq mode arg))))
                        (agent-recall-consult-search))
                      (list captured mode))"##;
    let expect = expect![[
        r#"OK (((dynamic agent-recall-consult--search-fn) "Recall: " consult--lookup-member state-function consult-grep (:input consult--grep-history) nil) 1)"#
    ]];
    assert_agent_recall_consult_parity(elisp_form, expect);
}
