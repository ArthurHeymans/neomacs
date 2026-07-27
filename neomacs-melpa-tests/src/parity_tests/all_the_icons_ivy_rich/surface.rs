use expect_test::expect;

use super::assert_all_the_icons_ivy_rich_parity;

#[test]
fn pinned_package_loads_its_real_ivy_rich_ivy_and_all_the_icons_dependency_graph() {
    let elisp_form = r##"(let ((packages
                    '(all-the-icons-ivy-rich
                      ivy-rich
                      ivy
                      all-the-icons)))
               (list
                (mapcar
                 (lambda (package)
                   (list
                    package
                    (featurep package)
                    (let ((description
                           (cadr (assq package package-alist))))
                      (and description
                           (package-version-join
                            (package-desc-version description))))
                    (file-name-nondirectory
                     (or (locate-library (symbol-name package)) ""))))
                 packages)
                (mapcar
                 (lambda (feature)
                   (and (featurep feature) feature))
                 '(cl-lib subr-x package bookmark project))))"##;
    let expect = expect![[
        r#"OK (((all-the-icons-ivy-rich t "20230420.1234" "all-the-icons-ivy-rich.el") (ivy-rich t "20230425.1422" "ivy-rich.el") (ivy t "20260413.2102" "ivy.el") (all-the-icons t "20250527.927" "all-the-icons.el")) (cl-lib subr-x package bookmark project))"#
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn customization_defaults_and_ui_faces_match_the_installed_package_contract() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (symbol)
                  (list
                   symbol
                   (symbol-value symbol)
                   (get symbol 'custom-type)
                   (get symbol 'standard-value)))
                '(all-the-icons-ivy-rich-icon
                  all-the-icons-ivy-rich-color-icon
                  all-the-icons-ivy-rich-icon-size
                  all-the-icons-ivy-rich-project
                  all-the-icons-ivy-rich-field-width))
               (mapcar
                (lambda (face)
                  (list
                   face
                   (facep face)
                   (get face 'face-defface-spec)
                   (documentation-property
                    face 'face-documentation)))
                '(all-the-icons-ivy-rich-on-face
                  all-the-icons-ivy-rich-off-face
                  all-the-icons-ivy-rich-icon-face
                  all-the-icons-ivy-rich-file-priv-dir
                  all-the-icons-ivy-rich-file-priv-read
                  all-the-icons-ivy-rich-file-priv-write
                  all-the-icons-ivy-rich-file-priv-exec
                  all-the-icons-ivy-rich-process-status-alt-face
                  all-the-icons-ivy-rich-imenu-doc-face)))"##;
    let expect = expect![[
        r#"OK (((all-the-icons-ivy-rich-icon t boolean ((funcall #'#[nil (t) #1=(ivy-posframe-buffer counsel--fzf-dir t)]))) (all-the-icons-ivy-rich-color-icon t boolean ((funcall #'#[nil (t) #1#]))) (all-the-icons-ivy-rich-icon-size 1.0 float ((funcall #'#[nil (1.0) #1#]))) (all-the-icons-ivy-rich-project t boolean ((funcall #'#[nil (t) #1#]))) (all-the-icons-ivy-rich-field-width 80 integer ((funcall #'#[nil (80) #1#])))) ((all-the-icons-ivy-rich-on-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit success)) "Face used to signal enabled modes.") (all-the-icons-ivy-rich-off-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit shadow)) "Face used to signal disabled modes.") (all-the-icons-ivy-rich-icon-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit default))) "Face used for the icons while ‘all-the-icons-ivy-rich-color-icon’ is nil.") (all-the-icons-ivy-rich-file-priv-dir [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-keyword-face)) "Face used to highlight the dir file privilege attribute.") (all-the-icons-ivy-rich-file-priv-read [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-type-face)) "Face used to highlight the read file privilege attribute.") (all-the-icons-ivy-rich-file-priv-write [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-builtin-face)) "Face used to highlight the write file privilege attribute.") (all-the-icons-ivy-rich-file-priv-exec [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-function-name-face)) "Face used to highlight the exec file privilege attribute.") (all-the-icons-ivy-rich-process-status-alt-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit all-the-icons-ivy-rich-error-face)) "Face used for process status: stop, exit, closed and failed.") (all-the-icons-ivy-rich-imenu-doc-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit all-the-icons-ivy-rich-doc-face :height 0.9))) "Face used for imenu documentation.")))"#
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn transformer_registry_describes_real_buffer_file_symbol_package_and_process_columns() {
    let elisp_form = r##"(let ((callers
                    '(ivy-switch-buffer
                      counsel-find-file
                      counsel-describe-symbol
                      package-install
                      counsel-list-processes
                      project-find-file
                      counsel-rg
                      counsel-bookmark)))
               (list
                (length
                 all-the-icons-ivy-rich-display-transformers-list)
                (mapcar
                 (lambda (caller)
                   (let* ((tail
                           (memq
                            caller
                            all-the-icons-ivy-rich-display-transformers-list))
                          (configuration (cadr tail))
                          (columns (plist-get configuration :columns)))
                     (list
                      caller
                      (mapcar
                       (lambda (column)
                         (list
                          (car column)
                          (cdr column)))
                       columns)
                      (plist-get configuration :delimiter)
                      (functionp
                       (plist-get configuration :predicate)))))
                 callers)))"##;
    let expect = expect![[
        r#"OK (206 ((ivy-switch-buffer ((all-the-icons-ivy-rich-buffer-icon nil) (ivy-switch-buffer-transformer ((:width 0.3))) (ivy-rich-switch-buffer-size ((:width 7 :face all-the-icons-ivy-rich-size-face))) (ivy-rich-switch-buffer-indicators ((:width 4 :face all-the-icons-ivy-rich-indicator-face :align right))) (all-the-icons-ivy-rich-switch-buffer-major-mode ((:width 18 :face all-the-icons-ivy-rich-major-mode-face))) (ivy-rich-switch-buffer-project ((:width 0.12 :face all-the-icons-ivy-rich-project-face))) (ivy-rich-switch-buffer-path ((:width (lambda (x) (ivy-rich-switch-buffer-shorten-path x (ivy-rich-minibuffer-width 0.3))) :face all-the-icons-ivy-rich-path-face)))) "\11" t) (counsel-find-file ((all-the-icons-ivy-rich-file-icon nil) (all-the-icons-ivy-rich-file-name ((:width 0.4))) (all-the-icons-ivy-rich-file-id ((:width 15 :face all-the-icons-ivy-rich-file-owner-face :align right))) (all-the-icons-ivy-rich-file-modes ((:width 12))) (all-the-icons-ivy-rich-file-size ((:width 7 :face all-the-icons-ivy-rich-size-face))) (all-the-icons-ivy-rich-file-modification-time ((:face all-the-icons-ivy-rich-time-face)))) "\11" nil) (counsel-describe-symbol ((all-the-icons-ivy-rich-symbol-icon nil) (ivy-rich-candidate ((:width 0.3))) (all-the-icons-ivy-rich-symbol-class ((:width 8 :face all-the-icons-ivy-rich-type-face))) (all-the-icons-ivy-rich-symbol-docstring ((:face all-the-icons-ivy-rich-doc-face)))) "\11" nil) (package-install ((all-the-icons-ivy-rich-package-icon nil) (ivy-rich-candidate ((:width 0.25))) (all-the-icons-ivy-rich-package-version ((:width 16 :face all-the-icons-ivy-rich-version-face))) (all-the-icons-ivy-rich-package-status ((:width 12))) (all-the-icons-ivy-rich-package-archive-summary ((:width 7 :face all-the-icons-ivy-rich-archive-face))) (all-the-icons-ivy-rich-package-install-summary ((:face all-the-icons-ivy-rich-pacage-desc-face)))) "\11" nil) (counsel-list-processes ((all-the-icons-ivy-rich-process-icon nil) (ivy-rich-candidate ((:width 25))) (all-the-icons-ivy-rich-process-id ((:width 7 :face all-the-icons-ivy-rich-process-id-face))) (all-the-icons-ivy-rich-process-status ((:width 7))) (all-the-icons-ivy-rich-process-buffer-name ((:width 25 :face all-the-icons-ivy-rich-process-buffer-face))) (all-the-icons-ivy-rich-process-tty-name ((:width 12 :face all-the-icons-ivy-rich-process-tty-face))) (all-the-icons-ivy-rich-process-thread ((:width 12 :face all-the-icons-ivy-rich-process-thread-face))) (all-the-icons-ivy-rich-process-command ((:face all-the-icons-ivy-rich-process-command-face)))) "\11" nil) (project-find-file ((all-the-icons-ivy-rich-file-icon nil) (all-the-icons-ivy-rich-project-find-file-transformer ((:width 0.4))) (all-the-icons-ivy-rich-project-file-id ((:width 15 :face all-the-icons-ivy-rich-file-owner-face :align right))) (all-the-icons-ivy-rich-project-file-modes ((:width 12))) (all-the-icons-ivy-rich-project-file-size ((:width 7 :face all-the-icons-ivy-rich-size-face))) (all-the-icons-ivy-rich-project-file-modification-time ((:face all-the-icons-ivy-rich-time-face)))) "\11" nil) (counsel-rg ((all-the-icons-ivy-rich-grep-file-icon nil) (all-the-icons-ivy-rich-grep-transformer nil)) "\11" nil) (counsel-bookmark ((all-the-icons-ivy-rich-bookmark-icon nil) (all-the-icons-ivy-rich-bookmark-name ((:width 0.25))) (ivy-rich-bookmark-type ((:width 10))) (all-the-icons-ivy-rich-bookmark-filename ((:width 0.3 :face all-the-icons-ivy-rich-bookmark-face))) (all-the-icons-ivy-rich-bookmark-context ((:face all-the-icons-ivy-rich-doc-face)))) "\11" nil)))"#
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn global_mode_enable_reload_and_disable_manage_hooks_advice_and_transformers() {
    let elisp_form = r##"(let ((original
                    ivy-rich-display-transformers-list)
                   enabled
                   reloaded
                   disabled)
               (unwind-protect
                   (progn
                     (all-the-icons-ivy-rich-mode 1)
                     (setq
                      enabled
                      (list
                       all-the-icons-ivy-rich-mode
                       (not
                        (null
                         (memq
                          #'all-the-icons-ivy-rich-minibuffer-align-icons
                          minibuffer-setup-hook)))
                       (not
                        (null
                         (advice-member-p
                          #'all-the-icons-ivy-rich-kill-buffer
                          #'kill-buffer)))
                       (eq
                        ivy-rich-display-transformers-list
                        all-the-icons-ivy-rich-display-transformers-list)))
                     (all-the-icons-ivy-rich-reload)
                     (setq
                      reloaded
                      (list
                       all-the-icons-ivy-rich-mode
                       (not
                        (null
                         (advice-member-p
                          #'all-the-icons-ivy-rich-kill-buffer
                          #'kill-buffer)))
                       (eq
                        ivy-rich-display-transformers-list
                        all-the-icons-ivy-rich-display-transformers-list)))
                     (all-the-icons-ivy-rich-mode -1)
                     (setq
                      disabled
                      (list
                       all-the-icons-ivy-rich-mode
                       (memq
                        #'all-the-icons-ivy-rich-minibuffer-align-icons
                        minibuffer-setup-hook)
                       (advice-member-p
                        #'all-the-icons-ivy-rich-kill-buffer
                        #'kill-buffer)
                       (eq
                        ivy-rich-display-transformers-list
                        original)))
                     (list enabled reloaded disabled))
                 (when all-the-icons-ivy-rich-mode
                   (all-the-icons-ivy-rich-mode -1))))"##;
    let expect = expect!["OK ((t t t t) (t t t) (nil nil nil t))"];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn graphical_icon_gate_and_buffer_alignment_follow_runtime_state() {
    let elisp_form = r##"(progn
               (require 'cl-lib)
               (let ((all-the-icons-ivy-rich-icon t)
                     graphical
                     nongraphical
                     disabled
                     aligned)
                 (cl-letf
                     (((symbol-function 'display-graphic-p)
                       (lambda (&optional _frame) t)))
                   (setq graphical
                         (all-the-icons-ivy-rich-icon-displayable)))
                 (cl-letf
                     (((symbol-function 'display-graphic-p)
                       (lambda (&optional _frame) nil)))
                   (setq nongraphical
                         (all-the-icons-ivy-rich-icon-displayable)))
                 (setq all-the-icons-ivy-rich-icon nil)
                 (cl-letf
                     (((symbol-function 'display-graphic-p)
                       (lambda (&optional _frame) t)))
                   (setq disabled
                         (all-the-icons-ivy-rich-icon-displayable)))
                 (with-temp-buffer
                   (setq tab-width 8)
                   (all-the-icons-ivy-rich-minibuffer-align-icons)
                   (setq aligned tab-width))
                 (list graphical nongraphical disabled aligned)))"##;
    let expect = expect!["OK (t nil nil 1)"];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}
