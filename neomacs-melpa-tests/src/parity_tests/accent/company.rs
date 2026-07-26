use expect_test::expect;

use super::assert_accent_parity;

#[test]
fn accent_company_interactive_command_starts_its_backend_for_direct_and_interactive_calls() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'company-begin-backend)
                     (lambda (backend)
                       (push
                        backend
                        calls)
                       'started)))
                 (with-temp-buffer
                   (insert
                    "a")
                   (list
                    (accent-company
                     'interactive)
                    (call-interactively
                     #'accent-company)
                    (nreverse
                     calls)
                    (interactive-form
                     'accent-company)))))"##;
    let expect = expect![
        "OK (started started (accent-company accent-company) (interactive (list 'interactive)))"
    ];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_prefix_matches_supported_letters_in_before_and_after_positions() {
    let elisp_form = r##"(list
               (with-temp-buffer
                 (insert
                  "cat")
                 (goto-char
                  3)
                 (let ((accent-position
                        'before))
                   (accent-company
                    'prefix)))
               (with-temp-buffer
                 (insert
                  "cat")
                 (goto-char
                  2)
                 (let ((accent-position
                        'after))
                   (accent-company
                    'prefix)))
               (with-temp-buffer
                 (insert
                  "x")
                 (let ((accent-position
                        'before))
                   (accent-company
                    'prefix)))
               (with-temp-buffer
                 (insert
                  "A")
                 (let ((accent-position
                        'before))
                   (accent-company
                    'prefix))))"##;
    let expect = expect![[r#"OK ("a" "a" nil "A")"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_before_candidates_return_exact_strings_in_diacritic_order() {
    let elisp_form = r##"(with-temp-buffer
              (insert
               "a")
              (let ((accent-position
                     'before))
                (list
                 (accent-company
                  'candidates)
                 (accent-company
                  'candidates
                  'ignored
                  17)
                 (buffer-string)
                 (point))))"##;
    let expect = expect![[
        r#"OK (("à" "á" "â" "ä" "æ" "ã" "å" "ā") ("à" "á" "â" "ä" "æ" "ã" "å" "ā") "a" 2)"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_after_candidates_prefix_each_option_with_the_character_before_point() {
    let elisp_form = r##"(with-temp-buffer
              (insert
               "xa")
              (goto-char
               2)
              (let ((accent-position
                     'after))
                (list
                 (accent-company
                  'candidates)
                 (buffer-string)
                 (point))))"##;
    let expect = expect![[r#"OK (("xà" "xá" "xâ" "xä" "xæ" "xã" "xå" "xā") "xa" 2)"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_candidates_include_custom_entries_and_preserve_duplicates() {
    let elisp_form = r##"(with-temp-buffer
              (insert
               "a")
              (let ((accent-position
                     'before)
                    (accent-custom
                     '((a
                        (ă
                         á)))))
                (accent-company
                 'candidates)))"##;
    let expect = expect![[r#"OK ("à" "á" "â" "ä" "æ" "ã" "å" "ā" "ă" "á")"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_post_completion_deletes_only_the_after_position_character() {
    let elisp_form = r##"(list
               (with-temp-buffer
                 (insert
                  "cat")
                 (goto-char
                  2)
                 (let ((accent-position
                        'after))
                   (list
                    (accent-company
                     'post-completion)
                    (buffer-string)
                    (point))))
               (with-temp-buffer
                 (insert
                  "cat")
                 (goto-char
                  3)
                 (let ((accent-position
                        'before))
                   (list
                    (accent-company
                     'post-completion)
                    (buffer-string)
                    (point)))))"##;
    let expect = expect![[r#"OK ((nil "ct" 2) (nil "cat" 3))"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_after_post_completion_deletes_even_an_unsupported_character() {
    let elisp_form = r##"(with-temp-buffer
              (insert
               "x")
              (goto-char
               1)
              (let ((accent-position
                     'after))
                (list
                 (accent-company
                  'post-completion)
                 (buffer-string)
                 (point))))"##;
    let expect = expect![[r#"OK (nil "" 1)"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_after_candidates_require_a_character_before_point_for_prefixing() {
    let elisp_form = r##"(with-temp-buffer
              (insert
               "a")
              (goto-char
               (point-min))
              (let ((accent-position
                     'after))
                (condition-case error
                    (accent-company
                     'candidates)
                  (error
                   (list
                    error
                    (buffer-string)
                    (point))))))"##;
    let expect = expect!["OK ((wrong-type-argument characterp nil) \"a\" 1)"];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_unsupported_letters_and_unknown_commands_return_nil_without_side_effects() {
    let elisp_form = r##"(with-temp-buffer
              (insert
               "x")
              (let ((accent-position
                     'before))
                (list
                 (accent-company
                  'prefix)
                 (accent-company
                  'candidates)
                 (accent-company
                  'unknown-command)
                 (buffer-string)
                 (point))))"##;
    let expect = expect![[r#"OK (nil nil nil "x" 2)"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_all_commands_read_the_selected_character_before_dispatch() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (command)
                  (condition-case error
                      (with-temp-buffer
                        (let ((accent-position
                               'before))
                          (accent-company
                           command)))
                    (error
                     error)))
                '(interactive
                  prefix
                  candidates
                  post-completion
                  unknown))
               (mapcar
                (lambda (command)
                  (condition-case error
                      (with-temp-buffer
                        (insert
                         "a")
                        (goto-char
                         (point-max))
                        (let ((accent-position
                               'after))
                          (accent-company
                           command)))
                    (error
                     error)))
                '(interactive
                  prefix
                  candidates
                  post-completion
                  unknown)))"##;
    let expect = expect![
        "OK (((wrong-type-argument characterp nil) (wrong-type-argument characterp nil) (wrong-type-argument characterp nil) (wrong-type-argument characterp nil) (wrong-type-argument characterp nil)) ((wrong-type-argument characterp nil) (wrong-type-argument characterp nil) (wrong-type-argument characterp nil) (wrong-type-argument characterp nil) (wrong-type-argument characterp nil)))"
    ];

    assert_accent_parity(elisp_form, expect);
}
