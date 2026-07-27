use expect_test::expect;

use super::{
    assert_aggressive_fill_paragraph_autoload_parity, assert_aggressive_fill_paragraph_parity,
};

#[test]
fn aggressive_fill_paragraph_registry_defaults_custom_metadata_and_feature_match() {
    let elisp_form = r##"(list
         (featurep 'aggressive-fill-paragraph)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value symbol)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (local-variable-if-set-p symbol)))
          '(afp-suppress-fill-pfunction-list
            afp-fill-comments-only-mode-list
            afp-fill-keys))
         (get 'aggressive-fill-paragraph 'group-documentation)
         (get 'aggressive-fill-paragraph 'custom-group))"##;
    let expect = expect![
        "OK (t ((afp-suppress-fill-pfunction-list (afp-repeated-whitespace? afp-markdown-inside-code-block? afp-bullet-list-in-comments? afp-in-org-table? afp-in-org-src-block-header?) (repeat function) nil nil) (afp-fill-comments-only-mode-list (emacs-lisp-mode sh-mode python-mode js-mode) (repeat symbol) nil nil) (afp-fill-keys (32 46) (repeat character) nil nil)) nil ((afp-suppress-fill-pfunction-list custom-variable) (afp-fill-comments-only-mode-list custom-variable)))"
    ];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_complete_callable_surface_and_command_status_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (macrop symbol)
            (autoloadp (symbol-function symbol))))
         '(afp-inside-comment?
           afp-current-line
           afp-markdown-inside-code-block?
           afp-repeated-whitespace?
           afp-bullet-list-in-comments?
           afp-in-org-table?
           afp-in-org-src-block-header?
           afp-only-fill-comments
           afp-suppress-fill?
           afp-choose-fill-function
           aggressive-fill-paragraph-post-self-insert-function
           aggressive-fill-paragraph-mode
           afp-setup-recommended-hooks))"##;
    let expect = expect![
        "OK ((afp-inside-comment? nil nil nil nil) (afp-current-line nil nil nil nil) (afp-markdown-inside-code-block? nil nil nil nil) (afp-repeated-whitespace? nil nil nil nil) (afp-bullet-list-in-comments? nil nil nil nil) (afp-in-org-table? nil t nil nil) (afp-in-org-src-block-header? nil nil nil nil) (afp-only-fill-comments (&optional justify) nil nil nil) (afp-suppress-fill? nil nil nil nil) (afp-choose-fill-function nil nil nil nil) (aggressive-fill-paragraph-post-self-insert-function nil nil nil nil) (aggressive-fill-paragraph-mode (&optional arg) t nil nil) (afp-setup-recommended-hooks nil t nil nil))"
    ];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_autoloads_expose_mode_and_setup_without_eager_load() {
    let elisp_form = r##"(list
         (featurep 'aggressive-fill-paragraph)
         (mapcar
          (lambda (symbol)
            (let ((definition (symbol-function symbol)))
              (list
               symbol
               (autoloadp definition)
               (nth 1 definition)
               (nth 4 definition)
               (commandp symbol))))
          '(aggressive-fill-paragraph-mode
            afp-setup-recommended-hooks)))"##;
    let expect = expect![[
        r#"OK (nil ((aggressive-fill-paragraph-mode t "aggressive-fill-paragraph" nil t) (afp-setup-recommended-hooks t "aggressive-fill-paragraph" nil t)))"#
    ]];
    assert_aggressive_fill_paragraph_autoload_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_minor_mode_is_buffer_local_and_manages_only_its_hook() {
    let elisp_form = r##"(let ((sentinel (lambda () 'sentinel)))
         (with-temp-buffer
           (add-hook 'post-self-insert-hook sentinel nil t)
           (list
            aggressive-fill-paragraph-mode
            (local-variable-p 'aggressive-fill-paragraph-mode)
            (aggressive-fill-paragraph-mode 1)
            aggressive-fill-paragraph-mode
            (local-variable-p 'aggressive-fill-paragraph-mode)
            (mapcar
             (lambda (function)
               (cond
                ((eq function sentinel) 'sentinel)
                ((eq function
                     #'aggressive-fill-paragraph-post-self-insert-function)
                 'worker)
                (t function)))
             post-self-insert-hook)
            (aggressive-fill-paragraph-mode 1)
            (cl-count
             #'aggressive-fill-paragraph-post-self-insert-function
             post-self-insert-hook)
            (aggressive-fill-paragraph-mode -1)
            aggressive-fill-paragraph-mode
            (mapcar
             (lambda (function)
               (if (eq function sentinel) 'sentinel function))
             post-self-insert-hook))))"##;
    let expect = expect!["OK (nil nil t t t (worker sentinel t) t 1 nil nil (sentinel t))"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_recommended_setup_adds_idempotent_major_mode_hooks() {
    let elisp_form = r##"(let ((text-mode-hook nil)
               (prog-mode-hook nil))
         (list
          (afp-setup-recommended-hooks)
          text-mode-hook
          prog-mode-hook
          (afp-setup-recommended-hooks)
          text-mode-hook
          prog-mode-hook
          (with-temp-buffer
            (text-mode)
            aggressive-fill-paragraph-mode)
          (with-temp-buffer
            (emacs-lisp-mode)
            aggressive-fill-paragraph-mode)))"##;
    let expect = expect![
        "OK (#1=(aggressive-fill-paragraph-mode) #2=(aggressive-fill-paragraph-mode) #1# #1# #2# #1# t t)"
    ];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}
