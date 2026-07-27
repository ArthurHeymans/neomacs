use expect_test::expect;

use super::assert_all_the_icons_ibuffer_parity;

#[test]
fn icon_column_dispatches_real_file_buffer_to_basename_with_exact_render_options() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "ati-report"))
      calls)
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq buffer-file-name "/workspace/reports/quarterly-report.rs"
                major-mode 'rust-mode))
        (cl-letf (((symbol-function 'all-the-icons-auto-mode-match?)
                   (lambda (&optional _file) t))
                  ((symbol-function 'all-the-icons-icon-for-file)
                   (lambda (file &rest args)
                     (setq calls (list file args))
                     (propertize "F" 'face '(:family "FileFamily"
                                             :foreground "blue")))))
          (let ((all-the-icons-ibuffer-display-predicate (lambda () t))
                (all-the-icons-ibuffer-icon-size 1.25)
                (all-the-icons-ibuffer-icon-v-adjust -0.1)
                (all-the-icons-ibuffer-color-icon t))
            (with-temp-buffer
              (funcall (ibuffer-compile-format '(icon)) buffer ?\s)
              (let ((rendered (buffer-string)))
                (list rendered calls
                      (get-text-property 0 'face rendered)
                      (get-text-property 1 'display rendered)))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("F " 0 1 (face (:family "FileFamily" :foreground "blue")) 1 2 (display ((space :relative-width 0.5)))) ("quarterly-report.rs" (:height 1.25 :v-adjust -0.1)) (:family "FileFamily" :foreground "blue") ((space :relative-width 0.5)))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn icon_column_dispatches_dired_buffer_to_directory_icon_with_directory_face() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "/workspace/project/"))
      calls)
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq major-mode 'dired-mode))
        (cl-letf (((symbol-function 'all-the-icons-icon-for-dir)
                   (lambda (directory &rest args)
                     (setq calls (list directory args))
                     (propertize "D" 'face '(:family "DirectoryFamily"
                                             :foreground "gold")))))
          (let ((all-the-icons-ibuffer-display-predicate (lambda () t))
                (all-the-icons-ibuffer-icon-size 1.1)
                (all-the-icons-ibuffer-icon-v-adjust 0.15))
            (with-temp-buffer
              (funcall (ibuffer-compile-format '(icon)) buffer ?\s)
              (list (buffer-string) calls)))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("D " 0 1 (face (:family "DirectoryFamily" :foreground "gold")) 1 2 (display ((space :relative-width 0.5)))) ("/workspace/project/" (:height 1.1 :v-adjust 0.15 :face all-the-icons-ibuffer-dir-face)))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn icon_column_dispatches_nonfile_buffer_to_major_mode_icon() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "*ati-compilation*"))
      calls)
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq major-mode 'compilation-mode))
        (cl-letf (((symbol-function 'all-the-icons-icon-for-mode)
                   (lambda (mode &rest args)
                     (setq calls (list mode args))
                     (propertize "M" 'face '(:family "ModeFamily"
                                             :foreground "green")))))
          (let ((all-the-icons-ibuffer-display-predicate (lambda () t))
                (all-the-icons-ibuffer-icon-size 0.9)
                (all-the-icons-ibuffer-icon-v-adjust -0.05))
            (with-temp-buffer
              (funcall (ibuffer-compile-format '(icon)) buffer ?\s)
              (list (buffer-string) calls)))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("M " 0 1 (face (:family "ModeFamily" :foreground "green")) 1 2 (display ((space :relative-width 0.5)))) (compilation-mode (:height 0.9 :v-adjust -0.05)))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn icon_column_falls_back_for_unknown_mode_and_scales_default_icon() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "*ati-unknown*"))
      calls)
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq major-mode 'ati-unknown-mode))
        (cl-letf (((symbol-function 'all-the-icons-icon-for-mode)
                   (lambda (mode &rest args)
                     (push (list 'mode mode args) calls)
                     mode))
                  ((symbol-function 'all-the-icons-faicon)
                   (lambda (name &rest args)
                     (push (list 'fallback name args) calls)
                     (propertize "?" 'face '(:family "FallbackFamily")))))
          (let ((all-the-icons-ibuffer-display-predicate (lambda () t))
                (all-the-icons-ibuffer-icon-size 1.5)
                (all-the-icons-ibuffer-icon-v-adjust 0.2)
                (all-the-icons-ibuffer-color-icon nil))
            (with-temp-buffer
              (funcall (ibuffer-compile-format '(icon)) buffer ?\s)
              (list (buffer-string) (nreverse calls))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("? " 0 1 (face (:family "FallbackFamily")) 1 2 (display ((space :relative-width 0.5)))) ((mode ati-unknown-mode (:height 1.5 :v-adjust 0.2)) (fallback "file-o" (:face all-the-icons-ibuffer-icon-face :height 1.35 :v-adjust 0.2))))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn icon_column_noncolor_mode_replaces_color_but_preserves_font_family() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "*ati-help*")))
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq major-mode 'help-mode))
        (cl-letf (((symbol-function 'all-the-icons-icon-for-mode)
                   (lambda (&rest _)
                     (propertize "H" 'face
                                 '(:family "HelpIconFont"
                                   :foreground "#ff00ff"
                                   :height 1.7)))))
          (let ((all-the-icons-ibuffer-display-predicate (lambda () t))
                (all-the-icons-ibuffer-color-icon nil))
            (with-temp-buffer
              (funcall (ibuffer-compile-format '(icon)) buffer ?\s)
              (let ((rendered (buffer-string)))
                (list rendered
                      (get-text-property 0 'face rendered)
                      (get-text-property 1 'display rendered)))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("H " 0 1 (face (:inherit all-the-icons-ibuffer-icon-face :family "HelpIconFont")) 1 2 (display ((space :relative-width 0.5)))) (:inherit all-the-icons-ibuffer-icon-face :family "HelpIconFont") ((space :relative-width 0.5)))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn icon_column_skips_every_icon_lookup_when_disabled_or_display_predicate_is_false() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "ati-disabled.rs"))
      (calls 0))
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (setq buffer-file-name "/workspace/ati-disabled.rs"
                major-mode 'rust-mode))
        (cl-letf (((symbol-function 'all-the-icons-auto-mode-match?)
                   (lambda (&optional _)
                     (setq calls (1+ calls))
                     t))
                  ((symbol-function 'all-the-icons-icon-for-file)
                   (lambda (&rest _)
                     (setq calls (1+ calls))
                     "F")))
          (mapcar
           (lambda (settings)
             (let ((all-the-icons-ibuffer-icon (car settings))
                   (all-the-icons-ibuffer-display-predicate
                    (lambda () (cadr settings))))
               (with-temp-buffer
                 (funcall (ibuffer-compile-format '(icon)) buffer ?\s)
                 (list settings (buffer-string) calls))))
           '((nil t) (t nil)))))
    (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK (((nil t) "" 0) ((t nil) "" 0))"#]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn real_all_the_icons_dependency_maps_practical_files_and_modes_to_stable_icon_families() {
    let elisp_form = r##"(list
 (mapcar
  (lambda (file)
    (let ((icon (all-the-icons-icon-for-file file
                                             :height 1.0
                                             :v-adjust 0.0)))
      (list file
            (string-to-list icon)
            (plist-get (get-text-property 0 'face icon) :family))))
  '("report.rs" "component.tsx" "README.md" "Dockerfile"
    "archive.tar.gz" "unknown.extension"))
 (mapcar
  (lambda (mode)
    (let ((icon (all-the-icons-icon-for-mode mode
                                             :height 1.0
                                             :v-adjust 0.0)))
      (list mode
            (if (symbolp icon) icon (string-to-list icon))
            (and (stringp icon)
                 (plist-get
                  (get-text-property 0 'face icon)
                  :family)))))
  '(emacs-lisp-mode dired-mode compilation-mode
    text-mode ati-missing-mode)))"##;
    let expect = expect![[
        r#"OK ((("report.rs" (59692) "all-the-icons") ("component.tsx" (59857) "file-icons") ("README.md" (61447) "github-octicons") ("Dockerfile" (61702) "file-icons") ("archive.tar.gz" (61588) "github-octicons") ("unknown.extension" (61462) "FontAwesome")) ((emacs-lisp-mode (59686) "file-icons") (dired-mode (61462) "github-octicons") (compilation-mode (61573) "FontAwesome") (text-mode (61457) "github-octicons") (ati-missing-mode ati-missing-mode nil)))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn real_auto_mode_matching_selects_file_icon_only_for_matching_buffer_mode() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (setq buffer-file-name (car case)
           major-mode (cadr case))
     (list case
           (all-the-icons-auto-mode-match?)
           (all-the-icons-auto-mode-match? (car case)))))
 '(("/workspace/service.rs" rust-mode)
   ("/workspace/init.el" emacs-lisp-mode)
   ("/workspace/notes.txt" text-mode)
   ("/workspace/component.tsx" fundamental-mode)
   ("/workspace/Makefile" makefile-mode)))"##;
    let expect = expect![[
        r#"OK ((("/workspace/service.rs" rust-mode) nil nil) (("/workspace/init.el" emacs-lisp-mode) t t) (("/workspace/notes.txt" text-mode) t t) (("/workspace/component.tsx" fundamental-mode) nil nil) (("/workspace/Makefile" makefile-mode) nil nil))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}
