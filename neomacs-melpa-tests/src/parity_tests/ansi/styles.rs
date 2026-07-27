use expect_test::expect;

use super::{assert_ansi_parity, assert_ansi_signal_parity};

#[test]
fn every_style_function_emits_the_exact_sgr_code_and_independent_reset() {
    let elisp_form = r##"(mapcar
 (lambda (entry)
   (let* ((symbol (car entry))
          (code (cdr entry))
          (value (funcall symbol "sample")))
     (list symbol code value (string-to-list value))))
 '((ansi-bold . 1)
   (ansi-dark . 2)
   (ansi-italic . 3)
   (ansi-underscore . 4)
   (ansi-blink . 5)
   (ansi-rapid . 6)
   (ansi-contrary . 7)
   (ansi-concealed . 8)
   (ansi-strike . 9)))"##;
    let expect = expect![[
        r#"OK ((ansi-bold 1 "\33[1msample\33[0m" (27 91 49 109 115 97 109 112 108 101 27 91 48 109)) (ansi-dark 2 "\33[2msample\33[0m" (27 91 50 109 115 97 109 112 108 101 27 91 48 109)) (ansi-italic 3 "\33[3msample\33[0m" (27 91 51 109 115 97 109 112 108 101 27 91 48 109)) (ansi-underscore 4 "\33[4msample\33[0m" (27 91 52 109 115 97 109 112 108 101 27 91 48 109)) (ansi-blink 5 "\33[5msample\33[0m" (27 91 53 109 115 97 109 112 108 101 27 91 48 109)) (ansi-rapid 6 "\33[6msample\33[0m" (27 91 54 109 115 97 109 112 108 101 27 91 48 109)) (ansi-contrary 7 "\33[7msample\33[0m" (27 91 55 109 115 97 109 112 108 101 27 91 48 109)) (ansi-concealed 8 "\33[8msample\33[0m" (27 91 56 109 115 97 109 112 108 101 27 91 48 109)) (ansi-strike 9 "\33[9msample\33[0m" (27 91 57 109 115 97 109 112 108 101 27 91 48 109)))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn deeply_nested_style_color_and_background_calls_preserve_open_and_reset_order() {
    let elisp_form = r##"(let ((direct
       (ansi-bold
        (ansi-underscore
         (ansi-bright-yellow
          (ansi-on-blue
           (ansi-italic "critical path"))))))
      (dsl
       (with-ansi
        (bold
         (underscore
          (bright-yellow
           (on-blue
            (italic "critical path"))))))))
  (list
   direct
   dsl
   (equal direct dsl)
   (string-to-list direct)))"##;
    let expect = expect![[
        r#"OK ("\33[1m\33[4m\33[93m\33[44m\33[3mcritical path\33[0m\33[0m\33[0m\33[0m\33[0m" "\33[1m\33[4m\33[93m\33[44m\33[3mcritical path\33[0m\33[0m\33[0m\33[0m\33[0m" t (27 91 49 109 27 91 52 109 27 91 57 51 109 27 91 52 52 109 27 91 51 109 99 114 105 116 105 99 97 108 32 112 97 116 104 27 91 48 109 27 91 48 109 27 91 48 109 27 91 48 109 27 91 48 109))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn styles_format_practical_diagnostic_values_before_wrapping_the_complete_text() {
    let elisp_form = r##"(list
 (ansi-bold "%s:%d:%d: %s" "src/main.rs" 41 9 "type mismatch")
 (ansi-underscore "https://example.test/jobs/%06d" 73)
 (ansi-strike "deprecated=%S replacement=%S" 'old-api 'new-api)
 (ansi-italic "Δ %.3f ± %.3f" 0.125 0.005)
 (ansi-concealed "token=%s" "secret-value"))"##;
    let expect = expect![[
        r#"OK ("\33[1msrc/main.rs:41:9: type mismatch\33[0m" "\33[4mhttps://example.test/jobs/000073\33[0m" "\33[9mdeprecated=old-api replacement=new-api\33[0m" "\33[3mΔ 0.125 ± 0.005\33[0m" "\33[8mtoken=secret-value\33[0m")"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn empty_and_multiline_payloads_are_still_wrapped_with_one_open_and_one_reset_sequence() {
    let elisp_form = r##"(mapcar
 (lambda (value)
   (list value
         (ansi-bold "%s" value)
         (length (ansi-bold "%s" value))))
 '("" "\n" "first\nsecond\n" "λ\n漢字"))"##;
    let expect = expect![[
        r#"OK (("" "\33[1m\33[0m" 8) ("\n" "\33[1m\n\33[0m" 9) ("first\nsecond\n" "\33[1mfirst\nsecond\n\33[0m" 21) ("λ\n漢字" "\33[1mλ\n漢字\33[0m" 12))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn nesting_a_rendered_bare_percent_preserves_the_outer_native_format_signal() {
    let elisp_form = r##"(ansi-bold (ansi-cyan "%d%%" 50))"##;
    let expect = expect![[r#"ERR (error "Not enough arguments for format string")"#]];
    assert_ansi_signal_parity(elisp_form, expect);
}
