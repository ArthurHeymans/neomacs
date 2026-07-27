use expect_test::expect;

use super::assert_ansi_parity;

#[test]
fn every_standard_and_bright_foreground_function_emits_its_exact_sgr_sequence() {
    let elisp_form = r##"(mapcar
 (lambda (entry)
   (let* ((symbol (car entry))
          (code (cdr entry))
          (value (funcall symbol "status")))
     (list symbol code value (string-to-list value))))
 '((ansi-black . 30)
   (ansi-red . 31)
   (ansi-green . 32)
   (ansi-yellow . 33)
   (ansi-blue . 34)
   (ansi-magenta . 35)
   (ansi-cyan . 36)
   (ansi-white . 37)
   (ansi-bright-black . 90)
   (ansi-bright-red . 91)
   (ansi-bright-green . 92)
   (ansi-bright-yellow . 93)
   (ansi-bright-blue . 94)
   (ansi-bright-magenta . 95)
   (ansi-bright-cyan . 96)
   (ansi-bright-white . 97)))"##;
    let expect = expect![[
        r#"OK ((ansi-black 30 "\33[30mstatus\33[0m" (27 91 51 48 109 115 116 97 116 117 115 27 91 48 109)) (ansi-red 31 "\33[31mstatus\33[0m" (27 91 51 49 109 115 116 97 116 117 115 27 91 48 109)) (ansi-green 32 "\33[32mstatus\33[0m" (27 91 51 50 109 115 116 97 116 117 115 27 91 48 109)) (ansi-yellow 33 "\33[33mstatus\33[0m" (27 91 51 51 109 115 116 97 116 117 115 27 91 48 109)) (ansi-blue 34 "\33[34mstatus\33[0m" (27 91 51 52 109 115 116 97 116 117 115 27 91 48 109)) (ansi-magenta 35 "\33[35mstatus\33[0m" (27 91 51 53 109 115 116 97 116 117 115 27 91 48 109)) (ansi-cyan 36 "\33[36mstatus\33[0m" (27 91 51 54 109 115 116 97 116 117 115 27 91 48 109)) (ansi-white 37 "\33[37mstatus\33[0m" (27 91 51 55 109 115 116 97 116 117 115 27 91 48 109)) (ansi-bright-black 90 "\33[90mstatus\33[0m" (27 91 57 48 109 115 116 97 116 117 115 27 91 48 109)) (ansi-bright-red 91 "\33[91mstatus\33[0m" (27 91 57 49 109 115 116 97 116 117 115 27 91 48 109)) (ansi-bright-green 92 "\33[92mstatus\33[0m" (27 91 57 50 109 115 116 97 116 117 115 27 91 48 109)) (ansi-bright-yellow 93 "\33[93mstatus\33[0m" (27 91 57 51 109 115 116 97 116 117 115 27 91 48 109)) (ansi-bright-blue 94 "\33[94mstatus\33[0m" (27 91 57 52 109 115 116 97 116 117 115 27 91 48 109)) (ansi-bright-magenta 95 "\33[95mstatus\33[0m" (27 91 57 53 109 115 116 97 116 117 115 27 91 48 109)) (ansi-bright-cyan 96 "\33[96mstatus\33[0m" (27 91 57 54 109 115 116 97 116 117 115 27 91 48 109)) (ansi-bright-white 97 "\33[97mstatus\33[0m" (27 91 57 55 109 115 116 97 116 117 115 27 91 48 109)))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn every_standard_and_bright_background_function_emits_its_exact_sgr_sequence() {
    let elisp_form = r##"(mapcar
 (lambda (entry)
   (let* ((symbol (car entry))
          (code (cdr entry))
          (value (funcall symbol "cell")))
     (list symbol code value (string-to-list value))))
 '((ansi-on-black . 40)
   (ansi-on-red . 41)
   (ansi-on-green . 42)
   (ansi-on-yellow . 43)
   (ansi-on-blue . 44)
   (ansi-on-magenta . 45)
   (ansi-on-cyan . 46)
   (ansi-on-white . 47)
   (ansi-on-bright-black . 100)
   (ansi-on-bright-red . 101)
   (ansi-on-bright-green . 102)
   (ansi-on-bright-yellow . 103)
   (ansi-on-bright-blue . 104)
   (ansi-on-bright-magenta . 105)
   (ansi-on-bright-cyan . 106)
   (ansi-on-bright-white . 107)))"##;
    let expect = expect![[
        r#"OK ((ansi-on-black 40 "\33[40mcell\33[0m" (27 91 52 48 109 99 101 108 108 27 91 48 109)) (ansi-on-red 41 "\33[41mcell\33[0m" (27 91 52 49 109 99 101 108 108 27 91 48 109)) (ansi-on-green 42 "\33[42mcell\33[0m" (27 91 52 50 109 99 101 108 108 27 91 48 109)) (ansi-on-yellow 43 "\33[43mcell\33[0m" (27 91 52 51 109 99 101 108 108 27 91 48 109)) (ansi-on-blue 44 "\33[44mcell\33[0m" (27 91 52 52 109 99 101 108 108 27 91 48 109)) (ansi-on-magenta 45 "\33[45mcell\33[0m" (27 91 52 53 109 99 101 108 108 27 91 48 109)) (ansi-on-cyan 46 "\33[46mcell\33[0m" (27 91 52 54 109 99 101 108 108 27 91 48 109)) (ansi-on-white 47 "\33[47mcell\33[0m" (27 91 52 55 109 99 101 108 108 27 91 48 109)) (ansi-on-bright-black 100 "\33[100mcell\33[0m" (27 91 49 48 48 109 99 101 108 108 27 91 48 109)) (ansi-on-bright-red 101 "\33[101mcell\33[0m" (27 91 49 48 49 109 99 101 108 108 27 91 48 109)) (ansi-on-bright-green 102 "\33[102mcell\33[0m" (27 91 49 48 50 109 99 101 108 108 27 91 48 109)) (ansi-on-bright-yellow 103 "\33[103mcell\33[0m" (27 91 49 48 51 109 99 101 108 108 27 91 48 109)) (ansi-on-bright-blue 104 "\33[104mcell\33[0m" (27 91 49 48 52 109 99 101 108 108 27 91 48 109)) (ansi-on-bright-magenta 105 "\33[105mcell\33[0m" (27 91 49 48 53 109 99 101 108 108 27 91 48 109)) (ansi-on-bright-cyan 106 "\33[106mcell\33[0m" (27 91 49 48 54 109 99 101 108 108 27 91 48 109)) (ansi-on-bright-white 107 "\33[107mcell\33[0m" (27 91 49 48 55 109 99 101 108 108 27 91 48 109)))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn effect_functions_apply_real_format_directives_width_precision_unicode_and_object_rendering() {
    let elisp_form = r##"(list
 (ansi-red "job %-8s %04d/%d" "build" 7 12)
 (ansi-bright-cyan "latency %.2fms" 12.345)
 (ansi-on-blue "用户=%s 状态=%S" "李雷" '通过)
 (ansi-yellow "percent=%% hex=%x char=%c" 255 65)
 (ansi-white "%-10.5s|%+6d" "deterministic" 42)
 (ansi-magenta "%S" '((phase . compile) (ok . t))))"##;
    let expect = expect![[
        r#"OK ("\33[31mjob build    0007/12\33[0m" "\33[96mlatency 12.35ms\33[0m" "\33[44m用户=李雷 状态=通过\33[0m" "\33[33mpercent=% hex=ff char=A\33[0m" "\33[37mdeter     |   +42\33[0m" "\33[35m((phase . compile) (ok . t))\33[0m")"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn symbol_and_numeric_apply_paths_are_equivalent_for_foreground_background_and_style_codes() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (let ((effect (nth 0 case))
         (code (nth 1 case))
         (format-string (nth 2 case))
         (objects (nthcdr 3 case)))
     (list
      effect
      (apply #'ansi-apply effect format-string objects)
      (apply #'ansi-apply code format-string objects)
      (equal
       (apply #'ansi-apply effect format-string objects)
       (apply #'ansi-apply code format-string objects)))))
 '((green 32 "PASS %-12s %03d" "parser" 9)
   (on-bright-red 101 " ALERT %s " "disk full")
   (bold 1 "%s: %.1f%%" "coverage" 98.75)))"##;
    let expect = expect![[
        r#"OK ((green "\33[32mPASS parser       009\33[0m" "\33[32mPASS parser       009\33[0m" t) (on-bright-red "\33[101m ALERT disk full \33[0m" "\33[101m ALERT disk full \33[0m" t) (bold "\33[1mcoverage: 98.8%\33[0m" "\33[1mcoverage: 98.8%\33[0m" t))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn adjacent_and_nested_effects_build_a_practical_colored_status_line_without_merging_resets() {
    let elisp_form = r##"(let* ((label (ansi-bold (ansi-white "BUILD")))
       (passed (ansi-green " 128 passed "))
       (failed (ansi-on-red (ansi-bright-white " 2 failed ")))
       (duration (ansi-dark " in %.2fs" 4.25))
       (line (concat label passed failed duration)))
  (list
   label
   passed
   failed
   duration
   line
   (string-to-list line)
   (length line)))"##;
    let expect = expect![[
        r#"OK ("\33[1m\33[37mBUILD\33[0m\33[0m" "\33[32m 128 passed \33[0m" "\33[41m\33[97m 2 failed \33[0m\33[0m" "\33[2m in 4.25s\33[0m" "\33[1m\33[37mBUILD\33[0m\33[0m\33[32m 128 passed \33[0m\33[41m\33[97m 2 failed \33[0m\33[0m\33[2m in 4.25s\33[0m" (27 91 49 109 27 91 51 55 109 66 85 73 76 68 27 91 48 109 27 91 48 109 27 91 51 50 109 32 49 50 56 32 112 97 115 115 101 100 32 27 91 48 109 27 91 52 49 109 27 91 57 55 109 32 50 32 102 97 105 108 101 100 32 27 91 48 109 27 91 48 109 27 91 50 109 32 105 110 32 52 46 50 53 115 27 91 48 109) 88)"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}
