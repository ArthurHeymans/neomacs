use expect_test::expect;

use super::assert_applescript_mode_parity;

#[test]
fn applescript_mode_font_lock_marks_control_commands_handlers_and_pseudo_keywords_in_real_code() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (insert
          "on greet(person)\n"
          "    if person is not false then\n"
          "        set messageText to \"hello\"\n"
          "        display dialog messageText\n"
          "    else\n"
          "        beep\n"
          "    end if\n"
          "end greet\n")
         (font-lock-ensure)
         (mapcar
          (lambda (token)
            (list
             token
             (applescript-test-face-at
              token)))
          '("on"
            "greet"
            "if"
            "is"
            "not"
            "false"
            "then"
            "set"
            "display dialog"
            "else"
            "beep"
            "end")))"##;
    let expect = expect![[
        r#"OK (("on" font-lock-keyword-face) ("greet" font-lock-function-name-face) ("if" font-lock-keyword-face) ("is" font-lock-keyword-face) ("not" font-lock-keyword-face) ("false" as-pseudo-keyword-face) ("then" font-lock-keyword-face) ("set" font-lock-keyword-face) ("display dialog" nil) ("else" font-lock-keyword-face) ("beep" font-lock-keyword-face) ("end" font-lock-keyword-face))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_font_lock_handles_multword_keywords_commands_and_misc_arguments() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (insert
          "using terms from application \"Finder\"\n"
          "with timeout of 10 seconds\n"
          "    choose file with prompt \"Pick\" with invisibles\n"
          "    display dialog \"Proceed?\" buttons {\"No\", \"Yes\"} default button \"Yes\"\n"
          "end timeout\n"
          "end using terms from\n")
         (font-lock-ensure)
         (mapcar
          (lambda (token)
            (list
             token
             (applescript-test-face-at
              token)))
          '("using terms from"
            "application"
            "with timeout"
            "choose file"
            "display dialog"
            "buttons"
            "default button"
            "end")))"##;
    let expect = expect![[
        r#"OK (("using terms from" nil) ("application" font-lock-keyword-face) ("with timeout" nil) ("choose file" nil) ("display dialog" nil) ("buttons" font-lock-keyword-face) ("default button" nil) ("end" font-lock-keyword-face))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_font_lock_does_not_treat_keyword_text_inside_strings_or_comments_as_code() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (insert
          "set literalText to \"tell application and display dialog\"\n"
          "-- repeat and beep should remain comment text\n"
          "(* if true then set hidden to false *)\n"
          "tell application \"Finder\"\n"
          "    beep\n"
          "end tell\n")
         (font-lock-ensure)
         (list
          (applescript-test-face-at
           "tell"
           1)
          (applescript-test-face-at
           "display dialog"
           1)
          (applescript-test-face-at
           "repeat")
          (applescript-test-face-at
           "true")
          (applescript-test-face-at
           "tell"
           2)
          (applescript-test-face-at
           "beep"
           2)
          (applescript-test-face-at
           "end")))"##;
    let expect = expect![
        "OK (font-lock-string-face font-lock-string-face font-lock-comment-face font-lock-comment-face font-lock-keyword-face font-lock-keyword-face font-lock-keyword-face)"
    ];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_font_lock_case_behavior_is_exposed_on_mixed_case_practical_source() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (insert
          "ON UpperHandler()\n"
          "    IF TRUE THEN\n"
          "        DISPLAY DIALOG \"LOUD\"\n"
          "    end if\n"
          "end UpperHandler\n")
         (font-lock-ensure)
         (mapcar
          (lambda (token)
            (list
             token
             (applescript-test-face-at
              token)))
          '("ON"
            "UpperHandler"
            "IF"
            "TRUE"
            "DISPLAY DIALOG"
            "end")))"##;
    let expect = expect![[
        r#"OK (("ON" nil) ("UpperHandler" nil) ("IF" nil) ("TRUE" nil) ("DISPLAY DIALOG" nil) ("end" font-lock-keyword-face))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_refontifies_changed_lines_and_removes_stale_keyword_faces() {
    let elisp_form = r##"(with-temp-buffer
         (applescript-mode)
         (insert
          "set answer to false\n"
          "return answer\n")
         (font-lock-ensure)
         (let ((before
                (list
                 (applescript-test-face-at
                  "set")
                 (applescript-test-face-at
                  "false")
                 (applescript-test-face-at
                  "return"))))
           (goto-char
            (point-min))
           (delete-region
            (line-beginning-position)
            (line-end-position))
           (insert
            "ordinaryName equals anotherName")
           (font-lock-flush)
           (font-lock-ensure)
           (list
            before
            (buffer-string)
            (applescript-test-face-at
             "ordinaryName")
            (applescript-test-face-at
             "return"))))"##;
    let expect = expect![[
        r#"OK ((font-lock-keyword-face font-lock-function-name-face nil) "ordinaryName equals anotherName\nreturn answer\n" nil nil)"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_font_lock_hook_initializes_custom_faces_from_keyword_face() {
    let elisp_form = r##"(let ((before
                (mapcar
                 (lambda (face)
                   (list
                    face
                    (face-attribute
                     face
                     :inherit
                     nil
                     'default)))
                 '(as-pseudo-keyword-face
                   as-command-face))))
         (set-face-attribute
          'as-pseudo-keyword-face
          nil
          :inherit
          'default)
         (set-face-attribute
          'as-command-face
          nil
          :inherit
          'default)
         (as-font-lock-mode-hook)
         (list
          before
          (mapcar
           (lambda (face)
             (list
              face
              (face-attribute
               face
               :inherit
               nil
               'default)
              (face-differs-from-default-p
               face)))
           '(as-pseudo-keyword-face
             as-command-face))
          (face-attribute
           'font-lock-keyword-face
           :inherit
           nil
           'default)))"##;
    let expect = expect![
        "OK (((as-pseudo-keyword-face nil) (as-command-face nil)) ((as-pseudo-keyword-face nil nil) (as-command-face nil nil)) nil)"
    ];

    assert_applescript_mode_parity(elisp_form, expect);
}
