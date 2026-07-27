use expect_test::expect;

use super::assert_acton_mode_parity;

#[test]
fn acton_mode_calculates_top_level_colon_default_and_blank_line_indentation() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (with-temp-buffer
             (acton-mode)
             (insert source)
             (goto-char
              (point-max))
             (beginning-of-line)
             (list
              source
              (current-indentation)
              (acton-calculate-indentation))))
         '("first"
           "first\nsecond"
           "if ready:\nbody"
           "    if ready:\nbody"
           "header\nif ready:\nbody"
           "header\nif ready:   \nbody"
           "header\n    value\nnext"
           "header\nif ready:\n\n\nbody"
           "header\n# comment:\nbody"
           "header\n\nbody"))"##;
    let expect = expect![[
        r#"OK (("first" 0 0) ("first\nsecond" 0 0) ("if ready:\nbody" 0 0) ("    if ready:\nbody" 0 0) ("header\nif ready:\nbody" 0 4) ("header\nif ready:   \nbody" 0 4) ("header\n    value\nnext" 0 4) ("header\nif ready:\n\n\nbody" 0 4) ("header\n# comment:\nbody" 0 4) ("header\n\nbody" 0 0))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_deindents_after_every_block_ending_statement_with_correct_priority() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (let ((statement
                  (car case))
                 (indent
                  (cadr case)))
             (with-temp-buffer
               (acton-mode)
               (insert
                "header\n"
                (make-string
                 indent
                 ?\s)
                statement
                "\n"
                "next")
               (goto-char
                (point-max))
               (beginning-of-line)
               (list
                statement
                indent
                (acton-calculate-indentation)))))
         '(("return value" 8)
           ("pass" 8)
           ("break" 8)
           ("continue" 8)
           ("raise Error" 8)
           ("return:" 8)
           ("yield value" 8)
           ("assert ready" 8)
           ("return value" 2)))"##;
    let expect = expect![[
        r#"OK (("return value" 8 4) ("pass" 8 4) ("break" 8 4) ("continue" 8 4) ("raise Error" 8 4) ("return:" 8 4) ("yield value" 8 8) ("assert ready" 8 8) ("return value" 2 0))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_indents_inside_each_pair_after_empty_and_inline_openers() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (with-temp-buffer
             (acton-mode)
             (insert source)
             (goto-char
              (point-max))
             (beginning-of-line)
             (list
              source
              (acton-calculate-indentation)
              (nth 1
                   (syntax-ppss)))))
         '("header\n    call(\nvalue"
           "header\n    call(value,\nnext"
           "header\n    items[\nvalue"
           "header\n    items[value,\nnext"
           "header\n    mapping{\nvalue"
           "header\n    mapping{key:\nnext"))"##;
    let expect = expect![[
        r#"OK (("header\n    call(\nvalue" 8 16) ("header\n    call(value,\nnext" 5 16) ("header\n    items[\nvalue" 8 17) ("header\n    items[value,\nnext" 5 17) ("header\n    mapping{\nvalue" 8 19) ("header\n    mapping{key:\nnext" 5 19))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_uses_opening_line_indentation_inside_single_and_double_strings() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (with-temp-buffer
             (acton-mode)
             (insert source)
             (goto-char
              (point-max))
             (beginning-of-line)
             (let ((state
                    (syntax-ppss)))
               (list
                source
                (nth 3 state)
                (nth 8 state)
                (acton-calculate-indentation)))))
         '("header\n    \"double text\ncontinuation"
           "header\n      'single text\ncontinuation"
           "header\n\"top text\ncontinuation"))"##;
    let expect = expect![[
        r#"OK (("header\n    \"double text\ncontinuation" 34 12 4) ("header\n      'single text\ncontinuation" 39 14 6) ("header\n\"top text\ncontinuation" 34 8 0))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_indent_line_reindents_text_and_preserves_or_moves_point_correctly() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (acton-mode)
           (insert
            "header\nif ready:\n  value")
           (goto-char
            (point-max))
           (let ((before
                  (list
                   (point)
                   (current-column)
                   (current-indentation))))
             (acton-indent-line)
             (list
              before
              (buffer-string)
              (point)
              (current-column)
              (current-indentation))))
         (with-temp-buffer
           (acton-mode)
           (insert
            "header\nif ready:\n  value")
           (goto-char
            (point-max))
           (beginning-of-line)
           (let ((before
                  (list
                   (point)
                   (current-column)
                   (current-indentation))))
             (acton-indent-line)
             (list
              before
              (buffer-string)
              (point)
              (current-column)
              (current-indentation)))))"##;
    let expect = expect![[
        r#"OK (((25 7 2) "header\nif ready:\n    value" 27 9 4) ((18 0 2) "header\nif ready:\n    value" 22 4 4))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_indent_line_clamps_negative_calculation_to_zero() {
    let elisp_form = r##"(with-temp-buffer
         (acton-mode)
         (insert
          "    value")
         (goto-char
          (point-max))
         (cl-letf
             (((symbol-function
                'acton-calculate-indentation)
               (lambda ()
                 -7)))
           (let ((result
                  (acton-indent-line)))
             (list
              result
              (buffer-string)
              (point)
              (current-column)
              (current-indentation)))))"##;
    let expect = expect![[r#"OK (nil "value" 6 5 0)"#]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_indent_line_falls_back_to_zero_when_calculation_signals() {
    let elisp_form = r##"(with-temp-buffer
         (acton-mode)
         (insert
          "    value")
         (goto-char
          (point-max))
         (cl-letf
             (((symbol-function
                'acton-calculate-indentation)
               (lambda ()
                 (error
                  "forced indentation failure"))))
           (let ((result
                  (acton-indent-line)))
             (list
              result
              (buffer-string)
              (point)
              (current-column)
              (current-indentation)))))"##;
    let expect = expect![[r#"OK (nil "value" 6 5 0)"#]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_custom_offset_controls_colon_pair_and_deindent_amounts() {
    let elisp_form = r##"(let ((acton-indent-offset
                2))
         (mapcar
          (lambda (source)
            (with-temp-buffer
              (acton-mode)
              (insert source)
              (goto-char
               (point-max))
              (beginning-of-line)
              (list
               source
               (acton-calculate-indentation))))
          '("header\nif ready:\nbody"
            "header\n  call(\nvalue"
            "header\n      return value\nnext"
            "header\n  value\nnext")))"##;
    let expect = expect![[
        r#"OK (("header\nif ready:\nbody" 2) ("header\n  call(\nvalue" 4) ("header\n      return value\nnext" 4) ("header\n  value\nnext" 2))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}
