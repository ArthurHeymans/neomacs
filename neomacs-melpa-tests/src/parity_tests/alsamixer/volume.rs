use expect_test::expect;

use super::assert_alsamixer_parity;

#[test]
fn get_volume_parses_first_channel_from_realistic_stereo_amixer_output() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure
   "Simple mixer control 'Master',0\n  Capabilities: pvolume pswitch\n  Front Left: Playback 16384 [50%] [-12.00dB] [on]\n  Front Right: Playback 24576 [75%] [-6.00dB] [on]\n")
  (list
   (alsamixer-get-volume)
   (alsamixer-test-log)))
"##;
    let expect = expect![[r#"OK (50 "<sget>\n<Master>\n<playback>\n")"#]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn get_volume_preserves_zero_hundred_and_leading_zero_numeric_semantics() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "[0%]\n")
  (let ((zero (alsamixer-get-volume)))
    (alsamixer-test-set-output "[100%]\n")
    (let ((hundred (alsamixer-get-volume)))
      (alsamixer-test-set-output "[007%]\n")
      (list
       zero
       hundred
       (alsamixer-get-volume)
       (alsamixer-test-log)))))
"##;
    let expect = expect![[
        r#"OK (0 100 7 "<sget>\n<Master>\n<playback>\n<sget>\n<Master>\n<playback>\n<sget>\n<Master>\n<playback>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn get_volume_skips_switch_and_decibel_brackets_before_percentage() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure
   "Simple mixer control 'Master',0\n  Front Left: Playback 10 [on] [-64.00dB] [23%]\n")
  (list
   (alsamixer-get-volume)
   (alsamixer-test-log)))
"##;
    let expect = expect![[r#"OK (23 "<sget>\n<Master>\n<playback>\n")"#]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn malformed_command_output_signals_with_program_and_complete_payload() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure
   "amixer: Unable to find simple control 'Master',0\n")
  (condition-case error-data
      (alsamixer-get-volume)
    (error
     (list
      (car error-data)
      (cdr error-data)
      (alsamixer-test-log)))))
"##;
    let expect = expect![[
        r#"OK (error ("Unexpected output from [ORACLE-SANDBOX]/fake-amixer: amixer: Unable to find simple control 'Master',0\n") "<sget>\n<Master>\n<playback>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn decimal_negative_and_missing_percentages_are_rejected_exactly() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "")
  (list
   (mapcar
    (lambda (output)
      (alsamixer-test-set-output output)
      (condition-case error-data
          (alsamixer-get-volume)
        (error
         (list
          (car error-data)
          (cadr error-data)
          (car (last error-data))))))
    '("[37.5%]\n"
      "[-1%]\n"
      "[on]\n"
      ""))
   (alsamixer-test-log)))
"##;
    let expect = expect![[
        r#"OK (((error "Unexpected output from [ORACLE-SANDBOX]/fake-amixer: [37.5%]\n" "Unexpected output from [ORACLE-SANDBOX]/fake-amixer: [37.5%]\n") (error "Unexpected output from [ORACLE-SANDBOX]/fake-amixer: [-1%]\n" "Unexpected output from [ORACLE-SANDBOX]/fake-amixer: [-1%]\n") (error "Unexpected output from [ORACLE-SANDBOX]/fake-amixer: [on]\n" "Unexpected output from [ORACLE-SANDBOX]/fake-amixer: [on]\n") (error "Unexpected output from [ORACLE-SANDBOX]/fake-amixer: " "Unexpected output from [ORACLE-SANDBOX]/fake-amixer: ")) "<sget>\n<Master>\n<playback>\n<sget>\n<Master>\n<playback>\n<sget>\n<Master>\n<playback>\n<sget>\n<Master>\n<playback>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn valid_percentage_is_accepted_even_when_amixer_exits_nonzero() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure
   "partial hardware failure [61%]\n"
   9)
  (list
   (alsamixer-get-volume)
   (alsamixer-test-log)))
"##;
    let expect = expect![[r#"OK (61 "<sget>\n<Master>\n<playback>\n")"#]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn get_volume_sends_card_device_control_and_playback_tokens_to_process() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "[44%]\n")
  (let ((alsamixer-card 3)
        (alsamixer-device "hw:3,1")
        (alsamixer-control "PCM"))
    (list
     (alsamixer-get-volume)
     (alsamixer-test-log))))
"##;
    let expect = expect![[r#"OK (44 "<-c>\n<3>\n<-D>\n<hw:3,1>\n<sget>\n<PCM>\n<playback>\n")"#]];
    assert_alsamixer_parity(elisp_form, expect);
}
