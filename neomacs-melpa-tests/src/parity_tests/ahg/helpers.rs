use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn ahg_line_position_round_trip_clamps_columns_on_shorter_lines() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "first line\nx\nthird λ line\n")
                      (goto-char (point-min))
                      (forward-line 2)
                      (forward-char 7)
                      (let ((position (ahg-line-point-pos)))
                        (ahg-goto-line 1)
                        (let ((first-point (point)))
                          (ahg-goto-line-point position)
                          (let ((roundtrip
                                 (list (line-number-at-pos)
                                       (- (point) (point-at-bol)))))
                            (ahg-goto-line-point '(2 . 99))
                            (list position
                                  first-point
                                  roundtrip
                                  (list (line-number-at-pos)
                                        (- (point) (point-at-bol))))))))"##;
    let expect = expect!["OK ((3 . 7) 1 (3 7) (2 1))"];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_string_matching_and_revset_detection_preserve_outer_match_data() {
    let elisp_form = r##"(progn
                      (string-match "\\(outer\\)" "outer")
                      (let ((before (match-data)))
                        (list
                         (ahg-string-match-p "needle" "hay needle stack")
                         (ahg-string-match-p "missing" "haystack")
                         (mapcar #'ahg-maybe-revset
                                 '("17" "tip" "17:0" "ancestors(.)"
                                   "branch(default)" "name-with-dash"))
                         before
                         (match-data))))"##;
    let expect = expect!["OK (4 nil (nil nil t t t nil) (0 5 0 5) (6 7))"];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_worktree_state_helpers_parse_clean_dirty_missing_and_mq_applied_results() {
    let elisp_form = r##"(let (scenario calls)
                      (cl-letf
                          (((symbol-function 'ahg-cd)
                            (lambda (directory)
                              (push (list 'cd directory) calls)
                              (not (equal directory "/missing/"))))
                           ((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments scenario) calls)
                              (pcase (list scenario command)
                                (`(clean "id") (insert "17\n") 0)
                                (`(dirty "id") (insert "17+\n") 0)
                                (`(tracked "status")
                                 (insert "C src/main.el\n")
                                 0)
                                (`(modified "status")
                                 (insert "M src/main.el\n")
                                 0)
                                (`(applied "qapplied")
                                 (insert "base\nfeature\n")
                                 0)
                                (`(none "qapplied") 0)
                                (_ 1)))))
                        (list
                         (progn
                           (setq scenario 'clean)
                           (ahg-uncommitted-changes-p "/repo/"))
                         (progn
                           (setq scenario 'dirty)
                           (ahg-uncommitted-changes-p "/repo/"))
                         (progn
                           (setq scenario 'tracked)
                           (ahg-file-status "/repo/src/main.el"))
                         (progn
                           (setq scenario 'modified)
                           (ahg-file-status "/repo/src/main.el"))
                         (progn
                           (setq scenario 'applied)
                           (ahg-mq-applied-patches-p "/repo/"))
                         (progn
                           (setq scenario 'none)
                           (ahg-mq-applied-patches-p "/repo/"))
                         (progn
                           (setq scenario 'clean)
                           (ahg-uncommitted-changes-p "/missing/"))
                         (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (nil t "C" "M" t nil nil ((cd "/repo/") ("id" ("-n") clean) (cd "/repo/") ("id" ("-n") dirty) (cd "/repo/src/") ("status" ("-A" "main.el") tracked) (cd "/repo/src/") ("status" ("-A" "main.el") modified) (cd "/repo/") ("qapplied" nil applied) (cd "/repo/") ("qapplied" nil none) (cd "/missing/")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_path_helpers_expand_against_repo_root_and_cd_reports_real_failures() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (nested (expand-file-name "src/lib" root)))
                      (make-directory (expand-file-name ".hg" root) t)
                      (make-directory nested t)
                      (list
                       (ahg-abspath "src/main.el" root)
                       (let ((default-directory
                               (file-name-as-directory nested)))
                         (ahg-abspath "../sibling.el"))
                       (with-temp-buffer
                         (let ((before default-directory))
                           (list
                            (ahg-cd root)
                            default-directory
                            (ahg-cd
                             (expand-file-name "does-not-exist" sandbox))
                            default-directory
                            before)))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/repo/src/main.el" "[ORACLE-SANDBOX]/sibling.el" ("[ORACLE-SANDBOX]/repo/" "[ORACLE-SANDBOX]/repo/" nil "[ORACLE-SANDBOX]/repo/" "[ORACLE-SANDBOX]/"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_confirmation_helper_selects_short_or_long_prompt_without_changing_result() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function 'y-or-n-p)
                            (lambda (prompt)
                              (push (list 'short prompt) calls)
                              'short-answer))
                           ((symbol-function 'yes-or-no-p)
                            (lambda (prompt)
                              (push (list 'long prompt) calls)
                              'long-answer)))
                        (list
                         (let ((ahg-yesno-short-prompt t))
                           (ahg-y-or-n-p "Proceed? "))
                         (let ((ahg-yesno-short-prompt nil))
                           (ahg-y-or-n-p "Proceed carefully? "))
                         (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (short-answer long-answer ((short "Proceed? ") (long "Proceed carefully? ")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_error_message_buffer_enters_command_mode_and_appends_unicode_diagnostics() {
    let elisp_form = r##"(let ((buffer
                           (generate-new-buffer
                            " *ahg-error-parity*")))
                      (unwind-protect
                          (cl-letf
                              (((symbol-function 'pop-to-buffer)
                                (lambda (target &rest _arguments)
                                  (set-buffer target)
                                  target)))
                            (with-current-buffer buffer
                              (insert "command output\n"))
                            (ahg-show-error-msg
                             "failure: repository λ is unavailable"
                             buffer)
                            (with-current-buffer buffer
                              (list
                               major-mode
                               mode-name
                               buffer-read-only
                               (lookup-key
                                (current-local-map) (kbd "q"))
                               (buffer-substring-no-properties
                                (point-min) (point-max)))))
                        (when (buffer-live-p buffer)
                          (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (ahg-command-mode "aHg command" t ahg-buffer-quit "command output\nfailure: repository λ is unavailable\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_log_command_map_installs_all_cross_view_switches_without_overwriting_other_keys() {
    let elisp_form = r##"(let ((map (make-sparse-keymap)))
                      (define-key map "x" #'ignore)
                      (ahg-add-log-commands map)
                      (list
                       (mapcar
                        (lambda (key)
                          (cons key (lookup-key map key)))
                        '("l" "L" "G" "H" "T" "B" "x"))
                       ahg-log-commands-map))"##;
    let expect = expect![[
        r#"OK ((("l" . ahg-short-log) ("L" . ahg-log) ("G" . ahg-glog) ("H" . ahg-heads) ("T" . ahg-tags) ("B" . ahg-bookmarks) ("x" . ignore)) (("l" . ahg-short-log) ("L" . ahg-log) ("G" . ahg-glog) ("H" . ahg-heads) ("T" . ahg-tags) ("B" . ahg-bookmarks)))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_version_and_custom_hg_executable_are_observable_through_public_helpers() {
    let elisp_form = r##"(let (messages)
                      (cl-letf
                          (((symbol-function 'message)
                            (lambda (format-string &rest arguments)
                              (let ((rendered
                                     (apply #'format
                                            format-string arguments)))
                                (push rendered messages)
                                rendered))))
                        (let ((ahg-hg-command "/opt/custom hg"))
                          (list
                           (ahg-version)
                           (ahg-hg-command)
                           (nreverse messages)))))"##;
    let expect = expect![[r#"OK ("aHg version 1.0.0" "/opt/custom hg" ("aHg version 1.0.0"))"#]];
    assert_ahg_parity(elisp_form, expect);
}
