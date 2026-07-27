use expect_test::expect;

use super::assert_actionscript_mode_parity;

#[test]
fn actionscript_mode_calculated_indentation_and_full_line_rewrite_match_nested_fixture() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "public function demo():void {\n")
         (insert
          "if (ready) {\n")
         (insert
          "trace(\"{\");   \n")
         (insert
          "} else {\n")
         (insert
          "// }\n")
         (insert
          "trace(2);\n")
         (insert
          "}\n")
         (insert
          "}\n")
         (actionscript-mode)
         (font-lock-ensure)
         (let (calculated
               points)
           (goto-char
            (point-min))
           (while
               (not
                (eobp))
             (push
              (as3-calculate-indentation)
              calculated)
             (end-of-line)
             (let ((before-point
                    (point))
                   (before-column
                    (current-column))
                   (before-relative
                    (-
                     (current-column)
                     (current-indentation))))
               (actionscript-indent-line)
               (push
                (list
                 before-point
                 (point)
                 before-column
                 (current-column)
                 before-relative
                 (-
                  (current-column)
                  (current-indentation)))
                points))
             (forward-line 1))
           (list
            (nreverse calculated)
            (nreverse points)
            (buffer-string)
            (buffer-modified-p))))"##;
    let expect = expect![[
        r#"OK ((0 4 8 4 8 8 4 0) ((30 30 29 29 29 29) (43 47 12 16 12 12) (62 60 14 19 14 11) (69 73 8 12 8 8) (78 79 4 12 4 4) (89 90 9 17 9 9) (92 96 1 5 1 1) (98 98 1 1 1 1)) #("public function demo():void {\n    if (ready) {\n\11trace(\"{\");\n    } else {\n\11// }\n\11trace(2);\n    }\n}\n" 0 6 (face font-lock-keyword-face) 7 15 (face font-lock-keyword-face) 16 20 (face font-lock-function-name-face) 23 27 (face font-lock-keyword-face) 34 36 (face font-lock-keyword-face) 48 53 (face font-lock-function-name-face) 54 57 (face font-lock-string-face) 66 70 (face font-lock-keyword-face) 74 77 (face font-lock-comment-delimiter-face) 77 80 (face font-lock-comment-face) 80 85 (face font-lock-function-name-face)) t)"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_leading_close_delimiter_skip_handles_whitespace_tokens_and_plain_lines() {
    let elisp_form = r##"(mapcar
         (lambda (line)
           (with-temp-buffer
             (insert line)
             (actionscript-mode)
             (goto-char
              (point-min))
             (let ((result
                    (as3-maybe-skip-leading-close-delim)))
               (list
                line
                result
                (point)
                (char-after)
                (current-column)))))
         '("    }\n"
           "\t]\n"
           "  ) tail\n"
           "    value\n"
           "\n"))"##;
    let expect = expect![[
        r#"OK (("    }\n" nil 6 10 5) ("\11]\n" nil 3 10 9) ("  ) tail\n" nil 4 32 3) ("    value\n" nil 1 32 0) ("\n" nil 1 10 0))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_scope_depth_counts_delimiters_but_ignores_fontified_strings_and_comments() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "{ call(\"{\"); // { ignored\n")
         (insert
          "  /* } ignored */ [ value ]\n")
         (insert
          "  if (x) { y(); }\n")
         (insert
          "}\n")
         (actionscript-mode)
         (let ((without-font-lock
                (as3-count-scope-depth
                 (point-min)
                 (point-max))))
           (font-lock-ensure)
           (list
            without-font-lock
            (as3-count-scope-depth
             (point-min)
             (point-max))
            (mapcar
             (lambda (needle)
               (goto-char
                (point-min))
               (search-forward
                needle)
               (list
                needle
                (as3-face-at-point
                 (1-
                  (point)))
                (syntax-ppss
                 (1-
                  (point)))))
             '("\"{\""
               "{ ignored"
               "} ignored"
               "if"
               "y")))))"##;
    let expect = expect![[
        r#"OK (1 0 (("\"{\"" font-lock-string-face (2 7 nil 34 nil nil 0 nil 8 (1 7) nil)) ("{ ignored" font-lock-comment-face (1 1 7 nil t nil 0 1 14 (1) nil)) ("} ignored" font-lock-comment-face (1 1 7 nil t nil 0 nil 29 (1) nil)) ("if" font-lock-keyword-face (1 1 57 nil nil nil 0 nil nil (1) nil)) ("y" nil (2 64 nil nil nil nil 0 nil nil (1 64) nil))))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_indent_level_customization_changes_depth_and_zero_or_negative_edges() {
    let elisp_form = r##"(mapcar
         (lambda (level)
           (let ((actionscript-indent-level
                  level))
             (with-temp-buffer
               (insert
                "if (x) {\nvalue();\n}\n")
               (actionscript-mode)
               (goto-char
                (point-min))
               (forward-line 1)
               (list
                level
                (as3-calculate-indentation)
                (condition-case error
                    (progn
                      (actionscript-indent-line)
                      (list
                       (current-indentation)
                       (buffer-string)))
                  (error
                   (list
                    (car error)
                    (cdr error))))))))
         '(8 2 0 -2))"##;
    let expect = expect![[
        r#"OK ((8 8 (8 "if (x) {\n\11value();\n}\n")) (2 2 (2 "if (x) {\n  value();\n}\n")) (0 0 (0 "if (x) {\nvalue();\n}\n")) (-2 -2 (0 "if (x) {\nvalue();\n}\n")))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}
