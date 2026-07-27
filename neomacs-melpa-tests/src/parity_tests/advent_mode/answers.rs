use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_default_answer_covers_empty_number_symbol_and_line_fallbacks() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (pcase-let ((`(,text ,offset) case))
             (with-temp-buffer
               (insert text)
               (goto-char (+ (point-min) offset))
               (list case
                     (advent--default-answer)
                     (stringp (advent--default-answer))))))
         '(("" 0)
           ("  123  " 3)
           ("value=-42 rest" 8)
           ("  foo-bar  " 3)
           ("   abc def   \nnext" 0)
           (" \t \n" 0)
           ("alpha\nbeta\n" 6)))"##;
    let expect = expect![[
        r#"OK ((("" 0) "" t) (("  123  " 3) "123" t) (("value=-42 rest" 8) "-42" t) (("  foo-bar  " 3) "foo-bar" t) (("   abc def   \nnext" 0) "abc def" t) ((" \11 \n" 0) "" t) (("alpha\nbeta\n" 6) "beta" t))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_default_answer_active_region_precedes_every_thing_at_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert "prefix  42 and symbol  suffix")
         (setq transient-mark-mode t)
         (goto-char 7)
         (set-mark 20)
         (activate-mark)
         (list
          (region-active-p)
          (buffer-substring-no-properties
           (region-beginning)
           (region-end))
          (advent--default-answer)
          (progn
            (deactivate-mark)
            (goto-char 10)
            (advent--default-answer))))"##;
    let expect = expect![[r#"OK (t "  42 and symb" "42 and symb" "42")"#]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_default_answer_strips_properties_and_trims_region_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (insert (propertize "  decorated answer  "
                             'face 'bold
                             'advent-test-property '(one two)))
         (setq transient-mark-mode t)
         (goto-char (point-min))
         (set-mark (point-max))
         (activate-mark)
         (let ((answer (advent--default-answer)))
           (list answer
                 (text-properties-at 1 answer)
                 (length answer)
                 (string= answer "decorated answer"))))"##;
    let expect = expect![[r#"OK ("decorated answer" nil 16 t)"#]];
    assert_advent_mode_parity(elisp_form, expect);
}
