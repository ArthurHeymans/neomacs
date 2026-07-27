use expect_test::expect;

use super::assert_alsamixer_parity;

#[test]
fn default_command_builder_covers_get_set_and_toggle_shapes() {
    let elisp_form = r##"
(list
 (alsamixer-command "sget %C playback")
 (alsamixer-command "sset %C playback %d%%" 73)
 (alsamixer-command "set %C toggle"))
"##;
    let expect = expect![[
        r#"OK ("amixer sget Master playback" "amixer sset Master playback 73%" "amixer set Master toggle")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn card_device_control_and_format_arguments_follow_exact_cli_order() {
    let elisp_form = r##"
(let ((alsamixer-amixer-command "/opt/audio/bin/amixer")
      (alsamixer-card 2)
      (alsamixer-device "hw:2,0")
      (alsamixer-control "PCM"))
  (list
   (alsamixer-command "sget %C playback")
   (alsamixer-command
    "sset %C playback %d%% %s"
    87 "unmute")))
"##;
    let expect = expect![[
        r#"OK ("/opt/audio/bin/amixer -c 2 -D hw:2,0 sget PCM playback" "/opt/audio/bin/amixer -c 2 -D hw:2,0 sset PCM playback 87% unmute")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn control_replacement_is_literal_case_insensitive_and_applies_everywhere() {
    let elisp_form = r##"
(let ((alsamixer-control "MiX\\1&Aux"))
  (list
   (alsamixer-command "%C + %c + %C")
   (alsamixer-command
    "set %C %s"
    "toggle")))
"##;
    let expect = expect![[
        r#"OK ("amixer MiX\\1&Aux + MiX\\1&Aux + MiX\\1&Aux" "amixer set MiX\\1&Aux toggle")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn nil_zero_and_empty_device_values_preserve_elisp_truthiness_rules() {
    let elisp_form = r##"
(mapcar
 (lambda (configuration)
   (let ((alsamixer-card (car configuration))
         (alsamixer-device (cadr configuration)))
     (alsamixer-command "sget %C")))
 '((nil nil)
   (0 nil)
   (nil "")
   (0 "")))
"##;
    let expect = expect![[
        r#"OK ("amixer sget Master" "amixer -c 0 sget Master" "amixer -D  sget Master" "amixer -c 0 -D  sget Master")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn spaces_and_shell_metacharacters_are_left_unquoted_in_constructed_command() {
    let elisp_form = r##"
(let ((alsamixer-amixer-command "env AUDIO_MODE=desk amixer")
      (alsamixer-device "hw:USB Audio,0")
      (alsamixer-control "Speaker & Headphone"))
  (alsamixer-command
   "sset %C playback %s"
   "40%; printf injected"))
"##;
    let expect = expect![[
        r#"OK "env AUDIO_MODE=desk amixer -D hw:USB Audio,0 sset Speaker & Headphone playback 40%; printf injected""#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn card_custom_type_and_runtime_integer_formatter_disagree_for_string_value() {
    let elisp_form = r##"
(let ((alsamixer-card "2"))
  (condition-case error-data
      (alsamixer-command "sget %C")
    (error
     (cons (car error-data)
           (cdr error-data)))))
"##;
    let expect = expect![[r#"OK (error "Format specifier doesn’t match argument type")"#]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn format_objects_support_strings_characters_width_and_literal_percent() {
    let elisp_form = r##"
(let ((case-fold-search nil))
  (alsamixer-command
   "sset %C playback %03d%% channel=%s marker=%c"
   7 "front-left" 65))
"##;
    let expect = expect![[r#"OK "amixer sset Master playback 007% channel=front-left marker=A""#]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn malformed_format_strings_report_exact_errors_after_command_assembly() {
    let elisp_form = r##"
(mapcar
 (lambda (case)
   (condition-case error-data
       (apply #'alsamixer-command case)
     (error
      (cons (car error-data)
            (cdr error-data)))))
 '(("sset %C %s")
   ("sset %C %q" "value")
   ("plain" "unused")))
"##;
    let expect = expect![[
        r#"OK ((error "Not enough arguments for format string") (error "Invalid format operation %q") "amixer plain")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn percent_characters_in_program_or_control_participate_in_final_format_pass() {
    let elisp_form = r##"
(list
 (let ((alsamixer-amixer-command "amixer-100%"))
   (condition-case error-data
       (alsamixer-command "sget %C")
     (error
      (cons (car error-data)
            (cdr error-data)))))
 (let ((alsamixer-control "PCM%Left"))
   (condition-case error-data
       (alsamixer-command "sget %C")
     (error
      (cons (car error-data)
            (cdr error-data))))))
"##;
    let expect = expect![[
        r#"OK ((error "Not enough arguments for format string") (error "Not enough arguments for format string"))"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}
