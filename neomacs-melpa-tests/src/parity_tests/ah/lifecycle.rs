use expect_test::expect;

use super::assert_ah_parity;

#[test]
fn ah_setup_installs_the_complete_documented_advice_matrix() {
    let elisp_form = r##"(unwind-protect
         (progn
           (ah--setup)
           (mapcar
            (lambda (entry)
              (list
               (car entry)
               (and
                (advice-member-p (cdr entry) (car entry))
                t)))
            '((next-line . ah--cur-next-line)
              (previous-line . ah--cur-previous-line)
              (forward-char . ah--cur-forward-char)
              (backward-char . ah--cur-backward-char)
              (syntax-subword-forward . ah--cur-syntax-subword-forward)
              (syntax-subword-backward . ah--cur-syntax-subword-backward)
              (move-beginning-of-line . ah--cur-move-beginning-of-line)
              (move-end-of-line . ah--cur-move-end-of-line)
              (beginning-of-buffer . ah--cur-beginning-of-buffer)
              (end-of-buffer . ah--cur-end-of-buffer)
              (keyboard-quit . ah--cg-keyboard-quit)
              (isearch-abort . ah--cg-isearch-abort)
              (enable-theme . ah--enable-theme))))
       (ah--abort))"##;
    let expect = expect![
        "OK ((next-line t) (previous-line t) (forward-char t) (backward-char t) (syntax-subword-forward t) (syntax-subword-backward t) (move-beginning-of-line t) (move-end-of-line t) (beginning-of-buffer t) (end-of-buffer t) (keyboard-quit t) (isearch-abort t) (enable-theme t))"
    ];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_abort_removes_every_package_advice_after_setup() {
    let elisp_form = r##"(progn
         (ah--setup)
         (ah--abort)
         (mapcar
          (lambda (entry)
            (list
             (car entry)
             (advice-member-p (cdr entry) (car entry))))
          '((next-line . ah--cur-next-line)
            (previous-line . ah--cur-previous-line)
            (forward-char . ah--cur-forward-char)
            (backward-char . ah--cur-backward-char)
            (syntax-subword-forward . ah--cur-syntax-subword-forward)
            (syntax-subword-backward . ah--cur-syntax-subword-backward)
            (move-beginning-of-line . ah--cur-move-beginning-of-line)
            (move-end-of-line . ah--cur-move-end-of-line)
            (beginning-of-buffer . ah--cur-beginning-of-buffer)
            (end-of-buffer . ah--cur-end-of-buffer)
            (keyboard-quit . ah--cg-keyboard-quit)
            (isearch-abort . ah--cg-isearch-abort)
            (enable-theme . ah--enable-theme))))"##;
    let expect = expect![
        "OK ((next-line nil) (previous-line nil) (forward-char nil) (backward-char nil) (syntax-subword-forward nil) (syntax-subword-backward nil) (move-beginning-of-line nil) (move-end-of-line nil) (beginning-of-buffer nil) (end-of-buffer nil) (keyboard-quit nil) (isearch-abort nil) (enable-theme nil))"
    ];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_global_mode_toggle_updates_state_and_advice_lifecycle() {
    let elisp_form = r##"(unwind-protect
         (let ((initial ah-mode))
           (ah-mode 1)
           (let ((enabled
                  (list
                   ah-mode
                   (and
                    (advice-member-p #'ah--cur-forward-char 'forward-char)
                    t)
                   (and
                    (advice-member-p #'ah--cg-keyboard-quit 'keyboard-quit)
                    t)
                   (and
                    (advice-member-p #'ah--enable-theme 'enable-theme)
                    t))))
             (ah-mode -1)
             (list
              initial
              enabled
              ah-mode
              (and
               (advice-member-p #'ah--cur-forward-char 'forward-char)
               t)
              (and
               (advice-member-p #'ah--cg-keyboard-quit 'keyboard-quit)
               t)
              (and
               (advice-member-p #'ah--enable-theme 'enable-theme)
               t))))
       (ah-mode -1))"##;
    let expect = expect!["OK (nil (t t t t) nil nil nil nil)"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_repeated_setup_does_not_duplicate_real_interactive_hook_delivery() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abc")
         (goto-char 1)
         (let* ((before 0)
                (after 0)
                (ah-before-move-cursor-hook
                (list (lambda () (setq before (1+ before)))))
               (ah-after-move-cursor-hook
                (list (lambda () (setq after (1+ after))))))
           (unwind-protect
               (progn
                 (ah--setup)
                 (ah--setup)
                 (call-interactively #'forward-char)
                 (list (point) before after))
             (ah--abort))))"##;
    let expect = expect!["OK (2 1 1)"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_abort_preserves_unrelated_third_party_advice_on_same_command() {
    let elisp_form = r##"(let* ((calls nil)
               (third-party
                (lambda (function &rest args)
                  (push 'third-party calls)
                  (apply function args))))
         (unwind-protect
             (progn
               (advice-add 'forward-char :around third-party)
               (ah--setup)
               (ah--abort)
               (with-temp-buffer
                 (insert "ab")
                 (goto-char 1)
                 (forward-char 1)
                 (list
                  (point)
                  (advice-member-p third-party 'forward-char)
                  (advice-member-p #'ah--cur-forward-char 'forward-char)
                  (nreverse calls))))
           (advice-remove 'forward-char third-party)
           (ah--abort)))"##;
    let expect = expect![[
        r#"OK (2 #[128 "������\3#��" [#[(function &rest args) ((setq calls (cons 'third-party calls)) (apply function args)) ((calls third-party))] #<subr forward-char> :around nil apply] 5 advice] nil (third-party))"#
    ]];
    assert_ah_parity(elisp_form, expect);
}
