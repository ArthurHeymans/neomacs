use expect_test::expect;

use super::assert_ah_parity;

#[test]
fn ah_user_theme_wrapper_orders_before_original_and_after_hooks() {
    let elisp_form = r##"(let* ((events nil)
               (ah-before-enable-theme-hook
                (list (lambda () (push 'before events))))
               (ah-after-enable-theme-hook
                (list (lambda () (push 'after events)))))
         (list
          (ah--enable-theme
           (lambda (theme) (push (list 'enable theme) events) 'enabled)
           'user)
          (nreverse events)))"##;
    let expect = expect!["OK (nil (before (enable user) after))"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_non_user_theme_wrapper_calls_original_without_additional_hooks() {
    let elisp_form = r##"(let* ((events nil)
               (ah-before-enable-theme-hook
                (list (lambda () (push 'before events))))
               (ah-after-enable-theme-hook
                (list (lambda () (push 'after events)))))
         (list
          (ah--enable-theme
           (lambda (theme) (push (list 'enable theme) events) 'enabled)
           'wombat)
          (nreverse events)))"##;
    let expect = expect!["OK (nil ((enable wombat)))"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_after_theme_hook_is_skipped_when_enable_theme_signals() {
    let elisp_form = r##"(let* ((events nil)
               (ah-before-enable-theme-hook
                (list (lambda () (push 'before events))))
               (ah-after-enable-theme-hook
                (list (lambda () (push 'after events)))))
         (list
          (condition-case error-data
              (ah--enable-theme
               (lambda (_theme) (push 'original events) (error "theme failed"))
               'user)
            (error (list (car error-data) (cadr error-data))))
          (nreverse events)))"##;
    let expect = expect![[r#"OK ((error "theme failed") (before original))"#]];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_real_user_theme_enablement_runs_hooks_around_emacs_theme_activation() {
    let elisp_form = r##"(let* ((events nil)
               (ah-before-enable-theme-hook
                (list (lambda () (push (list 'before custom-enabled-themes) events))))
               (ah-after-enable-theme-hook
                (list (lambda () (push (list 'after custom-enabled-themes) events)))))
         (unwind-protect
             (progn
               (ah-mode 1)
               (enable-theme 'user)
               (list (memq 'user custom-enabled-themes)
                     (nreverse events)))
           (disable-theme 'user)
           (ah-mode -1)))"##;
    let expect = expect!["OK (nil ((before nil) (after nil)))"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_real_load_theme_workflow_observes_generated_theme_activation() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "ah-theme"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (theme-file
                 (expand-file-name "ah-practical-theme-theme.el" root))
                (custom-theme-load-path (cons root custom-theme-load-path))
                (events nil)
                (ah-before-enable-theme-hook
                 (list (lambda () (push 'before events))))
                (ah-after-enable-theme-hook
                 (list (lambda () (push 'after events)))))
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "(deftheme ah-practical-theme)\n(custom-theme-set-variables 'ah-practical-theme '(fill-column 91))\n(provide-theme 'ah-practical-theme)\n"
                nil theme-file nil 'silent)
               (ah-mode 1)
               (load-theme 'ah-practical-theme t)
               (list
                (memq 'ah-practical-theme custom-enabled-themes)
                fill-column
                (nreverse events)))
           (when (custom-theme-p 'ah-practical-theme)
             (disable-theme 'ah-practical-theme))
           (ah-mode -1)
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect!["OK ((ah-practical-theme) 91 (before after))"];
    assert_ah_parity(elisp_form, expect);
}
