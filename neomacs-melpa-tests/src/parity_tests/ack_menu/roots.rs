use super::assert_ack_menu_parity;
use expect_test::expect;

#[test]
fn ack_menu_guess_project_root_finds_nearest_marker_and_handles_no_match() {
    let elisp_form = r##"(let ((root
                (make-temp-file
                 "ack-menu-root-"
                 t)))
         (unwind-protect
             (let* ((outer
                     (expand-file-name
                      "outer"
                      root))
                    (inner
                     (expand-file-name
                      "inner/deep"
                      outer))
                    (nearest
                     (expand-file-name
                      "inner"
                      outer)))
               (make-directory
                inner t)
               (make-directory
                (expand-file-name
                 ".fixture-root"
                 outer))
               (write-region
                ""
                nil
                (expand-file-name
                 ".fixture-root"
                 nearest)
                nil
                'silent)
               (let ((ack-project-root-file-patterns
                      '("\\`.fixture-root\\'")))
                 (list
                  (let ((buffer-file-name
                         (expand-file-name
                          "source.el"
                          inner)))
                    (file-relative-name
                     (ack-guess-project-root)
                     root))
                  (let ((buffer-file-name
                         nil)
                        (default-directory
                         (file-name-as-directory
                          inner)))
                    (file-relative-name
                     (ack-guess-project-root)
                     root))
                  (let ((ack-project-root-file-patterns
                         '("\\`fixture-never-present\\'"))
                        (buffer-file-name
                         (expand-file-name
                          "source.el"
                          inner)))
                    (ack-guess-project-root)))))
           (delete-directory
            root t)))"##;
    let expect = expect![[r#"OK ("outer/inner/" "outer/inner/" nil)"#]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_read_dir_covers_guess_confirmation_file_and_default_fallbacks() {
    let elisp_form = r##"(let (calls)
         (cl-labels
             ((scenario
               (guess prompt file)
               (let ((ack-prompt-for-directory
                      prompt)
                     (buffer-file-name
                      file)
                     (default-directory
                      "/fixture/default/"))
                 (cl-letf
                     (((symbol-function
                        'run-hook-with-args-until-success)
                       (lambda (&rest ignored)
                         ignored
                         guess))
                      ((symbol-function
                        'read-directory-name)
                       (lambda (&rest arguments)
                         (push arguments calls)
                         "/fixture/read/")))
                   (ack-read-dir)))))
           (list
            (scenario
             "/fixture/guessed/"
             nil
             "/fixture/file/name.el")
            (scenario
             "/fixture/guessed/"
             'unless-guessed
             nil)
            (scenario
             "/fixture/guessed/"
             t
             nil)
            (scenario
             nil nil
             "/fixture/file/name.el")
            (scenario
             nil nil nil)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("/fixture/guessed/" "/fixture/guessed/" "/fixture/read/" "/fixture/file/" "/fixture/default/" (("Directory: " "/fixture/guessed/" "/fixture/guessed/" t)))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_buffer_dir_selects_file_process_directory_or_home() {
    let elisp_form = r##"(let ((process-environment
                (copy-sequence
                 process-environment)))
         (setenv
          "HOME"
          "/fixture/home")
         (list
          (with-temp-buffer
            (setq buffer-file-name
                  "/fixture/project/file.el"
                  default-directory
                  "/fixture/project/"
                  major-mode
                  'emacs-lisp-mode)
            (ack-buffer-dir
             (current-buffer)))
          (with-temp-buffer
            (setq buffer-file-name
                  nil
                  default-directory
                  "/fixture/shell/"
                  major-mode
                  'shell-mode)
            (ack-buffer-dir
             (current-buffer)))
          (with-temp-buffer
            (setq buffer-file-name
                  nil
                  default-directory
                  "/fixture/plain/"
                  major-mode
                  'fundamental-mode)
            (ack-buffer-dir
             (current-buffer)))))"##;
    let expect = expect![[r#"OK ("/fixture/project/" "/fixture/shell/" "/fixture/home/")"#]];
    assert_ack_menu_parity(elisp_form, expect);
}
