use expect_test::expect;

use super::assert_aggressive_fill_paragraph_parity;

#[test]
fn aggressive_fill_paragraph_text_workflow_reflows_on_space_and_preserves_inserted_character() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (setq fill-column 54)
         (insert "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Pellentesque porttitor est justo, sed dignissim enim")
         (let ((last-command-event ?\s))
           (insert last-command-event)
           (aggressive-fill-paragraph-post-self-insert-function))
         (list
          (buffer-string)
          (= (char-before) ?\s)
          (current-column)))"##;
    let expect = expect![[
        r#"OK ("Lorem ipsum dolor sit amet, consectetur adipiscing\nelit. Pellentesque porttitor est justo, sed dignissim\nenim " t 5)"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_period_and_custom_trigger_reflow_real_prose() {
    let elisp_form = r##"(let ((text "One two three four five six seven eight nine ten eleven twelve"))
         (list
          (with-temp-buffer
            (text-mode)
            (setq fill-column 24)
            (insert text)
            (let ((last-command-event ?.))
              (insert last-command-event)
              (aggressive-fill-paragraph-post-self-insert-function))
            (buffer-string))
          (with-temp-buffer
            (text-mode)
            (setq fill-column 24)
            (let ((afp-fill-keys (list ?@)))
              (insert text)
              (let ((last-command-event ?@))
                (insert last-command-event)
                (aggressive-fill-paragraph-post-self-insert-function)))
            (buffer-string))))"##;
    let expect = expect![[
        r#"OK ("One two three four five\nsix seven eight nine ten\neleven twelve." "One two three four five\nsix seven eight nine ten\neleven twelve@")"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_non_trigger_and_double_space_paths_preserve_exact_prose() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (text-mode)
           (setq fill-column 12)
           (insert "alpha beta gamma delta")
           (let ((last-command-event ?!))
             (insert last-command-event)
             (aggressive-fill-paragraph-post-self-insert-function))
           (buffer-string))
         (with-temp-buffer
           (text-mode)
           (setq fill-column 12)
           (insert "alpha beta gamma delta ")
           (let ((last-command-event ?\s))
             (insert last-command-event)
             (aggressive-fill-paragraph-post-self-insert-function))
           (buffer-string))
         (with-temp-buffer
           (text-mode)
           (setq fill-column 12)
           (insert "alpha beta\t")
           (let ((last-command-event ?\s))
             (insert last-command-event)
             (aggressive-fill-paragraph-post-self-insert-function))
           (buffer-string)))"##;
    let expect =
        expect![[r#"OK ("alpha beta gamma delta!" "alpha beta gamma delta  " "alpha beta\11 ")"#]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_minor_mode_runs_worker_through_real_buffer_local_hook() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (setq fill-column 30)
         (aggressive-fill-paragraph-mode 1)
         (insert "This paragraph is deliberately long enough to wrap through the installed hook")
         (let ((last-command-event ?\s))
           (insert last-command-event)
           (run-hooks 'post-self-insert-hook))
         (let ((enabled
                (list
                 aggressive-fill-paragraph-mode
                 (buffer-string)
                 (memq
                  #'aggressive-fill-paragraph-post-self-insert-function
                  post-self-insert-hook))))
           (aggressive-fill-paragraph-mode -1)
           (let ((before (buffer-string))
                 (last-command-event ?.))
             (insert last-command-event)
             (run-hooks 'post-self-insert-hook)
             (list
              enabled
              aggressive-fill-paragraph-mode
              (equal (buffer-string) (concat before "."))
              post-self-insert-hook))))"##;
    let expect = expect![[
        r#"OK ((t "This paragraph is deliberately\nlong enough to wrap through\nthe installed hook " (aggressive-fill-paragraph-post-self-insert-function t)) nil t (electric-indent-post-self-insert-function blink-paren-post-self-insert-function))"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_programming_workflow_fills_comment_but_not_code() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (setq fill-column 42)
         (let ((code "(list 'alpha 'beta 'gamma 'delta 'epsilon 'zeta)")
               (comment ";; This practical comment contains enough words to be wrapped with its comment prefix"))
           (insert code "\n" comment)
           (goto-char (point-min))
           (let ((last-command-event ?\s))
             (insert last-command-event)
             (aggressive-fill-paragraph-post-self-insert-function))
           (let ((code-result
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position))))
             (goto-char (point-max))
             (let ((last-command-event ?\s))
               (insert last-command-event)
               (aggressive-fill-paragraph-post-self-insert-function))
             (list code-result (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (" (list 'alpha 'beta 'gamma 'delta 'epsilon 'zeta)" " (list 'alpha 'beta 'gamma 'delta 'epsilon 'zeta)\n;; This practical comment contains enough\n;; words to be wrapped with its comment\n;; prefix ")"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_comment_bullet_workflow_suppresses_reflow() {
    let elisp_form = r##"(with-temp-buffer
         (c++-mode)
         (setq fill-column 32)
         (insert "// * This long bullet remains on one line even though ordinary comments would wrap")
         (let ((before (buffer-string))
               (last-command-event ?\s))
           (insert last-command-event)
           (aggressive-fill-paragraph-post-self-insert-function)
           (list
            before
            (buffer-string)
            (= (count-lines (point-min) (point-max)) 1)
            (afp-bullet-list-in-comments?))))"##;
    let expect = expect![[
        r#"OK ("// * This long bullet remains on one line even though ordinary comments would wrap" "// * This long bullet remains on one line even though ordinary comments would wrap " t 0)"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_markdown_indented_code_workflow_preserves_layout() {
    let elisp_form = r##"(progn
         (unless (fboundp 'markdown-mode)
           (define-derived-mode markdown-mode text-mode "Markdown"))
         (with-temp-buffer
           (markdown-mode)
           (setq fill-column 25)
           (insert "    void example(int value) { return value * 10; }")
           (let ((before (buffer-string))
                 (last-command-event ?\s))
             (insert last-command-event)
             (aggressive-fill-paragraph-post-self-insert-function)
             (list
              before
              (buffer-string)
              (count-lines (point-min) (point-max))
              (afp-markdown-inside-code-block?)))))"##;
    let expect = expect![[
        r#"OK ("    void example(int value) { return value * 10; }" "    void example(int value) { return value * 10; } " 1 0)"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_org_table_edit_workflow_preserves_columns_and_rows() {
    let elisp_form = r##"(with-temp-buffer
         (org-mode)
         (setq fill-column 12)
         (insert "| Name | Value |\n| alpha | 1 |\n| beta | 2 |")
         (goto-char (point-min))
         (search-forward "Name")
         (let ((last-command-event ?\s))
           (insert last-command-event)
           (aggressive-fill-paragraph-post-self-insert-function))
         (insert "and label")
         (list
          (buffer-string)
          (afp-in-org-table?)
          (count-lines (point-min) (point-max))))"##;
    let expect = expect![[r#"OK ("| Name and label | Value |\n| alpha | 1 |\n| beta | 2 |" t 3)"#]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_org_source_header_workflow_preserves_directive_line() {
    let elisp_form = r##"(with-temp-buffer
         (org-mode)
         (setq fill-column 20)
         (insert "#+BEGIN_SRC emacs-lisp")
         (let ((last-command-event ?\s))
           (insert last-command-event)
           (aggressive-fill-paragraph-post-self-insert-function))
         (insert ":results output")
         (list
          (buffer-string)
          (afp-in-org-src-block-header?)
          (= (count-lines (point-min) (point-max)) 1)))"##;
    let expect = expect![[r##"OK ("#+BEGIN_SRC emacs-lisp :results output" t t)"##]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}
