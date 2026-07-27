use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn ahg_record_setup_creates_auditable_backup_and_editable_selected_patch() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          calls
                          patch-buffer)
                      (make-directory (expand-file-name ".hg" root) t)
                      (cl-letf
                          (((symbol-function 'ahg-cd)
                            (lambda (directory)
                              (setq default-directory directory)
                              t))
                           ((symbol-function 'ahg-push-window-configuration)
                            (lambda () nil))
                           ((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              (cond
                               ((equal command "parents")
                                (insert
                                 "0123456789abcdef0123456789abcdef01234567 ")
                                0)
                               ((equal command "status") 0)
                               ((equal command "diff")
                                (insert
                                 "diff --git a/src/main.el b/src/main.el\n"
                                 "--- a/src/main.el\n"
                                 "+++ b/src/main.el\n"
                                 "@@ -1 +1 @@\n-old\n+new\n")
                                0)
                               (t 1)))))
                        (let* ((result
                                (ahg-record-setup
                                 root '("src/main.el")))
                               (backup (nth 0 result))
                               (current-patch (nth 1 result))
                               (parent (nth 2 result)))
                          (setq patch-buffer (nth 3 result))
                          (unwind-protect
                              (list
                               (file-relative-name backup root)
                               (file-relative-name current-patch root)
                               parent
                               (with-temp-buffer
                                 (insert-file-contents backup)
                                 (buffer-string))
                               (with-current-buffer patch-buffer
                                 (buffer-substring-no-properties
                                  (point-min) (point-max)))
                               (nreverse calls))
                            (when (buffer-live-p patch-buffer)
                              (kill-buffer patch-buffer))))))"##;
    let expect = expect![[
        r##"OK (".hg/ahg-record-backup" ".hg/ahg-record-patch" "0123456789abcdef0123456789abcdef01234567" "# aHg record parent: 0123456789abcdef0123456789abcdef01234567\ndiff --git a/src/main.el b/src/main.el\n--- a/src/main.el\n+++ b/src/main.el\n@@ -1 +1 @@\n-old\n+new\n" "# aHg record interactive buffer\n# edit this patch file, and press C-c C-c when done to commit the changes\n# if something goes wrong, there's a backup file in\n# [ORACLE-SANDBOX]/repo/.hg/ahg-record-backup\n\ndiff --git a/src/main.el b/src/main.el\n--- a/src/main.el\n+++ b/src/main.el\n@@ -1 +1 @@\n-old\n+new\n" (("parents" ("--template" "{node} ")) ("status" ("-a" "-d" "-r")) ("diff" ("--git")) ("diff" ("--git" "src/main.el"))))"##
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_record_setup_rejects_stale_backup_merge_addremove_binary_and_diff_failure() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (backup (expand-file-name
                                   ".hg/ahg-record-backup" root))
                          scenario)
                      (make-directory (expand-file-name ".hg" root) t)
                      (cl-letf
                          (((symbol-function 'ahg-cd)
                            (lambda (directory)
                              (setq default-directory directory)
                              t))
                           ((symbol-function 'ahg-call-process)
                            (lambda (command &optional _arguments _global)
                              (pcase (list scenario command)
                                (`(merge "parents")
                                 (insert "parent-one parent-two ")
                                 0)
                                (`(no-parent "parents") 1)
                                (`(addremove "parents")
                                 (insert "parent ")
                                 0)
                                (`(addremove "status")
                                 (insert "A new.el\n")
                                 0)
                                (`(binary "parents")
                                 (insert "parent ")
                                 0)
                                (`(binary "status") 0)
                                (`(binary "diff")
                                 (insert
                                  "diff --git a/image.png b/image.png\n"
                                  "new file mode 100644\n"
                                  "GIT binary patch\nliteral 1\nA\n")
                                 0)
                                (`(diff-failure "parents")
                                 (insert "parent ")
                                 0)
                                (`(diff-failure "status") 0)
                                (`(diff-failure "diff") 1)
                                (`(_ "parents")
                                 (insert "parent ")
                                 0)
                                (`(_ "status") 0)
                                (`(_ "diff")
                                 (insert "diff --git a/a b/a\n")
                                 0)
                                (_ 1)))))
                        (mapcar
                         (lambda (case)
                           (setq scenario case)
                           (when (file-exists-p backup)
                             (delete-file backup))
                           (when (eq case 'stale)
                             (with-temp-file backup
                               (insert "existing")))
                           (list
                            case
                            (condition-case error-data
                                (let ((result
                                       (ahg-record-setup root nil)))
                                  (when (buffer-live-p (nth 3 result))
                                    (kill-buffer (nth 3 result)))
                                  'unexpected-success)
                              (error
                               (list (car error-data)
                                     (cadr error-data))))))
                         '(stale merge no-parent addremove binary
                                 diff-failure))))"##;
    let expect = expect![[
        r#"OK ((stale (error "stale ahg-record backup file detected in ‘[ORACLE-SANDBOX]/repo/.hg/ahg-record-backup’, aborting")) (merge (error "uncommitted merge detected, aborting")) (no-parent (error "could not in determine parents of working dir, aborting")) (addremove (error "pending additions and/or deletions detected, aborting")) (binary (error "binary files in patch, aborting")) (diff-failure (error "impossible to generate diff")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_commit_message_parser_keeps_multiline_content_and_discards_only_hg_metadata_lines() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "Subject line\n"
                       "\n"
                       "Detailed body with λ.\n"
                       "HG: Enter commit message.\n"
                       "Indented HG: is user content\n"
                       "HG: user: Test User\n"
                       "Trailer: value\n"
                       "\n")
                      (list
                       (ahg-parse-commit-message)
                       (progn
                         (erase-buffer)
                         (insert "HG: metadata only\nHG: --\n")
                         (ahg-parse-commit-message))))"##;
    let expect = expect![[
        r#"OK ("Subject line\n\nDetailed body with λ.\nIndented HG: is user content\nTrailer: value\n" "")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_log_edit_hook_renders_user_branch_root_selected_files_and_existing_message() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (default-directory root)
                          calls)
                      (make-directory root t)
                      (cl-letf
                          (((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              (pcase command
                                ("showconfig"
                                 (insert "Alice Example <alice@example.test>\n")
                                 0)
                                ("branch"
                                 (insert "feature/λ\n")
                                 0)
                                (_ 1))))
                           ((symbol-function 'log-edit-files)
                            (lambda ()
                              (list
                               (expand-file-name "src/main.el" root)
                               (expand-file-name "docs/guide.md" root)))))
                        (with-temp-buffer
                          (ahg-log-edit-hook
                           "amending changeset abc123\nreview carefully"
                           "Existing subject\n\nExisting body")
                          (list
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           (point)
                           (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("Existing subject\n\nExisting body\n\nHG: Enter commit message.  Lines beginning with 'HG:' are removed.\nHG: amending changeset abc123\nHG: review carefully\nHG: --\nHG: user: Alice Example <alice@example.test>\nHG: root: [ORACLE-SANDBOX]/repo/\nHG: branch: feature/λ\nHG: committing src/main.el docs/guide.md\nHG: Press C-c C-c when you are done editing." 1 (("showconfig" ("ui.username")) ("branch" nil)))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_histedit_revision_validation_accepts_single_ids_and_rejects_empty_or_ambiguous_results() {
    let elisp_form = r##"(let (value)
                      (cl-letf
                          (((symbol-function 'ahg-rev-id)
                            (lambda (_revision &optional _which) value)))
                        (mapcar
                         (lambda (case)
                           (setq value (cdr case))
                           (list
                            (car case)
                            (condition-case error-data
                                (ahg-histedit-rev-id (car case))
                              (error
                               (list (car error-data)
                                     (cadr error-data))))))
                         '(("17" . "abc123")
                           ("missing" . nil)
                           ("empty" . "")
                           ("many" . "abc123 def456")))))"##;
    let expect = expect![[
        r#"OK (("17" "abc123") ("missing" (error "bad revision number missing")) ("empty" (error "bad revision number empty")) ("many" (error "bad revision number many")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_histedit_preflight_distinguishes_dirty_unsupported_backup_failure_and_success() {
    let elisp_form = r##"(let (dirty supported backup)
                      (cl-letf
                          (((symbol-function 'ahg-uncommitted-changes-p)
                            (lambda (&optional _root) dirty))
                           ((symbol-function 'ahg-histedit-check-ok)
                            (lambda (_root _revision) supported))
                           ((symbol-function 'ahg-histedit-backup)
                            (lambda (_root _operation _revision) backup)))
                        (mapcar
                         (lambda (case)
                           (setq dirty (nth 1 case)
                                 supported (nth 2 case)
                                 backup (nth 3 case))
                           (list
                            (car case)
                            (condition-case error-data
                                (ahg-histedit-setup
                                 "/repo/" "drop" "17")
                              (error
                               (list (car error-data)
                                     (cadr error-data))))))
                         '((success nil t "/repo/.hg/backup.hg")
                           (dirty t t "/repo/.hg/backup.hg")
                           (unsupported nil nil "/repo/.hg/backup.hg")
                           (backup-failed nil t nil)))))"##;
    let expect = expect![[
        r#"OK ((success "/repo/.hg/backup.hg") (dirty (error "the working directory contains uncommited changes, aborting")) (unsupported (error "unsupported history layout (non-linear and/or public changesets detected), aborting")) (backup-failed (error "history backup failed, aborting")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_histedit_backup_creates_repo_metadata_and_builds_precise_bundle_revset() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          calls)
                      (make-directory (expand-file-name ".hg" root) t)
                      (cl-letf
                          (((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              0)))
                        (let ((result
                               (ahg-histedit-backup
                                root "xtract" "abc123")))
                          (list
                           (file-relative-name result root)
                           (file-directory-p
                            (expand-file-name
                             ".hg/ahg-histedit-backup" root))
                           (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (".hg/ahg-histedit-backup/xtract-abc123-bundle.hg" t (("bundle" ("--base" "not descendants(parents(abc123))" "[ORACLE-SANDBOX]/repo/.hg/ahg-histedit-backup/xtract-abc123-bundle.hg"))))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_histedit_query_helpers_issue_real_revsets_and_parse_messages_heads_and_layout() {
    let elisp_form = r##"(let (calls response)
                      (cl-letf
                          (((symbol-function 'ahg-cd)
                            (lambda (_directory) t))
                           ((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              (pcase response
                                ('empty 0)
                                ('message
                                 (insert "Subject\n\nBody λ")
                                 0)
                                ('head
                                 (insert "abc123")
                                 0)
                                ('layout
                                 (insert "badnode\n")
                                 0)
                                (_ 1)))))
                        (list
                         (progn
                           (setq response 'empty)
                           (ahg-histedit-check-ok "/repo/" "17"))
                         (progn
                           (setq response 'layout)
                           (ahg-histedit-check-ok "/repo/" "17"))
                         (progn
                           (setq response 'message)
                           (ahg-histedit-get-message "17"))
                         (progn
                           (setq response 'head)
                           (ahg-histedit-is-head "17"))
                         (progn
                           (setq response 'empty)
                           (ahg-histedit-is-head "18"))
                         (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (t nil "Subject\n\nBody λ" t nil (("log" ("--template" "{node|short}\\n" "-r" "descendants(17) & (branchpoint() + merge() + public())")) ("log" ("--template" "{node|short}\\n" "-r" "descendants(17) & (branchpoint() + merge() + public())")) ("log" ("-r" "17" "--template" "{desc}")) ("log" ("-r" "17 & head()" "--template" "{node|short}")) ("log" ("-r" "18 & head()" "--template" "{node|short}"))))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
