use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn ahg_mq_patch_completion_filters_prefixes_and_handles_command_failure() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function 'ahg-call-process)
                            (lambda (command &optional _arguments _global)
                              (push command calls)
                              (if (equal command "qseries")
                                  (progn
                                    (insert
                                     "alpha\nalphabet\nbeta\nrelease-λ\n\n")
                                    0)
                                1))))
                        (list
                         (ahg-complete-mq-patch-name "")
                         (ahg-complete-mq-patch-name "al")
                         (ahg-complete-mq-patch-name "beta")
                         (ahg-complete-mq-patch-name "z")
                         (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("alpha" "alphabet" "beta" "release-λ") ("alpha" "alphabet") ("beta") nil ("qseries" "qseries" "qseries" "qseries"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_mq_patch_ewoc_combines_series_applied_state_guards_and_navigation() {
    let elisp_form = r##"(with-temp-buffer
                      (setq default-directory "/repo/")
                      (let ((ew (ewoc-create #'ahg-mq-patch-pp)))
                        (setq ewoc ew)
                        (ahg-mq-patches-insert-contents
                         ew
                         '("base" "feature" "release")
                         '("base" "feature")
                         '(("base" "unguarded")
                           ("feature" "+linux" "-windows")
                           ("release" "+prod")))
                        (goto-char (ewoc-location (ewoc-nth ew 1)))
                        (let ((current (ahg-mq-patches-patch-at-point)))
                          (list
                           current
                           (mapcar #'identity
                                   (ewoc-collect ew #'identity))
                           (buffer-substring-no-properties
                            (point-min) (point-max))))))"##;
    let expect = expect![[
        r#"OK ("feature" ((0 ("base" . #1=("feature")) "base" ("unguarded")) (1 #1# "feature" ("+linux" "-windows")) (2 nil "release" ("+prod"))) "\n     0 |  *  | base                                                             \n     1 |  *  | feature (+linux -windows)                                        \n     2 |     | release (+prod)                                                  \n\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_qnew_handles_dirty_abort_force_selected_files_and_log_edit_boundaries() {
    let elisp_form = r##"(let (dirty commands messages edits)
                      (cl-letf
                          (((symbol-function 'ahg-uncommitted-changes-p)
                            (lambda (&optional _root) dirty))
                           ((symbol-function 'ahg-status-get-marked)
                            (lambda (&rest _arguments)
                              '((t "M" . "src/a.el")
                                (t "A" . "src/b.el"))))
                           ((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) "/repo/"))
                           ((symbol-function 'ahg-generic-command)
                            (lambda (command arguments &rest _rest)
                              (push (list command arguments) commands)
                              'fake-process))
                           ((symbol-function 'ahg-log-edit)
                            (lambda (_callback file-list buffer
                                     &optional message content)
                              (push
                               (list
                                (funcall file-list)
                                (buffer-name buffer)
                                message content)
                               edits)
                              (kill-buffer buffer)))
                           ((symbol-function 'message)
                            (lambda (format-string &rest arguments)
                              (push (apply #'format format-string arguments)
                                    messages))))
                        (let ((major-mode 'fundamental-mode)
                              (ahg-diff-use-git-format t))
                          (setq dirty t)
                          (ahg-qnew "blocked" nil nil)
                          (setq dirty nil)
                          (ahg-qnew "clean" nil nil))
                        (let ((major-mode 'ahg-status-mode)
                              (ahg-diff-use-git-format nil))
                          (setq dirty t)
                          (ahg-qnew "selected" t nil)
                          (ahg-qnew "documented" t t))
                        (list
                         (nreverse commands)
                         (nreverse edits)
                         (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ((("qnew" ("--git" "--force" "clean")) ("qnew" ("--force" "selected" "src/a.el" "src/b.el"))) ((("documented" "src/a.el" "src/b.el") "*aHg-log*" nil nil)) ("mq command qnew aborted."))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_qrefresh_builds_short_git_and_selected_file_arguments_for_each_mode() {
    let elisp_form = r##"(let (commands)
                      (cl-letf
                          (((symbol-function 'ahg-status-get-marked)
                            (lambda (&rest _arguments)
                              '((t "M" . "src/a.el")
                                (t "M" . "src/b.el"))))
                           ((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) "/repo/"))
                           ((symbol-function 'ahg-generic-command)
                            (lambda (command arguments &rest _rest)
                              (push (list command arguments) commands)
                              'fake-process)))
                        (let ((major-mode 'ahg-status-mode)
                              (ahg-qrefresh-use-short-flag t)
                              (ahg-diff-use-git-format t))
                          (ahg-qrefresh nil))
                        (let ((major-mode 'ahg-status-mode)
                              (ahg-qrefresh-use-short-flag nil)
                              (ahg-diff-use-git-format nil))
                          (ahg-qrefresh nil))
                        (let ((major-mode 'fundamental-mode)
                              (ahg-qrefresh-use-short-flag t)
                              (ahg-diff-use-git-format t))
                          (ahg-qrefresh nil))
                        (nreverse commands)))"##;
    let expect = expect![[
        r#"OK (("qrefresh" ("--short" "--git" "src/a.el" "src/b.el")) ("qrefresh" ("src/a.el" "src/b.el")) ("qrefresh" ("--git")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_qgoto_qmove_qswitch_qpop_qdelete_and_qdiff_dispatch_exact_commands() {
    let elisp_form = r##"(let (commands applied)
                      (cl-letf
                          (((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) "/repo/"))
                           ((symbol-function 'ahg-mq-applied-patches-p)
                            (lambda (&optional _root) applied))
                           ((symbol-function 'ahg-push-window-configuration)
                            (lambda () nil))
                           ((symbol-function 'ahg-generic-command)
                            (lambda (command arguments &rest _rest)
                              (push (list command arguments) commands)
                              'fake-process)))
                        (ahg-qgoto "feature" nil)
                        (ahg-qgoto "feature" t)
                        (ahg-qmove "release" nil)
                        (setq applied nil)
                        (ahg-qswitch "next" t)
                        (setq applied t)
                        (ahg-qswitch "other" nil)
                        (ahg-qpop-all nil)
                        (ahg-qpop-all t)
                        (ahg-qdelete "obsolete")
                        (let ((ahg-diff-use-git-format t))
                          (ahg-qdiff '("src/a.el")))
                        (prog1
                            (nreverse commands)
                          (when (get-buffer "*aHg diff*")
                            (kill-buffer "*aHg diff*")))))"##;
    let expect = expect![[
        r#"OK (("qgoto" ("feature")) ("qgoto" ("-f" "feature")) ("qpush" ("--move" "release")) ("qpush" ("--move" "-f" "next")) ("qpop" ("-a")) ("qpop" ("-a")) ("qpop" ("-f" "-a")) ("qdelete" ("obsolete")) ("qdiff" ("--git" "src/a.el")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_qapply_uses_repository_patch_file_force_mode_and_refreshes_worktree() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          calls messages refreshes)
                      (make-directory (expand-file-name ".hg/patches" root) t)
                      (with-temp-file
                          (expand-file-name ".hg/patches/feature" root)
                        (insert "patch body"))
                      (cl-letf
                          (((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) root))
                           ((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              0))
                           ((symbol-function 'set-window-configuration)
                            (lambda (&rest _arguments) nil))
                           ((symbol-function 'ahg-status-maybe-refresh)
                            (lambda (directory)
                              (push directory refreshes)))
                           ((symbol-function 'message)
                            (lambda (format-string &rest arguments)
                              (push (apply #'format format-string arguments)
                                    messages))))
                        (ahg-qapply "feature" nil)
                        (ahg-qapply "feature" t)
                        (list (nreverse calls)
                              (nreverse refreshes)
                              (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ((("patch" ("-p" "1" "--no-commit" "[ORACLE-SANDBOX]/repo/.hg/patches/feature")) ("patch" ("--force" "-p" "1" "--no-commit" "[ORACLE-SANDBOX]/repo/.hg/patches/feature"))) ("[ORACLE-SANDBOX]/repo/" "[ORACLE-SANDBOX]/repo/") ("Applied patch feature to the working copy" "Applied patch feature to the working copy"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_mq_patch_buffers_are_canonical_and_show_practical_queue_state() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (buffer (ahg-mq-get-patches-buffer root)))
                      (unwind-protect
                          (progn
                            (ahg-mq-show-patches-buffer
                             buffer
                             '("base" "feature" "release")
                             '("base")
                             '(("base" "unguarded")
                               ("feature" "+linux")
                               ("release" "+prod"))
                             root t nil)
                            (with-current-buffer buffer
                              (list
                               (buffer-name)
                               default-directory
                               major-mode
                               buffer-read-only
                               (mapcar #'identity
                                       (ewoc-collect ewoc #'identity))
                               (buffer-substring-no-properties
                                (point-min) (point-max)))))
                        (when (buffer-live-p buffer)
                          (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ("*aHg mq patches for: [ORACLE-SANDBOX]/repo/*" "[ORACLE-SANDBOX]/repo/" ahg-mq-patches-mode t ((0 ("base") "base" ("unguarded")) (1 nil "feature" ("+linux")) (2 nil "release" ("+prod"))) "mq patch queue for [ORACLE-SANDBOX]/repo/\n\n--------------------------------------------------------------------------------\n Index | App | Patch (Guards)\n--------------------------------------------------------------------------------\n     0 |  *  | base                                                             \n     1 |     | feature (+linux)                                                 \n     2 |     | release (+prod)                                                  \n--------------------------------------------------------------------------------\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_mq_selected_patch_actions_confirm_and_route_current_ewoc_patch() {
    let elisp_form = r##"(let (calls prompts dirty)
                      (with-temp-buffer
                        (let ((ew (ewoc-create #'ahg-mq-patch-pp)))
                          (setq-local ewoc ew)
                          (ewoc-enter-last
                           ew '(0 t "feature" ("+linux")))
                          (goto-char (ewoc-location (ewoc-nth ew 0)))
                          (cl-letf
                              (((symbol-function 'ahg-y-or-n-p)
                                (lambda (prompt)
                                  (push prompt prompts)
                                  t))
                               ((symbol-function 'ahg-uncommitted-changes-p)
                                (lambda (&optional _root) dirty))
                               ((symbol-function 'ahg-qgoto)
                                (lambda (&rest arguments)
                                  (push (cons 'goto arguments) calls)))
                               ((symbol-function 'ahg-qmove)
                                (lambda (&rest arguments)
                                  (push (cons 'move arguments) calls)))
                               ((symbol-function 'ahg-qswitch)
                                (lambda (&rest arguments)
                                  (push (cons 'switch arguments) calls)))
                               ((symbol-function 'ahg-qapply)
                                (lambda (&rest arguments)
                                  (push (cons 'apply arguments) calls)))
                               ((symbol-function 'ahg-qdelete)
                                (lambda (&rest arguments)
                                  (push (cons 'delete arguments) calls))))
                            (setq dirty nil)
                            (ahg-mq-patches-goto-patch)
                            (setq dirty t)
                            (ahg-mq-patches-moveto-patch)
                            (ahg-mq-patches-switchto-patch)
                            (ahg-mq-patches-apply-patch)
                            (ahg-mq-patches-delete-patch)
                            (list (nreverse calls)
                                  (nreverse prompts))))))"##;
    let expect = expect![[
        r#"OK (((goto "feature" nil) (move "feature" t) (switch "feature" t) (apply "feature" t) (delete "feature")) ("Go to patch feature? " "Move to patch feature? " "Overwrite local changes? " "Switch to patch feature? " "Overwrite local changes? " "Apply patch feature to the working copy? " "Working copy contains local changes, proceed anyway? " "Delete patch feature? "))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
