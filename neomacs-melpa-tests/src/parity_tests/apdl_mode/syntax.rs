use expect_test::expect;

use super::assert_apdl_mode_parity;

#[test]
fn syntax_table_classifies_apdl_comments_strings_symbols_words_and_operators() {
    let elisp_form = r##"(with-syntax-table apdl-mode-syntax-table
  (mapcar
   (lambda (character)
     (list character (char-syntax character)))
   '(?! ?\n ?' ?" ?_ ?: ?~ ?` ?$ ?+ ?- ?= ?> ?< ?. ?% ?| ?* ?/)))"##;
    let expect = expect![
        "OK ((33 60) (10 62) (39 34) (34 119) (95 95) (58 95) (126 95) (96 119) (36 46) (43 46) (45 46) (61 46) (62 46) (60 46) (46 46) (37 46) (124 46) (42 46) (47 46))"
    ];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn syntax_parser_distinguishes_real_inline_comments_quoted_paths_and_double_quotes() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "job = 'cantilever model' ! solve the production case\n"
   "/title,\"double quotes are APDL words\"\n")
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (let ((state (syntax-ppss)))
       (list needle
             (nth 3 state)
             (nth 4 state)
             (apdl-in-string-p)
             (apdl-in-comment-p)
             (apdl-in-string-or-comment-p)
             (apdl-not-in-string-or-comment-p))))
   '("cantilever" "production" "double quotes")))"##;
    let expect = expect![[
        r#"OK (("cantilever" 39 nil 39 nil 39 nil) ("production" nil t nil t t nil) ("double quotes" nil nil nil nil nil t))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn line_predicates_classify_code_comments_defaults_numbers_formats_and_condensed_input() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "! heading\n"
   "\n"
   "n,1,0,0,0 $ n,2,1,0,0 ! condensed\n"
   ",steel\n"
   "*vwrite,node,x\n"
   "(I8,2F12.5)\n"
   "1,0.0,0.0,0.0\n")
  (let (rows)
    (goto-char (point-min))
    (dotimes (_ 7)
      (push
       (list
        (line-number-at-pos)
        (apdl-code-line-p)
        (apdl-not-in-code-line-p)
        (apdl-default-command-p)
        (apdl-number-line-p)
        (apdl-in-format-command-line-p)
        (apdl-in-format-construct-p)
        (apdl-condensed-input-line-p)
        (apdl-continuation-line-p))
       rows)
      (forward-line))
    (nreverse rows)))"##;
    let expect = expect![
        "OK ((1 nil t nil nil nil nil nil nil) (2 nil t nil nil nil nil nil nil) (3 t nil nil nil nil nil t nil) (4 t nil t nil nil nil nil nil) (5 t nil nil nil t nil nil nil) (6 t nil nil nil nil t nil nil) (7 nil t nil t nil nil nil nil))"
    ];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn point_predicates_handle_indentation_first_last_and_code_boundaries_in_context() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert "  et,1,solid186 ! quadratic element\n\nfinish")
  (list
   (progn
     (goto-char (point-min))
     (list (apdl-first-line-p) (apdl-last-line-p)
           (apdl-in-indentation-p) (apdl-at-end-of-text-p)
           (apdl-at-end-of-code-p)))
   (progn
     (move-to-column 2)
     (list (apdl-in-indentation-p) (apdl-at-end-of-code-p)))
   (progn
     (search-forward "solid186")
     (list (apdl-in-indentation-p) (apdl-at-end-of-code-p)))
   (progn
     (search-forward "!")
     (list (apdl-at-end-of-code-p) (apdl-in-comment-p)))
   (progn
     (goto-char (point-max))
     (list (apdl-first-line-p) (apdl-last-line-p)
           (apdl-at-end-of-text-p) (apdl-at-end-of-code-p)))))"##;
    let expect = expect!["OK ((t nil t nil nil) (t nil) (nil t) (nil t) (nil t t t))"];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn asterisk_comments_and_string_command_detection_ignore_lookalikes_in_normal_code() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "n,1,0,0,0 * this is an asterisk comment\n"
   "*msg,info\n"
   "The result is %value%\n"
   "force = 3 * area\n")
  (list
   (progn
     (goto-char (point-min))
     (search-forward "asterisk")
     (apdl-in-asterisk-comment-p))
   (progn
     (forward-line)
     (apdl-in-string-command-line-p))
   (progn
     (forward-line)
     (apdl-in-format-construct-p))
   (progn
     (forward-line)
     (search-forward "area")
     (list (apdl-in-asterisk-comment-p)
           (apdl-in-string-command-line-p)
           (apdl-in-format-construct-p)))))"##;
    let expect = expect!["OK (t nil t (t nil nil))"];
    assert_apdl_mode_parity(elisp_form, expect);
}
