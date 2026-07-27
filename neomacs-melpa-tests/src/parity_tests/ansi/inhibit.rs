use expect_test::expect;

use super::assert_ansi_parity;

#[test]
fn inhibition_removes_sequences_from_every_effect_family_but_still_formats_payloads() {
    let elisp_form = r##"(let ((ansi-inhibit-ansi t))
  (mapcar
   (lambda (symbol)
     (list symbol
           (funcall symbol "%-6s:%03d" "value" 7)))
   '(ansi-black ansi-bright-white
     ansi-on-red ansi-on-bright-blue
     ansi-bold ansi-dark ansi-italic ansi-underscore
     ansi-blink ansi-rapid ansi-contrary ansi-concealed ansi-strike)))"##;
    let expect = expect![[
        r#"OK ((ansi-black "value :007") (ansi-bright-white "value :007") (ansi-on-red "value :007") (ansi-on-bright-blue "value :007") (ansi-bold "value :007") (ansi-dark "value :007") (ansi-italic "value :007") (ansi-underscore "value :007") (ansi-blink "value :007") (ansi-rapid "value :007") (ansi-contrary "value :007") (ansi-concealed "value :007") (ansi-strike "value :007"))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn inhibited_nested_dsl_produces_a_plain_practical_log_line_and_drops_all_cursor_controls() {
    let elisp_form = r##"(let ((ansi-inhibit-ansi t))
  (let ((value
         (with-ansi
          (column 1)
          (kill 2)
          (bold (red "ERROR"))
          ": "
          (on-yellow (black "disk at 99%%%%"))
          (next-line 1)
          (italic "free space: %.1f GiB" 0.2))))
    (list value (string-to-list value) (length value))))"##;
    let expect = expect![[
        r#"OK ("ERROR: disk at 99%free space: 0.2 GiB" (69 82 82 79 82 58 32 100 105 115 107 32 97 116 32 57 57 37 102 114 101 101 32 115 112 97 99 101 58 32 48 46 50 32 71 105 66) 37)"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn dynamic_inhibition_scope_restores_colored_output_after_plain_terminal_rendering() {
    let elisp_form = r##"(let ((ansi-inhibit-ansi nil))
  (let ((before (ansi-green "ready"))
        inside
        after)
    (let ((ansi-inhibit-ansi t))
      (setq inside
            (list
             (ansi-green "ready")
             (ansi-csi-apply 'column 20)
             (with-ansi (bold (cyan "%d%%%%" 50))))))
    (setq after (ansi-green "ready"))
    (list
     before inside after
     ansi-inhibit-ansi
     (equal before after))))"##;
    let expect =
        expect![[r#"OK ("\33[32mready\33[0m" ("ready" "" "50%") "\33[32mready\33[0m" nil t)"#]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn inhibited_princ_writes_only_formatted_plain_text_to_the_output_stream() {
    let elisp_form = r##"(let ((ansi-inhibit-ansi t))
  (with-output-to-string
    (with-ansi-princ
     (up 4)
     (bold "Result: ")
     (green "%d passed" 128)
     ", "
     (red "%d failed" 0)
     "\n")))"##;
    let expect = expect![[r#"OK "Result: 128 passed, 0 failed\n""#]];
    assert_ansi_parity(elisp_form, expect);
}
