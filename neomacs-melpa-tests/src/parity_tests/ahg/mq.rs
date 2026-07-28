use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn mq_queue_lists_guards_opens_a_real_patch_and_moves_to_the_selected_entry() {
    let elisp_form = r##"(let* ((fixture (neomacs-ahg-fixture))
       (root (nth 0 fixture))
       (fake-hg (nth 1 fixture))
       (patches (nth 5 fixture))
       (patch-file
        (expand-file-name
         "release-candidate" patches))
       (default-directory root)
       (ahg-hg-command fake-hg)
       (ahg-mq-list-patches-no-pop t)
       (ahg-auto-refresh-status-buffer nil)
       queue-buffer
       patch-buffer)
  (unwind-protect
      (progn
        (ahg-mq-list-patches root)
        (setq queue-buffer
              (ahg-mq-get-patches-buffer root t))
        (unless
            (neomacs-ahg-wait-until
             (lambda ()
               (and
                (buffer-live-p queue-buffer)
                (with-current-buffer queue-buffer
                  (and
                   (eq major-mode
                       'ahg-mq-patches-mode)
                   (= (length
                       (ewoc-collect
                        ewoc #'identity))
                      3))))))
          (error "aHg mq queue did not finish"))
        (let ((queue-state
               (with-current-buffer queue-buffer
                 (goto-char
                  (ewoc-location (ewoc-nth ewoc 1)))
                 (list
                  major-mode
                  (buffer-substring-no-properties
                   (point-min) (point-max))
                  (list
                   (lookup-key
                    ahg-mq-patches-mode-map "=")
                   (lookup-key
                    ahg-mq-patches-mode-map
                    (kbd "RET"))
                   (lookup-key
                    ahg-mq-patches-mode-map "m")
                   (lookup-key
                    ahg-mq-patches-mode-map "D"))))))
          (with-current-buffer queue-buffer
            (goto-char
             (ewoc-location (ewoc-nth ewoc 1)))
            (ahg-mq-patches-view-patch))
          (setq patch-buffer
                (get-file-buffer patch-file))
          (let ((patch-state
                 (with-current-buffer patch-buffer
                   (list
                    major-mode
                    (file-relative-name
                     buffer-file-name root)
                    (buffer-substring-no-properties
                     (point-min) (point-max))
                    (list
                     (lookup-key
                      ahg-diff-mode-map "q")
                     buffer-read-only)))))
            (with-current-buffer queue-buffer
              (goto-char
               (ewoc-location (ewoc-nth ewoc 1)))
              (cl-letf
                  (((symbol-function 'y-or-n-p)
                    (lambda (&rest _arguments) t))
                   ((symbol-function 'yes-or-no-p)
                    (lambda (&rest _arguments) t)))
                (ahg-mq-patches-goto-patch)))
            (unless
                (neomacs-ahg-wait-until
                 (lambda ()
                   (not
                    (cl-find-if
                     (lambda (process)
                       (string-prefix-p
                        "*ahg-command-qgoto*"
                        (process-name process)))
                     (process-list)))))
              (error "aHg qgoto did not finish"))
            (list
             queue-state
             patch-state
             (neomacs-ahg-file-string
              (expand-file-name
               ".hg/commands.log" root))))))
    (dolist (buffer (list patch-buffer queue-buffer))
      (when (buffer-live-p buffer)
        (set-buffer-modified-p nil)
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r##"OK ((ahg-mq-patches-mode "mq patch queue for [ORACLE-SANDBOX]/release-repo/\n\n--------------------------------------------------------------------------------\n Index | App | Patch (Guards)\n--------------------------------------------------------------------------------\n     0 |  *  | base                                                             \n     1 |  *  | release-candidate (+linux -windows)                              \n     2 |     | cleanup (+cleanup)                                               \n--------------------------------------------------------------------------------\n" (ahg-mq-patches-view-patch ahg-mq-patches-goto-patch ahg-mq-patches-moveto-patch ahg-mq-patches-delete-patch)) (ahg-diff-mode ".hg/patches/release-candidate" "# HG changeset patch\n# User Ada Lovelace <ada@example.test>\n# Date 1700000000 0\n#      Wed Nov 13 08:48:06 2024 +0000\n# Node ID c0ffee\n# Parent  bead123\nPrepare the release candidate\n\ndiff --git a/src/main.el b/src/main.el\n--- a/src/main.el\n+++ b/src/main.el\n@@ -1,1 +1,1 @@\n-(defun deploy-candidate ()\n+(defun deploy-release ()\n" (ahg-buffer-quit t)) "qseries|\nqapplied|\nqguard|-l\nid|-n\nqgoto|-f release-candidate\n")"##
    ]];
    assert_ahg_parity(elisp_form, expect);
}
