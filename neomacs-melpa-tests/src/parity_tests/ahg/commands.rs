use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn command_help_and_manifest_grep_form_a_practical_repository_inspection_session() {
    let elisp_form = r##"(let* ((fixture (neomacs-ahg-fixture))
       (root (nth 0 fixture))
       (fake-hg (nth 1 fixture))
       (default-directory root)
       (ahg-hg-command fake-hg)
       (ahg-do-command-show-buffer-immediately nil)
       (ahg-manifest-grep-use-xargs-grep nil)
       command-buffer
       help-buffer
       grep-buffer)
  (unwind-protect
      (progn
        (ahg-do-command "status --all")
        (setq command-buffer
              (get-buffer
               (concat "*hg command: " root "*")))
        (unless
            (neomacs-ahg-wait-until
             (lambda ()
               (and
                (buffer-live-p command-buffer)
                (not
                 (get-buffer-process command-buffer)))))
          (error "aHg arbitrary command did not finish"))
        (let ((command-state
               (with-current-buffer command-buffer
                 (list
                  major-mode
                  (buffer-substring-no-properties
                   (point-min) (point-max))
                  (list
                   (lookup-key
                    ahg-command-mode-map "!")
                   (lookup-key
                    ahg-command-mode-map "h")
                   (lookup-key
                    ahg-command-mode-map "q"))))))
          (ahg-command-help "status")
          (setq help-buffer (get-buffer "*hg help*"))
          (unless
              (neomacs-ahg-wait-until
               (lambda ()
                 (and
                  (buffer-live-p help-buffer)
                  (not
                   (get-buffer-process help-buffer))
                  (with-current-buffer help-buffer
                    (eq major-mode 'help-mode)))))
            (error "aHg command help did not finish"))
          (let ((help-state
                 (with-current-buffer help-buffer
                   (list
                    major-mode
                    view-mode
                    (buffer-substring-no-properties
                     (point-min) (point-max))
                    (key-binding "!")))))
            (ahg-manifest-grep "release" nil)
            (setq grep-buffer
                  (get-buffer "*ahg-grep*"))
            (unless
                (neomacs-ahg-wait-until
                 (lambda ()
                   (and
                    (buffer-live-p grep-buffer)
                    (with-current-buffer grep-buffer
                      (and
                       (null mode-line-process)
                       (save-excursion
                         (goto-char (point-min))
                         (search-forward
                          "aHg grep finished at"
                          nil t)))))))
              (error "aHg manifest grep did not finish"))
            (let ((grep-state
                   (with-current-buffer grep-buffer
                     (goto-char (point-min))
                     (let (matches)
                       (while
                           (re-search-forward
                            "release" nil t)
                         (push
                          (list
                           (line-number-at-pos)
                           (get-text-property
                            (match-beginning 0)
                            'font-lock-face))
                          matches))
                       (list
                        major-mode
                        (nreverse matches)
                        (replace-regexp-in-string
                         "aHg grep finished at [^\n]+"
                         "aHg grep finished at [TIME]"
                         (buffer-substring-no-properties
                          (point-min)
                          (point-max)))
                        (commandp
                         (key-binding "g")))))))
              (list
               command-state
               help-state
               grep-state
               (neomacs-ahg-file-string
                (expand-file-name
                 ".hg/commands.log"
                 root)))))))
    (dolist
        (buffer
         (list command-buffer help-buffer grep-buffer))
      (when (buffer-live-p buffer)
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((ahg-command-mode "output of 'hg status --all' on [ORACLE-SANDBOX]/release-repo/\n-------------------------------------------------------------------------------\n\nM src/main.el\n? notes.todo\n! removed.txt\n" (ahg-do-command ahg-command-help ahg-buffer-quit)) (help-mode t "hg status [OPTION]... [FILE]...\n\nshow changed files in the working directory\n\noptions:\n -A --all  show all files\n" ahg-do-command) (grep-mode ((1 font-lock-string-face) (1 ahg-header-line-root-face) (3 match) (4 match) (5 match) (6 match) (7 match) (8 match)) "searching for pattern release in [ORACLE-SANDBOX]/release-repo/\n\nsrc/main.el:1:(defun deploy-release ()\nsrc/main.el:2:  (message \"release ready\"))\nsrc/main.el:4:(defun rollback-release ()\ndocs/guide.md:1:# Release guide\ndocs/guide.md:3:Deploy the release after review.\ndocs/guide.md:4:Rollback the release if monitoring fails.\n\n-------------------------------------------------------------------------------\naHg grep finished at [TIME]\n" t) "status|--all\nhelp|status\nfiles|set:grep(release) & ! binary()\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
