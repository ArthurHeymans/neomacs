use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn ahg_root_discovers_nested_mercurial_repository_and_handles_outside_paths() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (repo (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (nested (file-name-as-directory
                                   (expand-file-name "src/deep" repo)))
                          (outside (file-name-as-directory
                                    (expand-file-name "outside" sandbox))))
                      (make-directory (expand-file-name ".hg" repo) t)
                      (make-directory nested t)
                      (make-directory outside t)
                      (list
                       (let ((default-directory nested))
                         (ahg-root))
                       (let ((default-directory repo))
                         (ahg-root))
                       (let ((default-directory outside))
                         (ahg-root t))
                       (let ((default-directory outside))
                         (condition-case error-data
                             (ahg-root)
                           (error
                            (list (car error-data)
                                  (cadr error-data)))))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/repo/" "[ORACLE-SANDBOX]/repo/" "[ORACLE-SANDBOX]/outside/" (error "aHg: no repository found in [ORACLE-SANDBOX]/outside/"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_identify_and_summary_run_real_command_sequences_and_fallbacks() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function 'ahg-cd)
                            (lambda (directory)
                              (push (list 'cd directory) calls)
                              t))
                           ((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              (cond
                               ((equal command "identify")
                                (insert "17+ default tip\n")
                                0)
                               ((equal command "summary")
                                (insert
                                 (if ahg-summary-remote
                                     "branch: default\nparent: 17:abc\nupdate: 2 new changesets\nremote failed\n"
                                   "branch: default\nparent: 17:abc\ncommit: 1 modified\n"))
                                (if ahg-summary-remote 1 0))
                               ((equal command "log")
                                (insert "r123\n0123456789abcdef\n")
                                0)
                               (t 1)))))
                        (list
                         (ahg-identify "/work/repo")
                         (let ((ahg-summary-remote nil)
                               (ahg-summary-git-svn-info t))
                           (ahg-summary-info "/work/repo"))
                         (let ((ahg-summary-remote t)
                               (ahg-summary-git-svn-info nil))
                           (ahg-summary-info "/work/repo"))
                         (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("Id: 17+ default tip" "parent: 17:abc\ncommit: 1 modified\nsvn:    r123\ngit:    0123456789abcdef" "parent: 17:abc\nupdate: 2 new changesets\nremote failed" (("identify" ("-nibt")) (cd "/work/repo") ("summary" nil) ("log" ("-r" "." "--template" "{svnrev}\n{gitnode}\n")) (cd "/work/repo") ("summary" ("--remote"))))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_sandboxed_hg_executable_drives_an_end_to_end_sync_repository_workflow() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (nested (file-name-as-directory
                                   (expand-file-name "src/lib" root)))
                          (source (expand-file-name "src/main.el" root))
                          (fake-hg (expand-file-name "bin/hg-fixture" sandbox)))
                      (make-directory (expand-file-name ".hg" root) t)
                      (make-directory nested t)
                      (make-directory (file-name-directory fake-hg) t)
                      (with-temp-file source
                        (insert "working copy\n"))
                      (with-temp-file fake-hg
                        (insert
                         "#!/bin/sh\n"
                         "if [ \"$1\" = \"--config\" ]; then shift 2; fi\n"
                         "command=$1\n"
                         "shift\n"
                         "case \"$command\" in\n"
                         "  identify) printf '3+ default tip\\n' ;;\n"
                         "  summary) printf 'branch: default\\nparent: 3:abc123\\ncommit: 1 modified\\n' ;;\n"
                         "  id) printf '3+\\n' ;;\n"
                         "  status) printf 'M src/main.el\\n' ;;\n"
                         "  qapplied) printf 'base\\nfeature\\n' ;;\n"
                         "  files) printf 'src/main.el\\ndocs/guide.md\\n' ;;\n"
                         "  log)\n"
                         "    case \"$*\" in\n"
                         "      *bookmarks*) printf 'work release\\n' ;;\n"
                         "      *) printf 'abc123 ' ;;\n"
                         "    esac ;;\n"
                         "  *) printf 'unsupported command: %s\\n' \"$command\" >&2; exit 9 ;;\n"
                         "esac\n"))
                      (set-file-modes fake-hg #o755)
                      (let ((default-directory nested)
                            (ahg-hg-command fake-hg)
                            (ahg-summary-remote nil)
                            (ahg-summary-git-svn-info nil))
                        (list
                         (ahg-root)
                         (ahg-identify root)
                         (ahg-summary-info root)
                         (ahg-file-status source)
                         (ahg-uncommitted-changes-p root)
                         (ahg-rev-id "tip")
                         (ahg-get-bookmarks "tip")
                         (ahg-mq-applied-patches-p root)
                         (with-temp-buffer
                           (let ((status
                                  (ahg-call-process
                                   "files" '("glob:**.el"))))
                             (list status (buffer-string))))
                         (ahg-manifest-grep-get-files "*.el"))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/repo/" "Id: 3+ default tip" "parent: 3:abc123\ncommit: 1 modified" "M" t "abc123" ("work" "release") t (0 "src/main.el\ndocs/guide.md\n") "[ORACLE-SANDBOX]/bin/hg-fixture files -0 'glob:*.el'")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_pretty_printer_preserves_codes_names_faces_and_marked_state() {
    let elisp_form = r##"(with-temp-buffer
                      (dolist (data '((nil "M" . "src/main.el")
                                      (t "A" . "new file.el")
                                      (nil "?" . "notes λ.txt")
                                      (nil "X" . "unknown")))
                        (let ((start (point)))
                          (ahg-status-pp data)
                          (insert "\n")
                          (put-text-property
                           start (1- (point)) 'captured-data data)))
                      (goto-char (point-min))
                      (let (lines)
                        (while (not (eobp))
                          (push
                           (list
                            (buffer-substring-no-properties
                             (point-at-bol) (point-at-eol))
                            (get-text-property (1+ (point-at-bol)) 'face)
                            (get-text-property (1+ (point-at-bol)) 'mouse-face)
                            (get-text-property (1+ (point-at-bol)) 'keymap)
                            (get-text-property (point-at-bol) 'captured-data))
                           lines)
                          (forward-line 1))
                        (nreverse lines)))"##;
    let expect = expect![[
        r#"OK ((" M src/main.el" ahg-status-modified-face highlight #1=(keymap (mouse-2 . ahg-status-visit-file-other-window)) (nil "M" . "src/main.el")) ("*A new file.el" ahg-status-marked-face nil nil (t "A" . "new file.el")) (" ? notes λ.txt" ahg-status-unknown-face highlight #1# (nil "?" . "notes λ.txt")) (" X unknown" default highlight #1# (nil "X" . "unknown")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_ewoc_mark_toggle_filter_and_unmark_all_work_as_one_session() {
    let elisp_form = r##"(with-temp-buffer
                      (let ((ew (ewoc-create #'ahg-status-pp)))
                        (setq-local ewoc ew)
                        (dolist (data '((nil "M" . "modified.el")
                                        (nil "?" . "untracked.el")
                                        (t "A" . "added.el")))
                          (ewoc-enter-last ew data))
                        (goto-char (ewoc-location (ewoc-nth ew 0)))
                        (ahg-status-toggle-mark)
                        (ahg-status-do-mark nil)
                        (goto-char (ewoc-location (ewoc-nth ew 1)))
                        (ahg-status-mark)
                        (let ((marked (ahg-status-get-marked nil))
                              (unknown
                               (ahg-status-get-marked
                                'all
                                (lambda (data)
                                  (string= (cadr data) "?"))))
                              (at-point (ahg-status-get-marked 'cur)))
                          (ahg-status-unmark-all)
                          (list
                           marked
                           unknown
                           at-point
                           (mapcar #'identity
                                   (ewoc-collect ew #'identity))
                           (buffer-substring-no-properties
                            (point-min) (point-max))))))"##;
    let expect = expect![[
        r#"OK ((#2=(nil "M" . "modified.el") #1=(nil "?" . "untracked.el") #3=(nil "A" . "added.el")) (#1#) (#2# #1# #3#) (#2# #1# #3#) "\n M modified.el\n ? untracked.el\n A added.el\n\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_show_filters_dispatch_the_exact_mercurial_status_switches() {
    let elisp_form = r##"(let (calls)
                      (cl-letf (((symbol-function 'ahg-status)
                                 (lambda (&rest arguments)
                                   (push arguments calls))))
                        (ahg-status-show-default)
                        (ahg-status-show-all)
                        (ahg-status-show-tracked)
                        (ahg-status-show-modified)
                        (ahg-status-show-added)
                        (ahg-status-show-removed)
                        (ahg-status-show-deleted)
                        (ahg-status-show-clean)
                        (ahg-status-show-unknown)
                        (ahg-status-show-ignored)
                        (nreverse calls)))"##;
    let expect = expect![[
        r#"OK (("") ("-A") ("-mardc") ("-m") ("-a") ("-r") ("-d") ("-c") ("-u") ("-i"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_add_remove_and_addremove_select_eligible_files_and_dispatch_commands() {
    let elisp_form = r##"(let (commands prompts)
                      (cl-letf
                          (((symbol-function 'ahg-status-get-marked)
                            (lambda (_fallback filter)
                              (let ((entries
                                     '((t "?" . "new.el")
                                       (t "I" . "ignored.log")
                                       (t "!" . "gone.el")
                                       (t "C" . "clean.el")
                                       (t "M" . "changed.el"))))
                                (if filter
                                    (delq nil
                                          (mapcar
                                           (lambda (entry)
                                             (and (funcall filter entry)
                                                  entry))
                                           entries))
                                  entries))))
                           ((symbol-function 'ahg-y-or-n-p)
                            (lambda (prompt)
                              (push prompt prompts)
                              t))
                           ((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) "/repo/"))
                           ((symbol-function 'ahg-generic-command)
                            (lambda (command arguments &rest _rest)
                              (push (list command arguments) commands)
                              'fake-process)))
                        (ahg-status-add)
                        (ahg-status-remove)
                        (ahg-status-addremove)
                        (list (nreverse prompts)
                              (nreverse commands))))"##;
    let expect = expect![[
        r#"OK (("Add 2 files to hg? " "Remove 2 files from hg? " "Add/Remove 3 files to/from hg? ") (("add" ("new.el" "ignored.log")) ("remove" ("gone.el" "clean.el")) ("addremove" ("new.el" "ignored.log" "gone.el"))))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_delete_removes_selected_real_files_but_honors_abort() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (one (expand-file-name "repo/one.txt" sandbox))
                          (two (expand-file-name "repo/two.txt" sandbox))
                          (three (expand-file-name "repo/three.txt" sandbox))
                          refreshes
                          answers)
                      (make-directory (file-name-directory one) t)
                      (dolist (pair `((,one . "one") (,two . "two") (,three . "three")))
                        (with-temp-file (car pair) (insert (cdr pair))))
                      (setq answers '(t nil))
                      (cl-letf
                          (((symbol-function 'ahg-status-get-marked)
                            (lambda (_fallback)
                              (if (file-exists-p one)
                                  `((t "?" . ,one) (t "M" . ,two))
                                `((t "M" . ,three)))))
                           ((symbol-function 'ahg-y-or-n-p)
                            (lambda (_prompt) (pop answers)))
                           ((symbol-function 'ahg-status-refresh)
                            (lambda () (setq refreshes (1+ (or refreshes 0))))))
                        (ahg-status-delete)
                        (ahg-status-delete)
                        (list
                         (mapcar #'file-exists-p (list one two three))
                         refreshes
                         answers)))"##;
    let expect = expect!["OK ((nil nil t) 1 nil)"];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_add_to_hgignore_appends_deterministic_globs_and_refreshes_repo() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (ignore-file (expand-file-name ".hgignore" root))
                          refreshes)
                      (make-directory root t)
                      (with-temp-file ignore-file
                        (insert "syntax: regexp\n^build/\n"))
                      (cl-letf
                          (((symbol-function 'ahg-status-get-marked)
                            (lambda (_fallback)
                              '((t "?" . "dist/*.zip")
                                (t "I" . "coverage/")
                                (t "?" . "notes λ.txt"))))
                           ((symbol-function 'ahg-y-or-n-p)
                            (lambda (_prompt) t))
                           ((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) root))
                           ((symbol-function 'current-time-string)
                            (lambda () "Fri Jul 10 17:07:00 2026"))
                           ((symbol-function 'ahg-status-maybe-refresh)
                            (lambda (directory)
                              (push directory refreshes))))
                        (ahg-status-add-to-hgignore)
                        (list
                         (with-temp-buffer
                           (insert-file-contents ignore-file)
                           (buffer-string))
                         refreshes)))"##;
    let expect = expect![[
        r#"OK ("syntax: regexp\n^build/\n\n# added by aHg on Fri Jul 10 17:07:00 2026\nsyntax: glob\ndist/*.zip\ncoverage/\nnotes λ.txt\n" ("[ORACLE-SANDBOX]/repo/"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_diff_and_ediff_dispatch_selected_files_revisions_and_empty_selection() {
    let elisp_form = r##"(let (calls messages)
                      (cl-letf
                          (((symbol-function 'ahg-status-get-marked)
                            (lambda (_fallback)
                              '((t "M" . "src/a.el")
                                (t "A" . "src/b.el"))))
                           ((symbol-function 'ahg-rev-id)
                            (lambda (revision &optional _which)
                              (concat "resolved:" revision)))
                           ((symbol-function 'ahg-diff)
                            (lambda (&rest arguments)
                              (push (cons 'diff arguments) calls)))
                           ((symbol-function 'message)
                            (lambda (format-string &rest arguments)
                              (push (apply #'format format-string arguments)
                                    messages))))
                        (ahg-status-diff nil t)
                        (cl-letf (((symbol-function 'read-string)
                                   (lambda (&rest _arguments) "release")))
                          (ahg-status-diff t t))
                        (with-temp-buffer
                          (let ((empty-ewoc (ewoc-create #'ahg-status-pp)))
                            (setq ewoc empty-ewoc)
                            (ahg-status-diff nil nil)))
                        (list (nreverse calls)
                              (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (((diff "resolved:." nil ("src/a.el" "src/b.el")) (diff "resolved:release" nil ("src/a.el" "src/b.el"))) ("aHg diff: no file selected."))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_status_buffer_names_are_canonical_distinct_and_reused_per_repository() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (one (file-name-as-directory
                                (expand-file-name "repo one" sandbox)))
                          (two (file-name-as-directory
                                (expand-file-name "repo-two" sandbox)))
                          (one-buffer (ahg-get-status-buffer one t))
                          (two-buffer (ahg-get-status-buffer two t))
                          (buffers (list one-buffer two-buffer)))
                      (make-directory one t)
                      (make-directory two t)
                      (unwind-protect
                          (list
                           (mapcar #'buffer-name buffers)
                           (eq (ahg-get-status-buffer one) one-buffer)
                           (eq (ahg-get-status-buffer two) two-buffer)
                           (not (eq one-buffer two-buffer)))
                        (mapc
                         (lambda (buffer)
                           (when (buffer-live-p buffer)
                             (kill-buffer buffer)))
                         buffers)))"##;
    let expect = expect![[
        r#"OK (("*hg status: [ORACLE-SANDBOX]/repo one/*" "*hg status: [ORACLE-SANDBOX]/repo-two/*") t t t)"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
