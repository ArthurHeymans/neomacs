use expect_test::expect;

use super::assert_aggressive_fill_paragraph_parity;

#[test]
fn aggressive_fill_paragraph_choose_fill_function_prefers_comment_only_for_configured_modes() {
    let elisp_form = r##"(let ((afp-fill-comments-only-mode-list
                '(prog-mode text-mode)))
         (list
          (with-temp-buffer
            (emacs-lisp-mode)
            (eq (afp-choose-fill-function)
                #'afp-only-fill-comments))
          (with-temp-buffer
            (text-mode)
            (eq (afp-choose-fill-function)
                #'afp-only-fill-comments))
          (with-temp-buffer
            (fundamental-mode)
            (eq (afp-choose-fill-function)
                #'fill-paragraph))))"##;
    let expect = expect!["OK (t t t)"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_choose_fill_function_honors_buffer_local_override() {
    let elisp_form = r##"(let ((custom-fill (lambda (&optional _justify) 'custom-result))
               (afp-fill-comments-only-mode-list nil))
         (with-temp-buffer
           (setq-local fill-paragraph-function custom-fill)
           (list
            (eq (afp-choose-fill-function) custom-fill)
            (funcall (afp-choose-fill-function))
            (local-variable-p 'fill-paragraph-function))))"##;
    let expect = expect!["OK (t custom-result t)"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_only_fill_comments_reflows_real_comment_and_leaves_code() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (setq fill-column 38)
         (insert ";; This comment contains enough words to require practical paragraph filling while preserving prefixes.\n")
         (insert "(message \"code remains untouched\")")
         (goto-char (point-min))
         (search-forward "enough")
         (list
          (afp-only-fill-comments)
          (buffer-string)
          (point)
          (afp-inside-comment?)))"##;
    let expect = expect![[
        r#"OK (t ";; This comment contains enough words\n;; to require practical paragraph\n;; filling while preserving prefixes.\n(message \"code remains untouched\")" 32 t)"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_comment_only_fill_outside_comment_reports_completion_without_edit() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (setq fill-column 20)
         (insert "(message \"a long string that should not be refilled as source code\")")
         (goto-char (point-min))
         (let ((before (buffer-string)))
           (list
            (afp-only-fill-comments)
            (equal before (buffer-string))
            (buffer-string))))"##;
    let expect = expect![[
        r#"OK (t t "(message \"a long string that should not be refilled as source code\")")"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}
