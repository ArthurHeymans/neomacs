use expect_test::expect;

use super::assert_anki_editor_view_parity;

#[test]
fn anki_editor_view_ripgrep_builds_exact_quoted_multi_directory_command() {
    let elisp_form = r##"(let (command)
         (cl-letf
             (((symbol-function
                'shell-command-to-string)
               (lambda (value)
                 (setq command value)
                 "")))
           (list
            (anki-editor-view--ripgrep-find-locations
             ":ANKI_NOTE_ID: 42"
             '("/notes/main"
               "/notes/with space"
               "/notes/with\"quote"))
            command)))"##;
    let expect = expect![[
        r#"OK (nil "rg -n -e \":ANKI_NOTE_ID: 42\" --no-heading  \"/notes/main\" \"/notes/with space\" \"/notes/with\\\"quote\"")"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_ripgrep_parses_multiple_realistic_matches_in_order() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'shell-command-to-string)
           (lambda (_command)
             (concat
              "/notes/decks.org:17::ANKI_NOTE_ID: 42\n"
              "/notes/archive.org:203:  :ANKI_NOTE_ID: 42\n"))))
         (anki-editor-view--ripgrep-find-locations
          ":ANKI_NOTE_ID: 42"
          '("/notes")))"##;
    let expect = expect![[
        r#"OK (((file . "/notes/decks.org") (line . 17)) ((file . "/notes/archive.org") (line . 203)))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_ripgrep_crlf_blank_line_exposes_malformed_result_error() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'shell-command-to-string)
           (lambda (_command)
             (concat
              "./deck.org:9:match\r\n"
              "./other.org:10:match\r\n"
              "\r\n"))))
         (condition-case error
             (list
              'value
              (anki-editor-view--ripgrep-find-locations
               "needle"
               '(".")))
           (error
            (list
             'error
             (car error)
             (cdr error)))))"##;
    let expect = expect![[r#"OK (error error ("Ripgrep result is malformed"))"#]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_ripgrep_empty_output_returns_no_locations() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'shell-command-to-string)
           (lambda (_command)
             "")))
         (list
          (anki-editor-view--ripgrep-find-locations
           "missing" '("/notes"))
          (anki-editor-view--ripgrep-find-locations
           "missing" nil)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_ripgrep_malformed_result_signals_exact_error() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'shell-command-to-string)
           (lambda (_command)
             (concat
              "/notes/good.org:2:match\n"
              "this result has no line number\n"))))
         (condition-case error
             (list
              'value
              (anki-editor-view--ripgrep-find-locations
               "needle" '("/notes")))
           (error
            (list
             'error
             (car error)
             (cdr error)))))"##;
    let expect = expect![[r#"OK (error error ("Ripgrep result is malformed"))"#]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_ripgrep_colon_in_filename_exposes_first_segment_parser() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'shell-command-to-string)
           (lambda (_command)
             (concat
              "/notes/topic:part.org:27:match\n"
              "C:/notes/windows.org:31:match\n"))))
         (anki-editor-view--ripgrep-find-locations
          "needle" '("/notes")))"##;
    let expect = expect![[
        r#"OK (((file . "part.org") (line . 27)) ((file . "/notes/windows.org") (line . 31)))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}

#[test]
fn anki_editor_view_ripgrep_preserves_relative_unicode_and_dash_paths() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'shell-command-to-string)
           (lambda (_command)
             (concat
              "./資料/記憶.org:108:match\n"
              "../-archive/deck.org:7:match\n"))))
         (anki-editor-view--ripgrep-find-locations
          "Ω" '(".")))"##;
    let expect = expect![[
        r#"OK (((file . "./資料/記憶.org") (line . 108)) ((file . "../-archive/deck.org") (line . 7)))"#
    ]];
    assert_anki_editor_view_parity(elisp_form, expect);
}
