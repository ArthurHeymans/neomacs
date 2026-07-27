use expect_test::expect;

use super::{assert_ah_autoload_parity, assert_ah_parity};

#[test]
fn ah_exact_package_metadata_feature_version_and_custom_defaults_match() {
    let elisp_form = r##"(progn
         (require 'lisp-mnt)
         (let ((descriptor (cadr (assq 'ah package-alist))))
           (list
            (package-desc-name descriptor)
            (package-version-join (package-desc-version descriptor))
            (package-desc-summary descriptor)
            (package-desc-kind descriptor)
            (package-desc-reqs descriptor)
            (package-desc-extras descriptor)
            (featurep 'ah)
            (with-temp-buffer
              (insert-file-contents (getenv "NEOMACS_PACKAGE_SOURCE"))
              (lm-header "version"))
            ah-lighter
            ah-before-move-cursor-hook
            ah-after-move-cursor-hook
            ah-before-c-g-hook
            ah-after-c-g-hook
            ah-before-enable-theme-hook
            ah-after-enable-theme-hook
            ah-mode)))"##;
    let expect = expect![[
        r#"OK (ah "20220730.1058" "Additional hooks." nil ((emacs (25 1))) ((:maintainers ("Takaaki ISHIKAWA" . "takaxpatieeedotorg")) (:authors ("Takaaki ISHIKAWA" . "takaxpatieeedotorg")) (:keywords "convenience") (:revdesc . "8e12223f0f42") (:commit . "8e12223f0f423e7fa882cc049a25af6db755902d") (:url . "https://github.com/takaxp/ah")) t nil " Hooks" nil nil nil nil nil nil nil)"#
    ]];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_complete_callable_surface_arglists_commands_and_source_files_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (interactive-form symbol)
            (let ((file (symbol-file symbol 'defun)))
              (and file (file-name-nondirectory file)))))
         '(ah--cur-next-line
           ah--cur-previous-line
           ah--cur-forward-char
           ah--cur-backward-char
           ah--cur-syntax-subword-forward
           ah--cur-syntax-subword-backward
           ah--cur-move-beginning-of-line
           ah--cur-move-end-of-line
           ah--cur-beginning-of-buffer
           ah--cur-end-of-buffer
           ah--cg-post-processing
           ah--cg-keyboard-quit
           ah--cg-isearch-abort
           ah--enable-theme
           ah--setup
           ah--abort
           ah-mode))"##;
    let expect = expect![[
        r#"OK ((ah--cur-next-line (f &optional arg try-vscroll) nil nil "ah.el") (ah--cur-previous-line (f &optional arg try-vscroll) nil nil "ah.el") (ah--cur-forward-char (f &optional N) nil nil "ah.el") (ah--cur-backward-char (f &optional N) nil nil "ah.el") (ah--cur-syntax-subword-forward (f &optional N) nil nil "ah.el") (ah--cur-syntax-subword-backward (f &optional N) nil nil "ah.el") (ah--cur-move-beginning-of-line (f ARG) nil nil "ah.el") (ah--cur-move-end-of-line (f ARG) nil nil "ah.el") (ah--cur-beginning-of-buffer (f &optional ARG) nil nil "ah.el") (ah--cur-end-of-buffer (f &optional ARG) nil nil "ah.el") (ah--cg-post-processing nil nil nil "ah.el") (ah--cg-keyboard-quit (f) nil nil "ah.el") (ah--cg-isearch-abort (f) nil nil "ah.el") (ah--enable-theme (f theme) nil nil "ah.el") (ah--setup nil nil nil "ah.el") (ah--abort nil nil nil "ah.el") (ah-mode (&optional arg) t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "ah.el"))"#
    ]];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_custom_variable_metadata_and_hook_contracts_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (custom-variable-p symbol)
            (get symbol 'custom-type)
            (get symbol 'standard-value)
            (get symbol 'custom-requests)
            (get symbol 'variable-documentation)))
         '(ah-lighter
           ah-before-move-cursor-hook
           ah-after-move-cursor-hook
           ah-before-c-g-hook
           ah-after-c-g-hook
           ah-before-enable-theme-hook
           ah-after-enable-theme-hook))"##;
    let expect = expect![[
        r#"OK ((ah-lighter #1=((funcall #'#[nil (" Hooks") #2=(t)])) string #1# nil "Lighter for this.") (ah-before-move-cursor-hook #3=((funcall #'#[nil (nil) #2#])) hook #3# nil "Hook runs before moving the cursor.") (ah-after-move-cursor-hook #4=((funcall #'#[nil (nil) #2#])) hook #4# nil "Hook runs after moving the cursor.") (ah-before-c-g-hook #5=((funcall #'#[nil (nil) #2#])) hook #5# nil "Hook runs before \\[keyboard-quit] and related commands.") (ah-after-c-g-hook #6=((funcall #'#[nil (nil) #2#])) hook #6# nil "Hook runs after \\[keyboard-quit] and related commands.") (ah-before-enable-theme-hook #7=((funcall #'#[nil (nil) #2#])) hook #7# nil "Hook runs before \\[enable-theme].") (ah-after-enable-theme-hook #8=((funcall #'#[nil (nil) #2#])) hook #8# nil "Hook runs after \\[enable-theme]."))"#
    ]];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_generated_autoload_exposes_only_the_global_minor_mode_entrypoint() {
    let elisp_form = r##"(list
         (featurep 'ah-autoloads)
         (featurep 'ah)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (and (fboundp symbol)
                  (autoloadp (symbol-function symbol)))
             (commandp symbol)
             (and (fboundp symbol)
                  (help-function-arglist symbol t))))
          '(ah-mode ah--setup ah--abort ah--enable-theme)))"##;
    let expect = expect![[
        r#"OK (t nil ((ah-mode t t t "[Arg list not available until function definition is loaded.]") (ah--setup nil nil nil nil) (ah--abort nil nil nil nil) (ah--enable-theme nil nil nil nil)))"#
    ]];
    assert_ah_autoload_parity(elisp_form, expect);
}

#[test]
fn ah_lighter_tracks_runtime_customization_without_recreating_mode_state() {
    let elisp_form = r##"(let ((ah-lighter " Hooks"))
         (unwind-protect
             (progn
               (ah-mode 1)
               (let ((first (format-mode-line minor-mode-alist)))
                 (setq ah-lighter " AH!")
                 (list
                  ah-mode
                  first
                  (format-mode-line minor-mode-alist)
                  (assq 'ah-mode minor-mode-alist))))
           (ah-mode -1)))"##;
    let expect = expect![[r#"OK (t "" "" (ah-mode (:eval (format "%s" ah-lighter))))"#]];
    assert_ah_parity(elisp_form, expect);
}
