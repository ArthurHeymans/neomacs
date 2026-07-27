use expect_test::expect;

use super::assert_aiken_mode_parity;

#[test]
fn comment_and_uncomment_region_roundtrip_preserves_aiken_program() {
    let elisp_form = r##"
(with-temp-buffer
  (aiken-mode)
  (insert
   "let amount = 42\n\
expect amount > 0\n\
trace @\"validated\"\n")
  (let ((original (buffer-string)))
    (comment-region (point-min) (point-max))
    (let ((commented (buffer-string)))
      (uncomment-region (point-min) (point-max))
      (list original commented (buffer-string)
            (equal original (buffer-string))))))
"##;
    let expect = expect![[
        r#"OK ("let amount = 42\nexpect amount > 0\ntrace @\"validated\"\n" "// let amount = 42\n// expect amount > 0\n// trace @\"validated\"\n" "let amount = 42\nexpect amount > 0\ntrace @\"validated\"\n" t)"#
    ]];
    assert_aiken_mode_parity(elisp_form, expect);
}

#[test]
fn syntax_parser_tracks_code_line_comments_strings_and_nested_delimiters() {
    let elisp_form = r##"
(with-temp-buffer
  (aiken-mode)
  (insert
   "fn validate(input: Data) {\n\
  let text = \"// not a comment\"\n\
  // real comment with { braces }\n\
  when input is { Constr(fields) -> fields, _ -> [] }\n\
}\n")
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (let* ((position (- (point) (length needle)))
            (state (syntax-ppss position)))
       (list needle (nth 0 state)
             (and (nth 3 state) t)
             (and (nth 4 state) t))))
   '("fn" "// not" "real comment" "when" "fields, _" "}")))
"##;
    let expect = expect![[
        r#"OK (("fn" 0 nil nil) ("// not" 1 t nil) ("real comment" 1 nil t) ("when" 1 nil nil) ("fields, _" 2 nil nil) ("}" 1 nil t))"#
    ]];
    assert_aiken_mode_parity(elisp_form, expect);
}

#[test]
fn underscore_identifiers_move_and_extract_as_single_symbols() {
    let elisp_form = r##"
(with-temp-buffer
  (aiken-mode)
  (insert "let payment_output_reference = own_ref\n")
  (goto-char (point-min))
  (search-forward "payment_")
  (let ((middle (point)))
    (list
     (thing-at-point 'symbol t)
     (progn (goto-char middle) (forward-symbol 1) (point))
     (progn
       (goto-char middle)
       (forward-symbol -1)
       (buffer-substring-no-properties
        (point)
        (progn (forward-symbol 1) (point))))
     (char-syntax ?_))))
"##;
    let expect = expect![[r#"OK ("payment_output_reference" 29 "payment_output_reference" 119)"#]];
    assert_aiken_mode_parity(elisp_form, expect);
}

#[test]
fn balanced_expression_navigation_skips_strings_and_comment_delimiters() {
    let elisp_form = r##"
(with-temp-buffer
  (aiken-mode)
  (insert
   "{ Payment { owner: \"}\" }, // ignored }\n\
  [Some(1), Some(2)] }\n")
  (goto-char (point-min))
  (let* ((start (point))
         (end (scan-sexps start 1))
         (text (buffer-substring-no-properties start end)))
    (goto-char start)
    (forward-sexp 1)
    (list text end (point) (= end (point))
          (char-before end))))
"##;
    let expect = expect![[
        r#"OK ("{ Payment { owner: \"}\" }, // ignored }\n  [Some(1), Some(2)] }" 62 62 t 125)"#
    ]];
    assert_aiken_mode_parity(elisp_form, expect);
}

#[test]
fn inherited_indent_region_produces_stable_tabs_free_editing_result() {
    let elisp_form = r##"
(with-temp-buffer
  (aiken-mode)
  (insert
   "validator spend {\n\
spend(datum: Data) {\n\
when datum is {\n\
Constr(fields) -> True\n\
_ -> False\n\
}\n\
}\n\
}\n")
  (indent-region (point-min) (point-max))
  (list
   (buffer-string)
   (string-match-p "\t" (buffer-string))
   indent-line-function
   indent-tabs-mode))
"##;
    let expect = expect![[
        r#"OK ("validator spend {\nspend(datum: Data) {\nwhen datum is {\nConstr(fields) -> True\n_ -> False\n}\n}\n}\n" nil indent-relative nil)"#
    ]];
    assert_aiken_mode_parity(elisp_form, expect);
}

#[test]
fn newline_and_indent_uses_inherited_prog_mode_behavior_inside_validator() {
    let elisp_form = r##"
(with-temp-buffer
  (aiken-mode)
  (insert "validator spend {")
  (newline-and-indent)
  (insert "spend(datum: Data) {")
  (newline-and-indent)
  (insert "True")
  (newline-and-indent)
  (insert "}")
  (newline-and-indent)
  (insert "}")
  (list
   (buffer-string)
   (mapcar
    (lambda (line)
      (save-excursion
        (goto-char (point-min))
        (forward-line line)
        (current-indentation)))
    '(0 1 2 3 4))))
"##;
    let expect =
        expect![[r#"OK ("validator spend {\nspend(datum: Data) {\nTrue\n}\n}" (0 0 0 0 0))"#]];
    assert_aiken_mode_parity(elisp_form, expect);
}

#[test]
fn comment_filling_changes_only_comment_text_not_neighboring_code() {
    let elisp_form = r##"
(with-temp-buffer
  (aiken-mode)
  (setq fill-column 36)
  (insert
   "// This validator checks that every payment output remains positive and belongs to the expected owner before settlement.\n\
let amount = calculate_payment_amount(transaction)\n")
  (goto-char (point-min))
  (fill-paragraph)
  (list
   (buffer-string)
   (save-excursion
     (goto-char (point-min))
     (forward-line 1)
     (looking-at-p "// "))
   (string-match-p
    "let amount = calculate_payment_amount(transaction)"
    (buffer-string))))
"##;
    let expect = expect![[
        r#"OK ("// This validator checks that every\n// payment output remains positive\n// and belongs to the expected owner\n// before settlement.\nlet amount = calculate_payment_amount(transaction)\n" t 130)"#
    ]];
    assert_aiken_mode_parity(elisp_form, expect);
}

#[test]
fn comment_dwim_appends_and_removes_end_of_line_comment_practically() {
    let elisp_form = r##"
(with-temp-buffer
  (aiken-mode)
  (insert "let amount = 42")
  (goto-char (point-max))
  (comment-dwim nil)
  (insert "positive amount")
  (let ((commented (buffer-string)))
    (goto-char (point-min))
    (search-forward "//")
    (comment-kill nil)
    (list commented (buffer-string) (current-column))))
"##;
    let expect = expect![[
        r#"OK ("let amount = 42                 // positive amount" "let amount = 42" 15)"#
    ]];
    assert_aiken_mode_parity(elisp_form, expect);
}
