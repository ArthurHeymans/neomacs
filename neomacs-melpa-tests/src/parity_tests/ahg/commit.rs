use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn current_file_commit_saves_edits_collects_metadata_and_submits_the_authored_message() {
    let elisp_form = r##"(let* ((fixture (neomacs-ahg-fixture))
       (root (nth 0 fixture))
       (fake-hg (nth 1 fixture))
       (source (nth 2 fixture))
       (default-directory root)
       (ahg-hg-command fake-hg)
       (ahg-auto-refresh-status-buffer nil)
       (log-edit-confirm nil)
       (log-edit-require-final-newline t)
       source-buffer
       log-buffer
       editor-state)
  (unwind-protect
      (progn
        (setq source-buffer
              (find-file-noselect source))
        (with-current-buffer source-buffer
          (goto-char (point-max))
          (insert
           "\n;; Reviewed by the release team.\n")
          (cl-letf
              (((symbol-function 'y-or-n-p)
                (lambda (&rest _arguments) t))
               ((symbol-function 'yes-or-no-p)
                (lambda (&rest _arguments) t)))
            (ahg-commit-cur-file)))
        (setq log-buffer (get-buffer "*aHg-log*"))
        (unless (buffer-live-p log-buffer)
          (error "aHg did not open a commit editor"))
        (setq editor-state
              (with-current-buffer log-buffer
                (list
                 major-mode
                 (mapcar
                  (lambda (file)
                    (file-relative-name file root))
                  (log-edit-files))
                 (buffer-substring-no-properties
                  (point-min) (point-max))
                 (lookup-key
                  log-edit-mode-map
                  (kbd "C-c C-c")))))
        (with-current-buffer log-buffer
          (goto-char (point-min))
          (insert
           "Ship release safely\n"
           "\n"
           "The release workflow now records review ownership.\n")
          (log-edit-done))
        (unless
            (neomacs-ahg-wait-until
             (lambda ()
               (and
                (file-exists-p
                 (expand-file-name
                  ".hg/last-commit-message" root))
                (not
                 (cl-find-if
                  (lambda (process)
                    (string-prefix-p
                     "*ahg-command-commit*"
                     (process-name process)))
                  (process-list))))))
          (error "aHg commit did not finish"))
        (list
         editor-state
         (buffer-live-p log-buffer)
         (with-current-buffer source-buffer
           (list
            (buffer-modified-p)
            (buffer-substring-no-properties
             (point-min) (point-max))))
         (neomacs-ahg-file-string
          (expand-file-name
           ".hg/last-commit-message" root))
         (neomacs-ahg-file-string
          (expand-file-name
           ".hg/last-commit-files" root))
         (neomacs-ahg-file-string
          (expand-file-name
           ".hg/commands.log" root))))
    (dolist (buffer (list log-buffer source-buffer))
      (when (buffer-live-p buffer)
        (set-buffer-modified-p nil)
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((log-edit-mode ("src/main.el") "\n\nHG: Enter commit message.  Lines beginning with 'HG:' are removed.\nHG: --\nHG: user: Ada Lovelace <ada@example.test>\nHG: root: [ORACLE-SANDBOX]/release-repo/src/\nHG: branch: feature\nHG: committing main.el\nHG: Press C-c C-c when you are done editing." log-edit-done) nil (nil "(defun deploy-release ()\n  (message \"release ready\"))\n\n(defun rollback-release ()\n  (message \"rollback ready\"))\n\n;; Reviewed by the release team.\n") "Ship release safely\n\nThe release workflow now records review ownership.\n\n" "[ORACLE-SANDBOX]/release-repo/src/main.el\n" "showconfig|ui.username\nbranch|\ncommit|-m Ship release safely\n\nThe release workflow now records review ownership.\n\n [ORACLE-SANDBOX]/release-repo/src/main.el\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
