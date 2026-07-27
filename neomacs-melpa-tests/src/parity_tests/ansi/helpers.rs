use expect_test::expect;

use super::{assert_ansi_parity, assert_ansi_signal_parity};

#[test]
fn concat_keeps_string_order_and_properties_while_ignoring_every_non_string_value() {
    let elisp_form = r##"(let* ((first (propertize "red" 'face 'error 'source 'left))
       (second (propertize "green" 'face 'success 'source 'right))
       (value
        (ansi--concat
         first nil 0 'symbol '(a list) ["vector"] second "")))
  (list
   value
   (length value)
   (text-properties-at 0 value)
   (text-properties-at 3 value)
   (text-properties-at 5 value)
   (substring-no-properties value)))"##;
    let expect = expect![[
        r#"OK (#("redgreen" 0 3 (source left face error) 3 8 (source right face success)) 8 (source left face error) #1=(source right face success) #1# "redgreen")"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn code_lookup_covers_every_public_effect_registry_and_returns_nil_for_non_effects() {
    let elisp_form = r##"(mapcar
 (lambda (effect)
   (cons effect (ansi--code effect)))
 (append
  (mapcar #'car ansi-colors)
  (mapcar #'car ansi-bright-colors)
  (mapcar #'car ansi-on-colors)
  (mapcar #'car ansi-on-bright-colors)
  (mapcar #'car ansi-styles)
  '(up column missing nil 31)))"##;
    let expect = expect![
        "OK ((black . 30) (red . 31) (green . 32) (yellow . 33) (blue . 34) (magenta . 35) (cyan . 36) (white . 37) (bright-black . 90) (bright-red . 91) (bright-green . 92) (bright-yellow . 93) (bright-blue . 94) (bright-magenta . 95) (bright-cyan . 96) (bright-white . 97) (on-black . 40) (on-red . 41) (on-green . 42) (on-yellow . 43) (on-blue . 44) (on-magenta . 45) (on-cyan . 46) (on-white . 47) (on-bright-black . 100) (on-bright-red . 101) (on-bright-green . 102) (on-bright-yellow . 103) (on-bright-blue . 104) (on-bright-magenta . 105) (on-bright-cyan . 106) (on-bright-white . 107) (bold . 1) (dark . 2) (italic . 3) (underscore . 4) (blink . 5) (rapid . 6) (contrary . 7) (concealed . 8) (strike . 9) (up) (column) (missing) (nil) (31))"
    ];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn alias_and_character_lookup_distinguish_effects_csi_names_and_unknown_values() {
    let elisp_form = r##"(mapcar
 (lambda (value)
   (list
    value
    (ansi--is-alias value)
    (ansi--char value)))
 '(black bright-red on-blue on-bright-white bold strike
   up down forward backward next-line previous-line column kill
   missing nil "up" 31))"##;
    let expect = expect![[
        r#"OK ((black black nil) (bright-red bright-red nil) (on-blue on-blue nil) (on-bright-white on-bright-white nil) (bold bold nil) (strike strike nil) (up up "A") (down down "B") (forward forward "C") (backward backward "D") (next-line next-line "E") (previous-line previous-line "F") (column column "G") (kill kill "K") (missing nil nil) (nil nil nil) ("up" nil nil) (31 nil nil))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn substitution_recursively_rewrites_registered_alias_heads_without_evaluating_the_body() {
    let elisp_form = r##"(let ((forms
       '((red "error")
         (if ready
             (green "pass")
           (yellow "wait"))
         (list (bold "x") plain (column 4))
         (lambda (value)
           (on-blue (bright-white "%s" value)))
         [red (red "inside-vector")]
         nil
         42)))
  (list
   forms
   (ansi--substitute forms)
   (equal forms (ansi--substitute forms))))"##;
    let expect = expect![[
        r#"OK (((red "error") (if ready (green "pass") (yellow "wait")) (list (bold "x") plain (column 4)) (lambda (value) (on-blue (bright-white "%s" value))) #1=[red (red "inside-vector")] nil 42) ((ansi-red "error") (if ready (ansi-green "pass") (ansi-yellow "wait")) (list (ansi-bold "x") plain (ansi-column 4)) (lambda (value) (ansi-on-blue (ansi-bright-white "%s" value))) #1# nil 42) nil)"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn define_macro_supports_a_runtime_extended_effect_using_the_same_formatting_contract() {
    let elisp_form = r##"(let ((original-colors ansi-colors))
  (unwind-protect
      (progn
        (setq ansi-colors
              (append ansi-colors '((orange . 38))))
        (eval '(ansi--define orange))
        (list
         (fboundp 'ansi-orange)
         (help-function-arglist 'ansi-orange t)
         (documentation 'ansi-orange t)
         (ansi-orange "warning %s=%03d" "count" 7)
         (ansi--is-alias 'orange)
         (ansi--code 'orange)))
    (setq ansi-colors original-colors)
    (when (fboundp 'ansi-orange)
      (fmakunbound 'ansi-orange))))"##;
    let expect = expect![[
        r#"OK (t (format-string &rest objects) "Add \\='orange\\=' ansi effect to text." "\33[38mwarning count=007\33[0m" orange 38)"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn unknown_csi_symbols_remain_observable_as_literal_nil_final_characters() {
    let elisp_form = r##"(list
 (ansi-csi-apply 'missing)
 (ansi-csi-apply 'missing 7)
 (string-to-list (ansi-csi-apply 'missing 7))
 (ansi--char 'missing))"##;
    let expect = expect![[r#"OK ("\33[1nil" "\33[7nil" (27 91 55 110 105 108) nil)"#]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn unknown_effect_names_signal_during_numeric_sgr_formatting() {
    let elisp_form = r##"(ansi-apply 'missing "value=%s" "x")"##;
    let expect = expect![[r#"ERR (error "Format specifier doesn’t match argument type")"#]];
    assert_ansi_signal_parity(elisp_form, expect);
}

#[test]
fn malformed_payload_format_arguments_preserve_native_format_signals() {
    let elisp_form = r##"(ansi-red "%d/%s" "not-a-number")"##;
    let expect = expect![[r#"ERR (error "Format specifier doesn’t match argument type")"#]];
    assert_ansi_signal_parity(elisp_form, expect);
}
