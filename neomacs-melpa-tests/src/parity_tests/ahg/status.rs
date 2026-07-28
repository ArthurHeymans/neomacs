use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn repository_status_session_discovers_changes_marks_entries_and_visits_real_files() {
    let elisp_form = r##"(let* ((fixture (neomacs-ahg-fixture))
       (root (nth 0 fixture))
       (fake-hg (nth 1 fixture))
       (source (nth 2 fixture))
       (default-directory
        (file-name-as-directory
         (expand-file-name "src" root)))
       (ahg-hg-command fake-hg)
       (ahg-summary-remote nil)
       (ahg-summary-git-svn-info nil)
       (ahg-status-no-pop t)
       status-buffer
       visited-buffer)
  (unwind-protect
      (progn
        (ahg-status)
        (unless
            (neomacs-ahg-wait-until
             (lambda ()
               (setq status-buffer
                     (ahg-get-status-buffer root))
               (and
                status-buffer
                (not (get-buffer-process "*aHg-status*")))))
          (error "aHg status did not finish"))
        (with-current-buffer status-buffer
          (goto-char
           (ewoc-location (ewoc-nth ewoc 0)))
          (ahg-status-visit-file)
          (setq visited-buffer
                (get-file-buffer source))
          (let ((visited
                 (with-current-buffer visited-buffer
                   (list
                    major-mode
                    (file-relative-name
                     buffer-file-name root)
                    (buffer-substring-no-properties
                     (point-min) (point-max))))))
            (set-buffer status-buffer)
            (goto-char
             (ewoc-location (ewoc-nth ewoc 0)))
            (ahg-status-toggle-mark)
            (ahg-status-mark)
            (ahg-status-prev-file)
            (list
             major-mode
             (file-relative-name
              default-directory root)
             (buffer-name)
             (mapcar
              #'copy-tree
              (ewoc-collect ewoc #'identity))
             (mapcar
              #'copy-tree
              (ahg-status-get-marked nil))
             (buffer-substring-no-properties
              (point-min) (point-max))
             (list
              (lookup-key ahg-status-mode-map " ")
              (lookup-key ahg-status-mode-map "m")
              (lookup-key ahg-status-mode-map "=")
              (lookup-key ahg-status-mode-map "f"))
             visited
             (neomacs-ahg-file-string
              (expand-file-name ".hg/commands.log"
                                root))))))
    (dolist
        (buffer
         (list visited-buffer status-buffer
               (get-buffer "*aHg-status*")))
      (when (buffer-live-p buffer)
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (ahg-status-mode "./" "*hg status: [ORACLE-SANDBOX]/release-repo/*" ((t "M" . "src/main.el") (t "?" . "notes.todo") (nil "!" . "removed.txt")) ((t "M" . "src/main.el") (t "?" . "notes.todo")) "hg status for [ORACLE-SANDBOX]/release-repo/\n\n*M src/main.el\n*? notes.todo\n ! removed.txt\n\n-------------------------------------------------------------------------------\nbranch: feature\nparent: 2:c0ffee\ncommit: 2 modified, 1 unknown\n" (ahg-status-toggle-mark ahg-status-mark ahg-status-diff ahg-status-visit-file) (emacs-lisp-mode "src/main.el" "(defun deploy-release ()\n  (message \"release ready\"))\n\n(defun rollback-release ()\n  (message \"rollback ready\"))\n") "status|\nsummary|\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
