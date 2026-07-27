use expect_test::expect;

use super::assert_all_the_icons_ivy_parity;

#[test]
fn all_the_icons_ivy_buffer_propertize_marks_only_modified_file_buffers() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               results)
         (dolist (case '((plain nil nil)
                         (saved t nil)
                         (modified t t)))
           (let* ((name (format " *ivy-%s*" (car case)))
                  (buffer (get-buffer-create name))
                  (has-file (cadr case))
                  (modified (caddr case)))
             (unwind-protect
                 (with-current-buffer buffer
                   (when has-file
                     (setq buffer-file-name
                           (expand-file-name
                            (format "%s.txt" (car case))
                            root)))
                   (set-buffer-modified-p modified)
                   (let ((candidate
                          (all-the-icons-ivy--buffer-propertize
                           buffer name)))
                     (push
                      (list
                       (car case)
                       candidate
                       (text-properties-at 0 candidate))
                      results)))
               (kill-buffer buffer))))
         (nreverse results))"##;
    let expect = expect![[
        r#"OK ((plain " *ivy-plain*" nil) (saved " *ivy-saved*" nil) (modified #(" *ivy-modified*" 0 15 (face ivy-modified-buffer)) (face ivy-modified-buffer)))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_icon_for_mode_accepts_real_icons_and_rejects_sentinel_symbols() {
    let elisp_form = r##"(mapcar
         (lambda (mode)
           (let ((icon
                  (all-the-icons-ivy--icon-for-mode mode)))
             (list mode
                   (and icon (string-to-list icon))
                   (and icon
                        (all-the-icons-icon-family icon))
                   (and icon
                        (text-properties-at 0 icon)))))
         '(emacs-lisp-mode rust-mode
           fundamental-mode unknown-ivy-mode))"##;
    let expect = expect![[
        r#"OK ((emacs-lisp-mode (59686) "file-icons" (face #1=(:family "file-icons" :height 1.2 :inherit all-the-icons-purple) font-lock-face #1# display (raise -0.12) rear-nonsticky t)) (rust-mode (59692) "all-the-icons" (face #2=(:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) font-lock-face #2# display (raise -0.24) rear-nonsticky t)) (fundamental-mode (59686) "file-icons" (face #3=(:family "file-icons" :height 1.2 :inherit all-the-icons-dsilver) font-lock-face #3# display (raise -0.12) rear-nonsticky t)) (unknown-ivy-mode nil nil nil))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_buffer_transformer_uses_direct_mode_icon_and_modified_face() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (buffer
                (get-buffer-create "ivy-direct.el")))
         (unwind-protect
             (with-current-buffer buffer
               (emacs-lisp-mode)
               (setq buffer-file-name
                     (expand-file-name "ivy-direct.el" root))
               (set-buffer-modified-p t)
               (let ((candidate
                      (all-the-icons-ivy-buffer-transformer
                       (buffer-name))))
                 (list
                  candidate
                  (substring-no-properties candidate)
                  (get-text-property 0 'display candidate)
                  (text-properties-at
                   (1- (length candidate))
                   candidate))))
           (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (#("\11\11ivy-direct.el" 0 1 (display #("" 0 1 (face #1=(:family "file-icons" :height 1.2 :inherit all-the-icons-purple) font-lock-face #1# display #2=(raise -0.12) rear-nonsticky t))) 2 15 (face ivy-modified-buffer)) "\11\11ivy-direct.el" #("" 0 1 (face #1# font-lock-face #1# display #2# rear-nonsticky t)) (face ivy-modified-buffer))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_buffer_transformer_walks_one_derived_mode_parent() {
    let elisp_form = r##"(let ((buffer
                (get-buffer-create " *ivy-derived*"))
               calls)
         (unwind-protect
             (with-current-buffer buffer
               (setq major-mode 'ivy-child-mode)
               (put 'ivy-child-mode
                    'derived-mode-parent 'emacs-lisp-mode)
               (cl-letf
                   (((symbol-function
                      'all-the-icons-ivy--icon-for-mode)
                     (lambda (mode)
                       (push mode calls)
                       (and (eq mode 'emacs-lisp-mode)
                            "PARENT"))))
                 (let ((candidate
                        (all-the-icons-ivy-buffer-transformer
                         (buffer-name))))
                   (list
                    (substring-no-properties candidate)
                    (get-text-property
                     0 'display candidate)
                    (nreverse calls)))))
           (put 'ivy-child-mode 'derived-mode-parent nil)
           (kill-buffer buffer)))"##;
    let expect =
        expect![[r#"OK ("\11\11 *ivy-derived*" "PARENT" (ivy-child-mode emacs-lisp-mode))"#]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_buffer_transformer_uses_configured_family_fallback() {
    let elisp_form = r##"(let ((buffer
                (get-buffer-create " *ivy-fallback*"))
               (all-the-icons-ivy-family-fallback-for-buffer
                (lambda (name &rest arguments)
                  (format "fallback:%s:%S" name arguments)))
               (all-the-icons-ivy-name-fallback-for-buffer
                "paper")
               (all-the-icons-spacer " | "))
         (unwind-protect
             (with-current-buffer buffer
               (setq major-mode 'no-icon-mode)
               (cl-letf
                   (((symbol-function
                      'all-the-icons-ivy--icon-for-mode)
                     (lambda (_mode) nil)))
                 (let ((candidate
                        (all-the-icons-ivy-buffer-transformer
                         (buffer-name))))
                   (list
                    (substring-no-properties candidate)
                    (get-text-property
                     0 'display candidate)))))
           (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ("\11 |  *ivy-fallback*" "fallback:paper:nil")"#]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_missing_buffer_candidate_falls_back_to_file_pipeline() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'all-the-icons-ivy-file-transformer)
               (lambda (candidate)
                 (push candidate calls)
                 (concat "FILE:" candidate))))
           (list
            (all-the-icons-ivy-buffer-transformer
             "definitely-not-a-live-buffer.rs")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("FILE:definitely-not-a-live-buffer.rs" ("definitely-not-a-live-buffer.rs"))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}
