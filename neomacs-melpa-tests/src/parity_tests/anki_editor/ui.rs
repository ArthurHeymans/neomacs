use expect_test::expect;

use super::assert_anki_editor_ui_parity;

#[test]
fn ui_source_loads_transient_commands_defaults_and_complete_helper_surface() {
    let elisp_form = r##"(list
                      (featurep 'anki-editor)
                      (featurep 'anki-editor-ui)
                      anki-editor-ui-match-preprompt
                      anki-editor-ui-deck-preprompt
                      (mapcar
                       (lambda (function)
                         (list
                          function
                          (fboundp function)
                          (help-function-arglist
                           function t)
                          (commandp function)))
                       '(anki-editor-ui
                         anki-editor-ui-push
                         anki-editor-ui--read-note-type
                         anki-editor-ui--read-decks
                         anki-editor-ui--read-match
                         anki-editor-ui-push-region
                         anki-editor-ui-push-subtree
                         anki-editor-ui-push-narrowed-buffer
                         anki-editor-ui-push-full-buffer
                         anki-editor-ui-push-agenda-files
                         anki-editor-ui-push--pass-args-and-push
                         anki-editor-ui-push--parse-args
                         anki-editor-ui--skip-unless-decks)))"##;
    let expect = expect![[
        r#"OK (t t "Syntax: [+-&|][tag|{tagregex}|property[=|<>|<|>|<=|>=]value]\n" "Use 'TAB' to complete, ',' to select multiple, and 'RET' to finalize.\n" ((anki-editor-ui t nil t) (anki-editor-ui-push t nil t) (anki-editor-ui--read-note-type t (prompt initial-input history) nil) (anki-editor-ui--read-decks t (prompt initial-input history) nil) (anki-editor-ui--read-match t (prompt initial-input history) nil) (anki-editor-ui-push-region t (&optional args) t) (anki-editor-ui-push-subtree t (&optional args) t) (anki-editor-ui-push-narrowed-buffer t (&optional args) t) (anki-editor-ui-push-full-buffer t (&optional args) t) (anki-editor-ui-push-agenda-files t (&optional args) t) (anki-editor-ui-push--pass-args-and-push t (args scope) nil) (anki-editor-ui-push--parse-args t (args) nil) (anki-editor-ui--skip-unless-decks t (&rest filter-decks) nil)))"#
    ]];
    assert_anki_editor_ui_parity(elisp_form, expect);
}

#[test]
fn transient_argument_parser_builds_exact_combined_matches_and_selected_decks() {
    let elisp_form = r##"(list
                      (anki-editor-ui-push--parse-args
                       '("new"
                         "note-type=Basic"
                         ("decks="
                          "Study"
                          "Archive")
                         "match=tag:due"))
                      (anki-editor-ui-push--parse-args
                       '("failed"
                         "existing"
                         "match=+priority=\"high\""))
                      (anki-editor-ui-push--parse-args
                       '("match=-suspended"))
                      (anki-editor-ui-push--parse-args
                       nil))"##;
    let expect = expect![[
        r#"OK (((fullmatch . "+ANKI_NOTE_ID=\"\"+ANKI_NOTE_TYPE=\"Basic\"+tag:due") (decks "Study" "Archive")) ((fullmatch . "+ANKI_FAILURE_REASON<>\"\"+ANKI_NOTE_ID<>\"\"+priority=\"high\"") (decks)) ((fullmatch . "-suspended") (decks)) ((fullmatch . "") (decks)))"#
    ]];
    assert_anki_editor_ui_parity(elisp_form, expect);
}

#[test]
fn ui_scope_wrappers_and_deck_filter_pass_exact_org_mapping_arguments() {
    let elisp_form = r##"(let (pushes)
                      (cl-letf
                          (((symbol-function
                             'anki-editor-push-notes)
                            (lambda (&rest arguments)
                              (push arguments pushes)
                              arguments)))
                        (let ((args
                               '("new"
                                 ("decks="
                                  "Study"
                                  "Archive")
                                 "match=tag:due")))
                          (anki-editor-ui-push-region
                           args)
                          (anki-editor-ui-push-subtree
                           args)
                          (anki-editor-ui-push-narrowed-buffer
                           args)
                          (anki-editor-ui-push-full-buffer
                           args)
                          (anki-editor-ui-push-agenda-files
                           args)))
                      (list
                       (nreverse pushes)
                       (with-temp-buffer
                         (org-mode)
                         (insert
                          "* Selected\n:PROPERTIES:\n:ANKI_DECK: Study\n:END:\nBody\n")
                         (goto-char
                          (point-min))
                         (anki-editor-ui--skip-unless-decks
                          "Study"
                          "Archive"))
                       (with-temp-buffer
                         (org-mode)
                         (insert
                          "* Skipped\n:PROPERTIES:\n:ANKI_DECK: Other\n:END:\nBody\n** Child\nNested\n")
                         (goto-char
                          (point-min))
                         (let ((result
                                (anki-editor-ui--skip-unless-decks
                                 "Study"
                                 "Archive")))
                           (list
                            result
                            (point)
                            (buffer-size))))))"##;
    let expect = expect![[
        r#"OK (((region "+ANKI_NOTE_ID=\"\"+tag:due" (apply #1=#'anki-editor-ui--skip-unless-decks '#2=("Study" "Archive"))) (tree "+ANKI_NOTE_ID=\"\"+tag:due" (apply #1# '#2#)) (nil "+ANKI_NOTE_ID=\"\"+tag:due" (apply #1# '#2#)) (file "+ANKI_NOTE_ID=\"\"+tag:due" (apply #1# '#2#)) (agenda "+ANKI_NOTE_ID=\"\"+tag:due" (apply #1# '#2#))) nil (68 1 68))"#
    ]];
    assert_anki_editor_ui_parity(elisp_form, expect);
}
