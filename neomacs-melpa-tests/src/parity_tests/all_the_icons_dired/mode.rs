use expect_test::expect;

use super::assert_all_the_icons_dired_parity;

#[test]
fn all_the_icons_dired_setup_overrides_fontifier_manages_display_and_refontifies_jit() {
    let elisp_form = r##"(with-temp-buffer
         (let ((font-lock-fontify-region-function
                #'font-lock-default-fontify-region)
               (font-lock-extra-managed-props '(face))
               (jit-lock-mode t)
               calls)
           (cl-letf
               (((symbol-function 'jit-lock-refontify)
                 (lambda (&rest arguments)
                   (push (cons 'jit arguments) calls))))
             (all-the-icons-dired--setup)
             (list
              (eq font-lock-fontify-region-function
                  #'all-the-icons-dired--fontify-region)
              font-lock-extra-managed-props
              (nreverse calls)))))"##;
    let expect = expect!["OK (nil (display face) ((jit)))"];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_setup_refontifies_full_buffer_without_jit() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha\nbeta\n")
         (let ((font-lock-fontify-region-function
                #'font-lock-default-fontify-region)
               (font-lock-extra-managed-props nil)
               (jit-lock-mode nil)
               (font-lock-mode t)
               calls)
           (cl-letf
               (((symbol-function 'font-lock-fontify-region)
                 (lambda (&rest arguments)
                   (push arguments calls))))
             (all-the-icons-dired--setup)
             (list font-lock-extra-managed-props
                   (nreverse calls)
                   (eq font-lock-fontify-region-function
                       #'all-the-icons-dired--fontify-region)))))"##;
    let expect = expect!["OK ((display) ((1 12)) nil)"];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_teardown_unfontifies_removes_override_and_managed_property() {
    let elisp_form = r##"(with-temp-buffer
         (let ((font-lock-fontify-region-function
                #'font-lock-default-fontify-region)
               (font-lock-extra-managed-props '(face syntax))
               (jit-lock-mode t)
               calls)
           (add-function
            :override
            (local 'font-lock-fontify-region-function)
            #'all-the-icons-dired--fontify-region)
           (setq-local font-lock-extra-managed-props
                       (cons 'display
                             font-lock-extra-managed-props))
           (cl-letf
               (((symbol-function 'font-lock-unfontify-buffer)
                 (lambda () (push 'unfontify calls)))
                ((symbol-function 'jit-lock-refontify)
                 (lambda (&rest arguments)
                   (push (cons 'jit arguments) calls))))
             (all-the-icons-dired--teardown)
             (list
              (eq font-lock-fontify-region-function
                  #'font-lock-default-fontify-region)
              font-lock-extra-managed-props
              (nreverse calls)))))"##;
    let expect = expect!["OK (t (face syntax) (unfontify (jit)))"];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_minor_mode_only_sets_up_derived_dired_buffers() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (setq major-mode 'text-mode)
           (let (calls)
             (cl-letf
                 (((symbol-function
                    'all-the-icons-dired--setup)
                   (lambda () (push 'setup calls)))
                  ((symbol-function
                    'all-the-icons-dired--teardown)
                   (lambda () (push 'teardown calls))))
               (all-the-icons-dired-mode 1)
               (list all-the-icons-dired-mode calls))))
         (with-temp-buffer
           (dired-mode)
           (let (calls)
             (cl-letf
                 (((symbol-function
                    'all-the-icons-dired--setup)
                   (lambda () (push 'setup calls)))
                  ((symbol-function
                    'all-the-icons-dired--teardown)
                   (lambda () (push 'teardown calls))))
               (all-the-icons-dired-mode 1)
               (all-the-icons-dired-mode -1)
               (list all-the-icons-dired-mode
                     (nreverse calls))))))"##;
    let expect = expect!["OK ((t nil) (nil (setup teardown)))"];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_real_mode_lifecycle_restores_font_lock_state() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (directory (expand-file-name "mode-cycle" root))
               buffer)
         (make-directory directory t)
         (with-temp-file
             (expand-file-name "sample.el" directory)
           (insert "(message \"hello\")\n"))
         (setq buffer (dired-noselect directory))
         (unwind-protect
             (with-current-buffer buffer
               (let ((original
                      font-lock-fontify-region-function)
                     enabled disabled)
                 (all-the-icons-dired-mode 1)
                 (setq enabled
                       (list
                        all-the-icons-dired-mode
                        (eq
                         font-lock-fontify-region-function
                         #'all-the-icons-dired--fontify-region)
                        (memq
                         'display
                         font-lock-extra-managed-props)))
                 (all-the-icons-dired-mode -1)
                 (setq disabled
                       (list
                        all-the-icons-dired-mode
                        (eq
                         font-lock-fontify-region-function
                         original)
                        (memq
                         'display
                         font-lock-extra-managed-props)))
                 (list enabled disabled)))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect!["OK ((t nil (display)) (nil t nil))"];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}
