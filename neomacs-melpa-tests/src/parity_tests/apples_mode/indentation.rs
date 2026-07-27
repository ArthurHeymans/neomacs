use expect_test::expect;

use super::assert_apples_mode_parity;

#[test]
fn nested_tell_if_repeat_and_handler_script_indents_as_a_real_program() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "on summarize(items)\n"
                 "set total to 0\n"
                 "repeat with itemValue in items\n"
                 "if itemValue is greater than 0 then\n"
                 "tell application \"Finder\"\n"
                 "set total to total + itemValue\n"
                 "end tell\n"
                 "else\n"
                 "set total to total - 1\n"
                 "end if\n"
                 "end repeat\n"
                 "return total\n"
                 "end summarize\n")
                (indent-region (point-min) (point-max))
                (buffer-string))"##;
    let expect = expect![[
        r#"OK "on summarize(items)\n    set total to 0\n    repeat with itemValue in items\n\11if itemValue is greater than 0 then\n\11    tell application \"Finder\"\n\11\11set total to total + itemValue\n\11    end tell\n\11else\n\11    set total to total - 1\n\11end if\n    end repeat\n    return total\nend summarize\n""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn inline_if_and_tell_to_forms_do_not_indent_the_following_statement() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "if ready then set status to \"done\"\n"
                 "set nextValue to 1\n"
                 "tell application \"Finder\" to activate\n"
                 "set finalValue to 2\n")
                (indent-region (point-min) (point-max))
                (buffer-string))"##;
    let expect = expect![[
        r#"OK "if ready then set status to \"done\"\nset nextValue to 1\ntell application \"Finder\" to activate\nset finalValue to 2\n""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn continuation_lines_indent_once_remain_aligned_and_deindent_after_the_chain() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "set messageText to \"first\" & ¬\n"
                 "\"second\" & ¬\n"
                 "\"third\"\n"
                 "set completed to true\n"
                 "tell application \"Finder\"\n"
                 "set namesList to name of every file & ¬\n"
                 "name of every folder\n"
                 "end tell\n")
                (indent-region (point-min) (point-max))
                (buffer-string))"##;
    let expect = expect![[
        r#"OK "set messageText to \"first\" & ¬\n    \"second\" & ¬\n    \"third\"\nset completed to true\ntell application \"Finder\"\n    set namesList to name of every file & ¬\n\11name of every folder\nend tell\n""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn multiline_comments_strings_and_blank_lines_do_not_corrupt_surrounding_indentation() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "tell application \"Finder\"\n"
                 "(* comment begins\n"
                 "if this were code then\n"
                 "end comment *)\n"
                 "\n"
                 "set prose to \"tell application\n"
                 "end tell\"\n"
                 "set answer to 42\n"
                 "end tell\n")
                (indent-region (point-min) (point-max))
                (buffer-string))"##;
    let expect = expect![[
        r#"OK "tell application \"Finder\"\n    (* comment begins\n    if this were code then\n    end comment *)\n\n    set prose to \"tell application\nend tell\"\n    set answer to 42\nend tell\n""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn on_error_inside_try_aligns_with_try_body_then_end_try_returns_to_column_zero() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "try\n"
                 "set payload to do shell script \"printf ok\"\n"
                 "on error messageText number errorNumber\n"
                 "display dialog messageText\n"
                 "return errorNumber\n"
                 "end try\n"
                 "return payload\n")
                (indent-region (point-min) (point-max))
                (buffer-string))"##;
    let expect = expect![[
        r#"OK "try\n    set payload to do shell script \"printf ok\"\non error messageText number errorNumber\n    display dialog messageText\n    return errorNumber\nend try\nreturn payload\n""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn parse_lines_reports_the_context_used_for_continuation_and_block_indentation() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert
                 "tell application \"Finder\"\n"
                 "    set xs to {1, 2} & ¬\n"
                 "        {3, 4} & ¬\n"
                 "        {5, 6}\n"
                 "    return xs\n"
                 "end tell\n")
                (goto-char (point-min))
                (forward-line 2)
                (move-to-column 12)
                (let ((continuation (multiple-value-list (apples-parse-lines))))
                  (forward-line 2)
                  (let ((return-line (multiple-value-list (apples-parse-lines))))
                    (list continuation return-line))))"##;
    let expect = expect![[
        r#"OK ((12 8 nil 27 4 "set" "set xs to {1, 2} & ¬" 19 nil) (0 4 "return" 71 8 nil "{5, 6}" nil 9))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn toggle_indent_cycles_relative_to_previous_code_and_continuation_offsets() {
    let elisp_form = r##"(with-temp-buffer
                (setq apples-plist (list :AS-version "2.1" :tmp-files nil))
                (apples-mode)
                (insert "tell application \"Finder\"\nset x to 1\nset y to 2\n")
                (goto-char (point-min))
                (forward-line 1)
                (apples-toggle-indent)
                (let ((first (current-indentation)))
                  (apples-toggle-indent)
                  (let ((second (current-indentation)))
                    (apples-toggle-indent)
                    (list first second (current-indentation)
                          (buffer-string)))))"##;
    let expect =
        expect![[r#"OK (4 0 4 "tell application \"Finder\"\n    set x to 1\nset y to 2\n")"#]];
    assert_apples_mode_parity(elisp_form, expect);
}
