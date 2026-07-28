use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn log_views_render_real_release_history_and_support_revision_navigation() {
    let elisp_form = r##"(let* ((fixture (neomacs-ahg-fixture))
       (root (nth 0 fixture))
       (fake-hg (nth 1 fixture))
       (default-directory root)
       (ahg-hg-command fake-hg)
       short-buffer
       detail-buffer)
  (unwind-protect
      (progn
        (ahg-short-log "2" "0")
        (setq short-buffer
              (get-buffer
               (concat "*hg log (summary): "
                       root "*")))
        (unless
            (neomacs-ahg-wait-until
             (lambda ()
               (and
                (buffer-live-p short-buffer)
                (with-current-buffer short-buffer
                  (eq major-mode
                      'ahg-short-log-mode)))))
          (error "aHg short log did not finish"))
        (let ((short-state
               (with-current-buffer short-buffer
                 (goto-char
                  (ewoc-location (ewoc-nth ewoc 0)))
                 (let ((first
                        (ahg-short-log-revision-at-point)))
                   (ahg-short-log-next 1)
                   (let ((second
                          (ahg-short-log-revision-at-point)))
                     (ahg-short-log-goto-revision "0")
                     (list
                      major-mode
                      (mapcar
                       #'copy-tree
                       (ewoc-collect ewoc #'identity))
                      first
                      second
                      (ahg-short-log-revision-at-point)
                      (buffer-substring-no-properties
                       (point-min) (point-max))
                      (list
                       (lookup-key
                        ahg-short-log-mode-map "n")
                       (lookup-key
                        ahg-short-log-mode-map "p")
                       (lookup-key
                        ahg-short-log-mode-map "=")
                       (lookup-key
                        ahg-short-log-mode-map
                        (kbd "RET")))))))))
          (ahg-log "2" "1")
          (setq detail-buffer
                (get-buffer
                 (concat "*hg log (details): "
                         root "*")))
          (unless
              (neomacs-ahg-wait-until
               (lambda ()
                 (and
                  (buffer-live-p detail-buffer)
                  (with-current-buffer detail-buffer
                    (eq major-mode
                        'ahg-log-mode)))))
            (error "aHg detailed log did not finish"))
          (let ((detail-state
                 (with-current-buffer detail-buffer
                   (goto-char (point-min))
                   (re-search-forward
                    "^changeset: +2:c0ffee")
                   (let ((first
                          (list
                           (ahg-log-revision-at-point)
                           (ahg-log-revision-at-point t))))
                     (ahg-log-next 1)
                     (let ((second
                            (list
                             (ahg-log-revision-at-point)
                             (ahg-log-revision-at-point t))))
                       (goto-char (point-min))
                       (re-search-forward
                        "^files: +src/main.el$")
                       (list
                        major-mode
                        first
                        second
                        (ahg-log-filename-at-point
                         (point) t)
                        (buffer-substring-no-properties
                         (point-min) (point-max))
                        (list
                         (lookup-key
                          ahg-log-mode-map "n")
                         (lookup-key
                          ahg-log-mode-map "p")
                         (lookup-key
                          ahg-log-mode-map "=")
                         (lookup-key
                          ahg-log-mode-map
                          (kbd "RET")))))))))
            (list
             short-state
             detail-state
             (neomacs-ahg-file-string
              (expand-file-name
               ".hg/ahg-log-style-map"
               root))
             (neomacs-ahg-file-string
              (expand-file-name
               ".hg/commands.log"
               root))))))
    (dolist (buffer (list short-buffer detail-buffer))
      (when (buffer-live-p buffer)
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((ahg-short-log-mode (("2" "2024-11-13" "ada" "Ship release safely") ("1" "2024-11-12" "grace" "Add rollback procedure") ("0" "2024-11-11" "ada" "Bootstrap repository")) "2" "1" "0" "hg log (summary) for [ORACLE-SANDBOX]/release-repo/\n\n--------------------------------------------------------------------------------\n    Rev |    Date    |  Author  | Summary\n--------------------------------------------------------------------------------\n      2*| 2024-11-13 |      ada | Ship release safely                           \n      1 | 2024-11-12 |    grace | Add rollback procedure                        \n      0 | 2024-11-11 |      ada | Bootstrap repository                          \n--------------------------------------------------------------------------------\n" (ahg-short-log-next ahg-short-log-previous ahg-short-log-view-diff ahg-short-log-update-to-rev)) (ahg-log-mode ("c0ffee" "2") ("bead123" "1") "src/main.el" "hg log for [ORACLE-SANDBOX]/release-repo/\n\nchangeset:   2:c0ffee\nphase:       draft\nbranch:      feature\ntag:         v2.0\nbookmark:    release\nparent:      1:bead123\nuser:        Ada Lovelace <ada@example.test>\ndate:        1700000000 0\nfiles:       src/main.el\n             docs/guide.md\ndescription:\nShip release safely\nDocument deployment\n\n\n\nchangeset:   1:bead123\nparent:      0:0000000\nuser:        Grace Hopper <grace@example.test>\ndate:        1699900000 0\nfiles:       src/main.el\ndescription:\nAdd rollback procedure\n\n\n\n" (ahg-log-next ahg-log-previous ahg-log-view-diff ahg-log-update-to-rev)) "changeset = \"{rev}:{node|short}\\n{svnrev}\\n{gitnode}\\n{phase}\\n{branches}\\n{tags}\\n{bookmarks}\\n{parents}\\n{author}\\n{date|date}\\n{files}\\n\\t{desc|tabindent}\\n\"\nfile = \"{file}\\n\"\n" "log|-r 2:0 --template {rev} {date|shortdate} {author|user} {desc|firstline}\\n\nlog|-r . --template {rev} \nlog|-r 2:1 --style .hg/ahg-log-style-map\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn diff_and_annotate_render_real_file_history_and_jump_back_to_source() {
    let elisp_form = r##"(let* ((fixture (neomacs-ahg-fixture))
       (root (nth 0 fixture))
       (fake-hg (nth 1 fixture))
       (source (nth 2 fixture))
       (default-directory root)
       (ahg-hg-command fake-hg)
       diff-buffer
       annotate-buffer
       source-buffer)
  (unwind-protect
      (progn
        (ahg-diff "1" "2" '("src/main.el"))
        (setq diff-buffer (get-buffer "*aHg-diff*"))
        (unless
            (neomacs-ahg-wait-until
             (lambda ()
               (and
                (buffer-live-p diff-buffer)
                (with-current-buffer diff-buffer
                  (eq major-mode 'ahg-diff-mode)))))
          (error "aHg diff did not finish"))
        (let ((diff-state
               (with-current-buffer diff-buffer
                 (goto-char (point-min))
                 (list
                  major-mode
                  (copy-tree ahg-diff-revs)
                  (buffer-substring-no-properties
                   (point-min) (point-max))
                  (diff-hunk-file-names)
                  (list
                   (lookup-key ahg-diff-mode-map "e")
                   (lookup-key ahg-diff-mode-map "q"))))))
          (with-current-buffer
              (find-file-noselect source)
            (goto-char (point-min))
            (forward-line 1)
            (ahg-annotate-cur-file))
          (setq annotate-buffer
                (get-buffer
                 (concat "*hg annotate: "
                         source "*")))
          (unless
              (neomacs-ahg-wait-until
               (lambda ()
                 (and
                  (buffer-live-p annotate-buffer)
                  (with-current-buffer annotate-buffer
                    (eq major-mode
                        'ahg-annotate-mode)))))
            (error "aHg annotate did not finish"))
          (let ((annotate-state
                 (with-current-buffer annotate-buffer
                   (font-lock-ensure)
                   (goto-char (point-min))
                   (let ((annotate-mode major-mode)
                         (annotate-keys
                          (list
                           (lookup-key
                            ahg-annotate-mode-map "=")
                           (lookup-key
                            ahg-annotate-mode-map "l")
                           (lookup-key
                            ahg-annotate-mode-map "u")
                           (lookup-key
                            ahg-annotate-mode-map
                            (kbd "RET"))))
                         lines)
                     (while (not (eobp))
                       (push
                        (list
                         (buffer-substring-no-properties
                          (point-at-bol) (point-at-eol))
                         (get-text-property
                          (point-at-bol)
                          'ahg-line-number)
                         (get-text-property
                          (point-at-bol) 'face)
                         (get-text-property
                          (point-at-bol) 'help-echo))
                        lines)
                       (forward-line 1))
                     (goto-char (point-min))
                     (ahg-annotate-goto-line)
                     (setq source-buffer
                           (get-file-buffer source))
                     (list
                      annotate-mode
                      (nreverse lines)
                      annotate-keys
                      (with-current-buffer source-buffer
                        (list
                         (line-number-at-pos)
                         (buffer-substring-no-properties
                          (point-at-bol)
                          (point-at-eol)))))))))
            (list
             diff-state
             annotate-state
             (neomacs-ahg-file-string
              (expand-file-name
               ".hg/commands.log"
               root))))))
    (dolist
        (buffer
         (list diff-buffer annotate-buffer source-buffer))
      (when (buffer-live-p buffer)
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((ahg-diff-mode ("c0ffee" . "bead123") "diff --git a/src/main.el b/src/main.el\n--- a/src/main.el\n+++ b/src/main.el\n@@ -1,2 +1,2 @@\n-(defun deploy-candidate ()\n+(defun deploy-release ()\n   (message \"release ready\"))\n" ("b/src/main.el" "a/src/main.el") (ahg-diff-ediff ahg-buffer-quit)) (ahg-annotate-mode (("Ada 2 2024-11-13: 1: (defun deploy-release ()" "1" ahg-annotate-face-E665170A170A " Ship release safely") ("Ada 2 2024-11-13: 2:   (message \"release ready\"))" "2" ahg-annotate-face-E665170A170A " Ship release safely") ("Grace 1 2024-11-12: 4: (defun rollback-release ()" "4" ahg-annotate-face-4082170AE665 " Add rollback procedure") ("Grace 1 2024-11-12: 5:   (message \"rollback ready\"))" "5" ahg-annotate-face-4082170AE665 " Add rollback procedure")) (ahg-annotate-diff ahg-annotate-log ahg-annotate-uncover ahg-annotate-goto-line) (1 "(defun deploy-release ()")) "diff|--git -r 1 -r 2 src/main.el\nlog|-r 2 --template {node|short} \nlog|-r 1 --template {node|short} \nlog|--template {rev} {desc|firstline}\\n src/main.el\nannotate|-undql src/main.el\nstatus|-A main.el\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
