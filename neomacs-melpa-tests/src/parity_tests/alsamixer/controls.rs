use expect_test::expect;

use super::assert_alsamixer_parity;

#[test]
fn set_volume_executes_in_range_value_and_returns_exact_user_message() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "hardware acknowledgement\n")
  (let ((message-log-max 100))
    (list
     (alsamixer-set-volume 47)
     message-log-max
     (alsamixer-test-log))))
"##;
    let expect =
        expect![[r#"OK ("Volume set to 47%" 100 "<sset>\n<Master>\n<playback>\n<47%>\n")"#]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn set_volume_clamps_every_value_outside_zero_to_hundred_range() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "")
  (let ((results
         (mapcar #'alsamixer-set-volume
                 '(-200 -1 0 1 99 100 101 900))))
    (list
     results
     (alsamixer-test-log))))
"##;
    let expect = expect![[
        r#"OK (("Volume set to 0%" "Volume set to 0%" "Volume set to 0%" "Volume set to 1%" "Volume set to 99%" "Volume set to 100%" "Volume set to 100%" "Volume set to 100%") "<sset>\n<Master>\n<playback>\n<0%>\n<sset>\n<Master>\n<playback>\n<0%>\n<sset>\n<Master>\n<playback>\n<0%>\n<sset>\n<Master>\n<playback>\n<1%>\n<sset>\n<Master>\n<playback>\n<99%>\n<sset>\n<Master>\n<playback>\n<100%>\n<sset>\n<Master>\n<playback>\n<100%>\n<sset>\n<Master>\n<playback>\n<100%>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn set_volume_reports_success_even_when_amixer_process_exits_nonzero() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure
   "amixer: write failed\n"
   17)
  (list
   (alsamixer-set-volume 52)
   (alsamixer-test-log)))
"##;
    let expect = expect![[r#"OK ("Volume set to 52%" "<sset>\n<Master>\n<playback>\n<52%>\n")"#]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn up_volume_reads_current_level_then_applies_default_increment() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure
   "Front Left: Playback 12000 [41%] [on]\n")
  (let ((alsamixer-default-volume-increment 5))
    (list
     (alsamixer-up-volume)
     (alsamixer-test-log))))
"##;
    let expect = expect![[
        r#"OK ("Volume set to 46%" "<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<46%>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn up_volume_honors_explicit_positive_negative_and_large_steps_with_clamping() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "[40%]\n")
  (let ((positive (alsamixer-up-volume 7)))
    (alsamixer-test-set-output "[40%]\n")
    (let ((negative (alsamixer-up-volume -75)))
      (alsamixer-test-set-output "[40%]\n")
      (let ((large (alsamixer-up-volume 500)))
        (list
         positive
         negative
         large
         (alsamixer-test-log))))))
"##;
    let expect = expect![[
        r#"OK ("Volume set to 47%" "Volume set to 0%" "Volume set to 100%" "<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<47%>\n<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<0%>\n<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<100%>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn down_volume_uses_default_or_explicit_step_and_clamps_below_zero() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "[12%]\n")
  (let ((alsamixer-default-volume-increment 6))
    (let ((default (alsamixer-down-volume)))
      (alsamixer-test-set-output "[12%]\n")
      (let ((explicit (alsamixer-down-volume 9)))
        (alsamixer-test-set-output "[12%]\n")
        (let ((clamped (alsamixer-down-volume 90)))
          (list
           default
           explicit
           clamped
           (alsamixer-test-log)))))))
"##;
    let expect = expect![[
        r#"OK ("Volume set to 6%" "Volume set to 3%" "Volume set to 0%" "<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<6%>\n<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<3%>\n<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<0%>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn set_volume_truncates_float_for_amixer_but_reports_original_and_rejects_nonnumbers() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "")
  (list
   (mapcar
    (lambda (value)
      (condition-case error-data
          (alsamixer-set-volume value)
        (error
         (cons (car error-data)
               (cdr error-data)))))
    '(50.5 "50" nil))
   (alsamixer-test-log)))
"##;
    let expect = expect![[
        r#"OK (("Volume set to 50.5%" (wrong-type-argument number-or-marker-p "50") (wrong-type-argument number-or-marker-p nil)) "<sset>\n<Master>\n<playback>\n<50%>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn volume_adjustment_flow_preserves_custom_card_device_and_control_on_both_calls() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "[66%]\n")
  (let ((alsamixer-card 4)
        (alsamixer-device "hw:4,0")
        (alsamixer-control "Headphone"))
    (list
     (alsamixer-down-volume 11)
     (alsamixer-test-log))))
"##;
    let expect = expect![[
        r#"OK ("Volume set to 55%" "<-c>\n<4>\n<-D>\n<hw:4,0>\n<sget>\n<Headphone>\n<playback>\n<-c>\n<4>\n<-D>\n<hw:4,0>\n<sset>\n<Headphone>\n<playback>\n<55%>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn interactive_set_volume_reads_number_and_runs_complete_command_flow() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "")
  (let (read-calls)
    (cl-letf
        (((symbol-function 'read-number)
          (lambda (&rest arguments)
            (push arguments read-calls)
            63)))
      (list
       (call-interactively #'alsamixer-set-volume)
       (nreverse read-calls)
       (alsamixer-test-log)))))
"##;
    let expect = expect![[
        r#"OK ("Volume set to 63%" (("Volume (percentage): ")) "<sset>\n<Master>\n<playback>\n<63%>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn interactive_up_and_down_accept_numeric_prefix_arguments() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "[50%]\n")
  (let ((up
         (let ((current-prefix-arg 8))
           (call-interactively
            #'alsamixer-up-volume))))
    (alsamixer-test-set-output "[50%]\n")
    (let ((down
           (let ((current-prefix-arg 13))
             (call-interactively
              #'alsamixer-down-volume))))
      (list
       up
       down
       (alsamixer-test-log)))))
"##;
    let expect = expect![[
        r#"OK ("Volume set to 58%" "Volume set to 37%" "<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<58%>\n<sget>\n<Master>\n<playback>\n<sset>\n<Master>\n<playback>\n<37%>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn interactive_universal_prefix_exposes_raw_prefix_arithmetic_error() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "[50%]\n")
  (list
   (condition-case error-data
       (let ((current-prefix-arg '(4)))
         (call-interactively
          #'alsamixer-up-volume))
     (error
      (cons (car error-data)
            (cdr error-data))))
   (alsamixer-test-log)))
"##;
    let expect = expect![[
        r#"OK ((wrong-type-argument number-or-marker-p (4)) "<sget>\n<Master>\n<playback>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}

#[test]
fn toggle_mute_supports_direct_and_interactive_calls_with_exact_cli_tokens() {
    let elisp_form = r##"
(progn
  (alsamixer-test-configure "mute toggled\n")
  (let ((direct (alsamixer-toggle-mute))
        (interactive
         (call-interactively
          #'alsamixer-toggle-mute)))
    (list
     direct
     interactive
     (alsamixer-test-log))))
"##;
    let expect = expect![[
        r#"OK ("mute toggled\n" "mute toggled\n" "<set>\n<Master>\n<toggle>\n<set>\n<Master>\n<toggle>\n")"#
    ]];
    assert_alsamixer_parity(elisp_form, expect);
}
