use expect_test::expect;

use super::assert_ansi_parity;

#[test]
fn dsl_aliases_cover_every_foreground_background_style_and_cursor_registry_entry() {
    let elisp_form = r##"(let ((direct
       (list
        (ansi-black "x") (ansi-red "x") (ansi-green "x")
        (ansi-yellow "x") (ansi-blue "x") (ansi-magenta "x")
        (ansi-cyan "x") (ansi-white "x")
        (ansi-bright-black "x") (ansi-bright-red "x")
        (ansi-bright-green "x") (ansi-bright-yellow "x")
        (ansi-bright-blue "x") (ansi-bright-magenta "x")
        (ansi-bright-cyan "x") (ansi-bright-white "x")
        (ansi-on-black "x") (ansi-on-red "x") (ansi-on-green "x")
        (ansi-on-yellow "x") (ansi-on-blue "x") (ansi-on-magenta "x")
        (ansi-on-cyan "x") (ansi-on-white "x")
        (ansi-on-bright-black "x") (ansi-on-bright-red "x")
        (ansi-on-bright-green "x") (ansi-on-bright-yellow "x")
        (ansi-on-bright-blue "x") (ansi-on-bright-magenta "x")
        (ansi-on-bright-cyan "x") (ansi-on-bright-white "x")
        (ansi-bold "x") (ansi-dark "x") (ansi-italic "x")
        (ansi-underscore "x") (ansi-blink "x") (ansi-rapid "x")
        (ansi-contrary "x") (ansi-concealed "x") (ansi-strike "x")
        (ansi-up 2) (ansi-down 2) (ansi-forward 2) (ansi-backward 2)
        (ansi-next-line 2) (ansi-previous-line 2)
        (ansi-column 2) (ansi-kill 2)))
      (dsl
       (list
        (with-ansi (black "x")) (with-ansi (red "x"))
        (with-ansi (green "x")) (with-ansi (yellow "x"))
        (with-ansi (blue "x")) (with-ansi (magenta "x"))
        (with-ansi (cyan "x")) (with-ansi (white "x"))
        (with-ansi (bright-black "x")) (with-ansi (bright-red "x"))
        (with-ansi (bright-green "x")) (with-ansi (bright-yellow "x"))
        (with-ansi (bright-blue "x")) (with-ansi (bright-magenta "x"))
        (with-ansi (bright-cyan "x")) (with-ansi (bright-white "x"))
        (with-ansi (on-black "x")) (with-ansi (on-red "x"))
        (with-ansi (on-green "x")) (with-ansi (on-yellow "x"))
        (with-ansi (on-blue "x")) (with-ansi (on-magenta "x"))
        (with-ansi (on-cyan "x")) (with-ansi (on-white "x"))
        (with-ansi (on-bright-black "x"))
        (with-ansi (on-bright-red "x"))
        (with-ansi (on-bright-green "x"))
        (with-ansi (on-bright-yellow "x"))
        (with-ansi (on-bright-blue "x"))
        (with-ansi (on-bright-magenta "x"))
        (with-ansi (on-bright-cyan "x"))
        (with-ansi (on-bright-white "x"))
        (with-ansi (bold "x")) (with-ansi (dark "x"))
        (with-ansi (italic "x")) (with-ansi (underscore "x"))
        (with-ansi (blink "x")) (with-ansi (rapid "x"))
        (with-ansi (contrary "x")) (with-ansi (concealed "x"))
        (with-ansi (strike "x"))
        (with-ansi (up 2)) (with-ansi (down 2))
        (with-ansi (forward 2)) (with-ansi (backward 2))
        (with-ansi (next-line 2)) (with-ansi (previous-line 2))
        (with-ansi (column 2)) (with-ansi (kill 2)))))
  (list (length direct) (length dsl) (equal direct dsl) direct dsl))"##;
    let expect = expect![[
        r#"OK (49 49 t ("\33[30mx\33[0m" "\33[31mx\33[0m" "\33[32mx\33[0m" "\33[33mx\33[0m" "\33[34mx\33[0m" "\33[35mx\33[0m" "\33[36mx\33[0m" "\33[37mx\33[0m" "\33[90mx\33[0m" "\33[91mx\33[0m" "\33[92mx\33[0m" "\33[93mx\33[0m" "\33[94mx\33[0m" "\33[95mx\33[0m" "\33[96mx\33[0m" "\33[97mx\33[0m" "\33[40mx\33[0m" "\33[41mx\33[0m" "\33[42mx\33[0m" "\33[43mx\33[0m" "\33[44mx\33[0m" "\33[45mx\33[0m" "\33[46mx\33[0m" "\33[47mx\33[0m" "\33[100mx\33[0m" "\33[101mx\33[0m" "\33[102mx\33[0m" "\33[103mx\33[0m" "\33[104mx\33[0m" "\33[105mx\33[0m" "\33[106mx\33[0m" "\33[107mx\33[0m" "\33[1mx\33[0m" "\33[2mx\33[0m" "\33[3mx\33[0m" "\33[4mx\33[0m" "\33[5mx\33[0m" "\33[6mx\33[0m" "\33[7mx\33[0m" "\33[8mx\33[0m" "\33[9mx\33[0m" "\33[2A" "\33[2B" "\33[2C" "\33[2D" "\33[2E" "\33[2F" "\33[2G" "\33[2K") ("\33[30mx\33[0m" "\33[31mx\33[0m" "\33[32mx\33[0m" "\33[33mx\33[0m" "\33[34mx\33[0m" "\33[35mx\33[0m" "\33[36mx\33[0m" "\33[37mx\33[0m" "\33[90mx\33[0m" "\33[91mx\33[0m" "\33[92mx\33[0m" "\33[93mx\33[0m" "\33[94mx\33[0m" "\33[95mx\33[0m" "\33[96mx\33[0m" "\33[97mx\33[0m" "\33[40mx\33[0m" "\33[41mx\33[0m" "\33[42mx\33[0m" "\33[43mx\33[0m" "\33[44mx\33[0m" "\33[45mx\33[0m" "\33[46mx\33[0m" "\33[47mx\33[0m" "\33[100mx\33[0m" "\33[101mx\33[0m" "\33[102mx\33[0m" "\33[103mx\33[0m" "\33[104mx\33[0m" "\33[105mx\33[0m" "\33[106mx\33[0m" "\33[107mx\33[0m" "\33[1mx\33[0m" "\33[2mx\33[0m" "\33[3mx\33[0m" "\33[4mx\33[0m" "\33[5mx\33[0m" "\33[6mx\33[0m" "\33[7mx\33[0m" "\33[8mx\33[0m" "\33[9mx\33[0m" "\33[2A" "\33[2B" "\33[2C" "\33[2D" "\33[2E" "\33[2F" "\33[2G" "\33[2K"))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn dsl_evaluates_each_body_form_once_in_order_and_concatenates_only_string_results() {
    let elisp_form = r##"(let (events)
  (let ((value
         (with-ansi
          (progn (push 'first events) "prefix ")
          (progn (push 'ignored-nil events) nil)
          (progn (push 'ignored-number events) 42)
          (progn (push 'colored events) (green "%s" "ready"))
          (progn (push 'ignored-list events) '(not text))
          (progn (push 'last events) "\n"))))
    (list value (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("prefix \33[32mready\33[0m\n" (first ignored-nil ignored-number colored ignored-list last))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn dsl_works_inside_real_conditionals_loops_and_local_value_bindings() {
    let elisp_form = r##"(let ((rows
       '((compile . success)
         (lint . warning)
         (test . failure))))
  (mapconcat
   (lambda (row)
     (let ((name (symbol-name (car row)))
           (state (cdr row)))
       (with-ansi
        (bold "%-8s" name)
        " "
        (cond
         ((eq state 'success) (green "PASS"))
         ((eq state 'warning) (yellow "WARN"))
         (t (on-red (bright-white "FAIL")))))))
   rows
   "\n"))"##;
    let expect = expect![[
        r#"OK "\33[1mcompile \33[0m \33[32mPASS\33[0m\n\33[1mlint    \33[0m \33[33mWARN\33[0m\n\33[1mtest    \33[0m \33[41m\33[97mFAIL\33[0m\33[0m""#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn with_ansi_princ_writes_the_complete_composed_value_to_the_selected_output_stream() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer " *ansi-princ*")))
  (unwind-protect
      (let ((standard-output buffer))
        (list
         (with-ansi-princ
          (bold "Task: ")
          (cyan "%-10s" "compile")
          " "
          (green "%d/%d" 12 12)
          "\n")
         (with-current-buffer buffer
           (buffer-string))
         (with-current-buffer buffer
           (string-to-list (buffer-string)))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ("\33[1mTask: \33[0m\33[36mcompile   \33[0m \33[32m12/12\33[0m\n" "\33[1mTask: \33[0m\33[36mcompile   \33[0m \33[32m12/12\33[0m\n" (27 91 49 109 84 97 115 107 58 32 27 91 48 109 27 91 51 54 109 99 111 109 112 105 108 101 32 32 32 27 91 48 109 32 27 91 51 50 109 49 50 47 49 50 27 91 48 109 10))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn macroexpansion_rewrites_only_registered_alias_calls_and_preserves_nested_control_forms() {
    let elisp_form = r##"(mapcar
 (lambda (form)
   (list form
         (macroexpand-1 form)
         (macroexpand-all form)))
 '((with-ansi (red "error") " " (bold "%d" 3))
   (with-ansi
    (if ready (green "ready") (yellow "waiting"))
    (column 20))
   (with-ansi-princ
    (on-blue (bright-white " title "))
    "\n")))"##;
    let expect = expect![[
        r#"OK (((with-ansi (red "error") " " (bold "%d" 3)) (ansi--concat (ansi-red "error") " " (ansi-bold "%d" 3)) (ansi--concat (ansi-red "error") " " (ansi-bold "%d" 3))) ((with-ansi (if ready (green "ready") (yellow "waiting")) (column 20)) (ansi--concat (if ready (ansi-green "ready") (ansi-yellow "waiting")) (ansi-column 20)) (ansi--concat (if ready (ansi-green "ready") (ansi-yellow "waiting")) (ansi-column 20))) ((with-ansi-princ #1=(on-blue (bright-white " title ")) "\n") (princ (with-ansi #1# "\n")) (princ (ansi--concat (ansi-on-blue (ansi-bright-white " title ")) "\n"))))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn deeply_nested_dsl_matches_the_equivalent_explicit_function_pipeline_byte_for_byte() {
    let elisp_form = r##"(let ((explicit
       (ansi-bold
        (ansi-red
         (ansi-on-bright-white
          (ansi-underscore
           (ansi-blink "deploy %s #%d" "api" 2048))))))
      (dsl
       (with-ansi
        (bold
         (red
          (on-bright-white
           (underscore
            (blink "deploy %s #%d" "api" 2048))))))))
  (list explicit dsl (equal explicit dsl) (string-to-list dsl)))"##;
    let expect = expect![[
        r#"OK ("\33[1m\33[31m\33[107m\33[4m\33[5mdeploy api #2048\33[0m\33[0m\33[0m\33[0m\33[0m" "\33[1m\33[31m\33[107m\33[4m\33[5mdeploy api #2048\33[0m\33[0m\33[0m\33[0m\33[0m" t (27 91 49 109 27 91 51 49 109 27 91 49 48 55 109 27 91 52 109 27 91 53 109 100 101 112 108 111 121 32 97 112 105 32 35 50 48 52 56 27 91 48 109 27 91 48 109 27 91 48 109 27 91 48 109 27 91 48 109))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}
