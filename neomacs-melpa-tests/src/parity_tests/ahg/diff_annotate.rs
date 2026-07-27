use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn ahg_diff_mode_removes_crlf_noise_and_installs_revision_aware_actions() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "diff --git a/src/main.el b/src/main.el\r\n"
                       "--- a/src/main.el\r\n"
                       "+++ b/src/main.el\r\n"
                       "@@ -1 +1 @@\r\n"
                       "-old\r\n+new\r\n")
                      (setq default-directory
                            (file-name-as-directory
                             (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                      (make-directory
                       (expand-file-name ".hg" default-directory) t)
                      (ahg-set-diff-mode '("oldrev" . "newrev"))
                      (list
                       major-mode
                       buffer-read-only
                       (bound-and-true-p ahg-diff-revs)
                       (lookup-key ahg-diff-mode-map (kbd "e"))
                       (lookup-key ahg-diff-mode-map (kbd "q"))
                       (save-excursion
                         (goto-char (point-min))
                         (search-forward "\r" nil t))
                       (buffer-substring-no-properties
                        (point-min) (point-max))))"##;
    let expect = expect![[
        r#"OK (ahg-diff-mode t ("oldrev" . "newrev") ahg-diff-ediff ahg-buffer-quit nil "diff --git a/src/main.el b/src/main.el\n--- a/src/main.el\n+++ b/src/main.el\n@@ -1 +1 @@\n-old\n+new\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_diff_commands_build_git_disjoint_revision_and_file_arguments() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function 'ahg-push-window-configuration)
                            (lambda () nil))
                           ((symbol-function 'ahg-generic-command)
                            (lambda (command arguments &rest rest)
                              (push (list command arguments
                                          (and (nth 1 rest)
                                               (buffer-name (nth 1 rest))))
                                    calls)
                              'fake-process)))
                        (let ((ahg-diff-use-git-format t)
                              (default-directory "/repo/"))
                          (ahg-diff "tip" "17" '("src/a.el" "docs/b.md"))
                          (ahg-diff "tip" nil nil)
                          (ahg-diff-c "18" '("src/a.el")))
                        (let ((ahg-diff-use-git-format nil)
                              (default-directory "/repo/"))
                          (ahg-diff nil nil '("plain.txt")))
                        (prog1
                            (nreverse calls)
                          (when (get-buffer "*aHg-diff*")
                            (kill-buffer "*aHg-diff*")))))"##;
    let expect = expect![[
        r#"OK (("diff" ("--git" "-r" "tip" "-r" "17" "src/a.el" "docs/b.md") "*aHg-diff*") ("diff" ("--git" "-r" "tip") "*aHg-diff*") ("diff" ("--git" "-c" "18" "src/a.el") "*aHg-diff*") ("diff" ("plain.txt") "*aHg-diff*"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_ediff_buffer_loader_reads_worktree_and_mocked_historical_revision_content() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (file (expand-file-name "src/main.el" root))
                          buffers calls)
                      (make-directory (expand-file-name ".hg" root) t)
                      (make-directory (file-name-directory file) t)
                      (with-temp-file file
                        (insert "working λ\n"))
                      (cl-letf
                          (((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              (insert "historical\n")
                              0)))
                        (unwind-protect
                            (progn
                              (push
                               (ahg-diff-get-ediff-buffer
                                root "src/main.el" nil)
                               buffers)
                              (push
                               (ahg-diff-get-ediff-buffer
                                root "src/main.el" "abc123")
                               buffers)
                              (list
                               (mapcar
                                (lambda (buffer)
                                  (with-current-buffer buffer
                                    (list (buffer-name)
                                          (buffer-string)
                                          default-directory)))
                                (nreverse buffers))
                               (nreverse calls)))
                          (mapc
                           (lambda (buffer)
                             (when (buffer-live-p buffer)
                               (kill-buffer buffer)))
                           buffers))))"##;
    let expect = expect![[
        r#"OK ((("*aHg-diff-*workdir*-src/main.el" "working λ\n" "[ORACLE-SANDBOX]/repo/") ("*aHg-diff-abc123-src/main.el" "historical\n" "[ORACLE-SANDBOX]/repo/")) (("cat" ("-r" "abc123" "src/main.el"))))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_hsv_color_conversion_covers_gray_boundaries_and_all_six_color_sectors() {
    let elisp_form = r##"(mapcar
                      (lambda (color)
                        (append color (list (apply #'ahg-hsv-to-hex color))))
                      '((0.0 0.0 0.0)
                        (0.0 0.0 1.0)
                        (0.0 1.0 1.0)
                        (0.1666667 1.0 1.0)
                        (0.3333333 1.0 1.0)
                        (0.5 1.0 1.0)
                        (0.6666667 1.0 1.0)
                        (0.8333333 1.0 1.0)
                        (1.0 1.0 1.0)
                        (0.42 0.9 0.75)))"##;
    let expect = expect![[
        r##"OK ((0.0 0.0 0.0 "#000000000000") (0.0 0.0 1.0 "#FFFFFFFFFFFF") (0.0 1.0 1.0 "#FFFF00000000") (0.1666667 1.0 1.0 "#FFFEFFFF0000") (0.3333333 1.0 1.0 "#0000FFFF0000") (0.5 1.0 1.0 "#0000FFFFFFFF") (0.6666667 1.0 1.0 "#00000000FFFF") (0.8333333 1.0 1.0 "#FFFE0000FFFF") (1.0 1.0 1.0 "#FFFF00000000") (0.42 0.9 0.75 "#1333BFFF6D0D"))"##
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_annotate_line_helpers_read_revision_and_original_line_properties() {
    let elisp_form = r##"(progn
                      (require 'thingatpt)
                      (with-temp-buffer
                      (insert
                       "alice 17 2026-07-10:  41:first line\n"
                       "bob    8 2026-07-09:   9:second line\n")
                      (put-text-property
                       (point-min)
                       (save-excursion (goto-char (point-min)) (point-at-eol))
                       'ahg-line-number "41")
                      (put-text-property
                       (save-excursion (goto-char (point-min))
                                       (forward-line 1)
                                       (point-at-bol))
                       (point-max)
                       'ahg-line-number "9")
                      (goto-char (point-min))
                      (let ((first
                             (list (ahg-annotate-revision-at-line)
                                   (ahg-annotate-line-at-line))))
                        (forward-line 1)
                        (list first
                              (list (ahg-annotate-revision-at-line)
                                    (ahg-annotate-line-at-line))))))"##;
    let expect = expect![[
        r#"OK ((#("17" 0 2 (ahg-line-number "41")) 41) (#("8" 0 1 (ahg-line-number "9")) 9))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_annotate_region_assigns_revision_color_and_changeset_tooltip() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "alice 10 line one\nalice 10 line two\nbob 20 line three\n")
                      (setq-local ahg-annotate-min-revision 10)
                      (setq-local ahg-annotate-max-revision 20)
                      (setq-local
                       ahg-changeset-descriptions
                       (let ((table (make-hash-table)))
                         (puthash 10 "old change" table)
                         (puthash 20 "new change" table)
                         table))
                      (ahg-annotate-region
                         (point-min)
                         (save-excursion
                           (goto-char (point-min))
                           (forward-line 2)
                           (point))
                         "10")
                        (ahg-annotate-region
                         (save-excursion
                           (goto-char (point-min))
                           (forward-line 2)
                           (point))
                         (point-max)
                         "20")
                        (list
                         (mapcar
                          (lambda (position)
                            (let ((face (get-text-property position 'face)))
                              (list
                               (and face (symbol-name face))
                               (get-text-property position 'help-echo))))
                          (list (point-min)
                                (save-excursion
                                  (goto-char (point-min))
                                  (forward-line 2)
                                  (point))))
                         (ahg-hsv-to-hex 0.7 0.9 0.9)
                         (ahg-hsv-to-hex 0.0 0.9 0.9)))"##;
    let expect = expect![[
        r##"OK ((("ahg-annotate-face-4082170AE665" "old change") ("ahg-annotate-face-E665170A170A" "new change")) "#4082170AE665" "#E665170A170A")"##
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_annotate_actions_dispatch_revision_file_and_line_context() {
    let elisp_form = r##"(let (calls messages)
                      (cl-letf
                          (((symbol-function 'ahg-annotate-revision-at-line)
                            (lambda () "17"))
                           ((symbol-function 'ahg-annotate-line-at-line)
                            (lambda () 41))
                           ((symbol-function 'ahg-log)
                            (lambda (&rest arguments)
                              (push (cons 'log arguments) calls)))
                           ((symbol-function 'ahg-diff-c)
                            (lambda (&rest arguments)
                              (push (cons 'diff-c arguments) calls)))
                           ((symbol-function 'ahg-annotate)
                            (lambda (&rest arguments)
                              (push (cons 'annotate arguments) calls)))
                           ((symbol-function 'message)
                            (lambda (format-string &rest arguments)
                              (push (apply #'format format-string arguments)
                                    messages))))
                        (with-temp-buffer
                          (setq-local
                           ahg-annotate-current-file "/repo/src/main.el")
                          (ahg-annotate-log)
                          (ahg-annotate-diff)
                          (ahg-annotate-annotate)
                          (ahg-annotate-uncover)
                          (cl-letf
                              (((symbol-function
                                 'ahg-annotate-revision-at-line)
                                (lambda () "0")))
                            (ahg-annotate-uncover)))
                        (list (nreverse calls)
                              (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (((log "17" nil) (diff-c "17" ("/repo/src/main.el")) (annotate "/repo/src/main.el" "17" 41) (annotate "/repo/src/main.el" "16" 41)) ("Already at revision zero"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_current_file_diff_entrypoints_route_by_mode_file_and_revision_prompt() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function 'call-interactively)
                            (lambda (command &optional _record _keys)
                              (push (list 'interactive command) calls)))
                           ((symbol-function 'read-string)
                            (lambda (&rest _arguments) "release"))
                           ((symbol-function 'ahg-diff)
                            (lambda (&rest arguments)
                              (push (cons 'diff arguments) calls)))
                           ((symbol-function 'ahg-diff-ediff)
                            (lambda (&rest arguments)
                              (push (cons 'ediff arguments) calls)))
                           ((symbol-function 'ahg-rev-id)
                            (lambda (revision &optional _which)
                              (concat "resolved:" revision)))
                           ((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) "/repo/")))
                        (with-temp-buffer
                          (setq major-mode 'ahg-status-mode)
                          (ahg-diff-cur-file nil)
                          (ahg-diff-ediff-cur-file nil))
                        (with-temp-buffer
                          (setq major-mode 'ahg-short-log-mode)
                          (ahg-diff-cur-file nil))
                        (with-temp-buffer
                          (setq major-mode 'ahg-log-mode)
                          (ahg-diff-cur-file nil))
                        (with-temp-buffer
                          (setq buffer-file-name "/repo/src/main.el")
                          (ahg-diff-cur-file t)
                          (ahg-diff-ediff-cur-file nil)
                          (ahg-diff-ediff-cur-file t))
                        (nreverse calls)))"##;
    let expect = expect![[
        r#"OK ((interactive ahg-status-diff) (interactive ahg-status-diff-ediff) (interactive ahg-short-log-view-diff) (interactive ahg-log-view-diff) (diff "release" nil ("/repo/src/main.el")) (ediff "src/main.el") (ediff "src/main.el"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
