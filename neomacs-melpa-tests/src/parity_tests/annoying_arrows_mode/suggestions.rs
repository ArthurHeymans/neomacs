use expect_test::expect;

use super::assert_annoying_arrows_mode_parity;

#[test]
fn annoying_arrows_add_suggestion_prepends_and_deduplicates() {
    let elisp_form = r##"(let ((symbol 'annoying-arrows-test-command))
         (put symbol 'annoying-arrows--alts nil)
         (aa-add-suggestion symbol 'forward-word)
         (aa-add-suggestion symbol 'ace-jump-mode)
         (aa-add-suggestion symbol 'forward-word)
         (list (get symbol 'annoying-arrows--alts)
               (aa-add-suggestion symbol 'ace-jump-mode)
               (get symbol 'annoying-arrows--alts)))"##;
    let expect = expect!["OK (#1=(ace-jump-mode forward-word) nil #1#)"];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_add_suggestions_preserves_existing_and_input_order() {
    let elisp_form = r##"(let ((symbol 'annoying-arrows-test-command))
         (put symbol 'annoying-arrows--alts '(existing-a existing-b))
         (aa-add-suggestions
          symbol
          '(new-a existing-b new-b new-a existing-a new-c))
         (list (get symbol 'annoying-arrows--alts)
               (aa-add-suggestions symbol '(new-b existing-a))
               (get symbol 'annoying-arrows--alts)))"##;
    let expect = expect!["OK (#1=(new-a new-b new-a new-c existing-a existing-b) #1# #1#)"];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_shortcut_filter_keeps_only_bound_commands() {
    let elisp_form = r##"(let ((map (make-sparse-keymap)))
         (define-key map (kbd "C-c f") #'forward-word)
         (define-key map (kbd "M-g g") #'goto-line)
         (use-local-map map)
         (list
          (annoying-arrows--commands-with-shortcuts
           '(forward-word goto-line beginning-of-buffer
             annoying-arrows-definitely-unbound))
          (mapcar
           (lambda (command)
             (list command
                   (substitute-command-keys
                    (format "\\[%S]" command))))
           '(forward-word goto-line beginning-of-buffer
             annoying-arrows-definitely-unbound))))"##;
    let expect = expect![[
        r#"OK ((forward-word goto-line beginning-of-buffer) ((forward-word #("C-c f" 0 5 (font-lock-face help-key-binding face help-key-binding))) (goto-line #("M-g g" 0 5 (font-lock-face help-key-binding face help-key-binding))) (beginning-of-buffer #("M-<" 0 3 (font-lock-face help-key-binding face help-key-binding))) (annoying-arrows-definitely-unbound #("M-x annoying-arrows-definitely-unbound" 0 38 (font-lock-face help-key-binding face help-key-binding)))))"#
    ]];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_shortcut_filter_handles_empty_and_all_unbound_lists() {
    let elisp_form = r##"(list
         (annoying-arrows--commands-with-shortcuts nil)
         (annoying-arrows--commands-with-shortcuts
          '(annoying-arrows-unbound-a annoying-arrows-unbound-b)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_custom_advice_macro_registers_command_property_and_advice() {
    let elisp_form = r##"(progn
         (defun annoying-arrows-fixture-command () 'ran)
         (add-annoying-arrows-advice
          annoying-arrows-fixture-command
          '(forward-word beginning-of-line))
         (list
          (memq 'annoying-arrows-fixture-command annoying-arrows--commands)
          (get 'annoying-arrows-fixture-command 'annoying-arrows--alts)
          (not
           (null
            (ad-find-advice
             'annoying-arrows-fixture-command
             'before
             'annoying-arrows)))
          (annoying-arrows-fixture-command)))"##;
    let expect = expect![
        "OK ((annoying-arrows-fixture-command backward-delete-char backward-delete-char-untabify backward-char forward-char left-char right-char next-line previous-line) (forward-word beginning-of-line) t ran)"
    ];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}
