use expect_test::expect;

use super::assert_ado_mode_parity;

#[test]
fn ado_mode_comment_stripping_covers_line_continuation_nested_and_error_cases() {
    let elisp_form = r##"(mapcar
         (lambda (input)
           (condition-case error-data
               (ado-strip-comments input)
             (error (list 'signal (car error-data) (cdr error-data)))))
         '("display  one   two"
           "display 1 // ignored\ndisplay 2"
           "display 1 /// continued\ndisplay 2"
           "display 1 /* outer /* nested */ rest */ display 2"
           "/* whole */display 3"
           "display \"//inside\" // trailing"
           "display 1 */"
           "display 1 /* unfinished"
           "display 1 ///"))"##;
    let expect = expect![[
        r#"OK ("display one two" "display 1\ndisplay 2" "display 1display 2" "display 1  display 2" "display 3" "display \"//inside\"" (signal error ("Too many */ in a /* */-style comment")) (signal error ("Too many /* in a /* */-style comment")) (signal error ("Found /// with no continuation")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_semicolon_eol_and_trim_transformations_cover_boundaries() {
    let elisp_form = r##"(list
         (mapcar #'ado-convert-semicolons
                 '("one;two;" "one\ntwo;three" ";a;;b" "plain"))
         (mapcar #'ado-one-eol '("one" "one\n" "one\n\n"))
         (mapcar #'ado-string-trim-left
                 '(" \t\nleft " "clean" "" " \t"))
         (mapcar #'ado-string-trim-right
                 '(" right \t\n" "clean" "" "\r "))
         (mapcar #'ado-string-trim
                 '(" \t both \r\n" "clean" "" " \n\t ")))"##;
    let expect = expect![[
        r#"OK (("one\ntwo\n" "one two\nthree" "\na\n\nb" "plain") ("one\n" "one\n" "one\n\n") ("left " "clean" "" "") (" right" "clean" "" "") ("both" "clean" "" ""))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_grab_something_covers_region_word_buffer_commands_prefixes_and_errors() {
    let elisp_form = r##"(with-temp-buffer
         (setq ado-add-sysdir-font-lock nil
               ado-mode-home "/virtual/ado-mode/"
               ado-site-template-dir "/virtual/templates/"
               ado-script-dir "/virtual/scripts/"
               ado-new-dir "/virtual/new/"
               ado-personal-dir "/virtual/personal/")
         (insert "quietly: regress mpg weight\nsummarize price\n")
         (ado-mode)
         (let (results)
           (goto-char (point-min))
           (search-forward "regress")
           (push (ado-grab-something nil) results)
           (push (ado-grab-something 0) results)
           (push (ado-grab-something -1) results)
           (push (ado-grab-something -2) results)
           (goto-char (point-min))
           (set-mark (point))
           (search-forward "quietly")
           (setq mark-active t)
           (push (ado-grab-something -2) results)
           (setq mark-active nil)
           (push
            (condition-case error-data
                (ado-grab-something 1)
              (error (list 'signal (car error-data) (cdr error-data))))
            results)
           (nreverse results)))"##;
    let expect = expect![[
        r#"OK ("regress" "regress" "quietly: regress mpg weight" "quietly: regress mpg weight\nsummarize price\n" "quietly" (signal error ("`ado-grab-something': argument must be nil, 0, -1, or -2")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_command_to_clip_covers_command_dofile_delimit_whitespace_and_empty_paths() {
    let elisp_form = r##"(let* (clipboard
                (interprogram-cut-function
                 (lambda (value) (push value clipboard) value)))
         (cl-letf (((symbol-function 'ado-delimit-is-semi-p)
                    (lambda () ado-test-semi)))
           (let (results)
             (dolist
                 (case
                  '((nil "command" nil nil "  display  1 // note\n")
                    (nil "command" t nil "  display 1;\n display 2;  ")
                    (t "command" t nil "display 1;\ndisplay 2;")
                    (t "dofile" t nil "display 1;\ndisplay 2;")
                    (nil "include" t t "  display 1  ")))
               (with-temp-buffer
                 (setq ado-add-sysdir-font-lock nil
                       ado-mode-home "/virtual/ado-mode/"
                       ado-site-template-dir "/virtual/templates/"
                       ado-script-dir "/virtual/scripts/"
                       ado-new-dir "/virtual/new/"
                       ado-personal-dir "/virtual/personal/")
                 (insert (nth 4 case))
                 (ado-mode)
                 (goto-char (point-min))
                 (setq ado-test-semi (nth 0 case))
                 (push
                  (condition-case error-data
                      (ado-command-to-clip
                       (nth 1 case) (nth 2 case) (nth 3 case))
                    (error (list 'signal (car error-data) (cdr error-data))))
                  results)))
             (with-temp-buffer
               (setq ado-add-sysdir-font-lock nil
                     ado-mode-home "/virtual/ado-mode/"
                     ado-site-template-dir "/virtual/templates/"
                     ado-script-dir "/virtual/scripts/"
                     ado-new-dir "/virtual/new/"
                     ado-personal-dir "/virtual/personal/")
               (ado-mode)
               (setq ado-test-semi nil)
               (push
                (condition-case error-data
                    (ado-command-to-clip "command" t)
                  (error (list 'signal (car error-data) (cdr error-data))))
                results))
             (list (nreverse results) (nreverse clipboard)))))"##;
    let expect = expect![[
        r##"OK (("display 1" "display 1;\n display 2;" "display 1\n display 2\n" "#delimit ;\ndisplay 1;\ndisplay 2;" "  display 1  " (signal error ("Buffer is empty"))) ("display 1" "display 1;\n display 2;" "display 1\n display 2\n" "#delimit ;\ndisplay 1;\ndisplay 2;" "  display 1  "))"##
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_other_clip_and_help_wrappers_preserve_prefix_suffix_and_dispatch() {
    let elisp_form = r##"(let* ((clipboard nil)
                (messages nil)
                (grabs nil)
                (interprogram-cut-function
                 (lambda (value) (push value clipboard) value)))
         (cl-letf (((symbol-function 'ado-grab-something)
                    (lambda (where)
                      (push where grabs)
                      (if (null where) "word" "command")))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (let ((value (apply #'format format-string arguments)))
                        (push value messages)
                        value))))
           (list
            (ado-other-to-clip nil nil nil)
            (ado-other-to-clip 0 "help" "now")
            (ado-help-at-point-to-clip)
            (ado-help-command-to-clip)
            (nreverse clipboard)
            (nreverse messages)
            (nreverse grabs))))"##;
    let expect = expect![[
        r#"OK ("word" "help command now" "help word" "help command" ("word" "help command now" "help word" "help command") ("word" "help command now" "help word" "help command") (nil nil 0 0 nil nil 0 0))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}
