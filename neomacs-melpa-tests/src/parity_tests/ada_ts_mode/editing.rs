use expect_test::expect;

use super::assert_ada_ts_mode_parity;

#[test]
fn ada_ts_mode_real_defun_comment_box_matches_upstream_nested_resource_case() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "package body Test is\n"
          "   procedure Hello_World is\n"
          "   begin\n"
          "      Put_Line (\"Hello, world!\");\n"
          "   end Hello_World;\n"
          "end Test;\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (goto-char
          (point-min))
         (search-forward
          "Hello, world!")
         (ada-ts-mode-defun-comment-box)
         (list
          (buffer-string)
          (point)))"##;
    let expect = expect![[
        r#"OK ("package body Test is\n   -----------------\n   -- Hello_World --\n   -----------------\n\n   procedure Hello_World is\n   begin\n      Put_Line (\"Hello, world!\");\n   end Hello_World;\nend Test;\n" 153)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_fill_reindent_command_wraps_upstream_comment_shape() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "--------------------------------------------------------------------\n"
          "-- 12345 67890 12345 67890 12345 67890 12345 67890 12345 67890 12345 67890\n"
          "-- 23456 78901 23456 78901 23456 78901 23456 78901 23456 78901\n"
          "-- 34567 89012 34567 89012 34567 89012 34567 89012 34567 89012\n"
          "--------------------------------------------------------------------\n")
         (let ((ada-ts-mode-grammar-install
                nil)
               (fill-column
                70))
           (ada-ts-mode))
         (goto-char
          (point-min))
         (search-forward
          "78901 23456")
         (ada-ts-mode-fill-reindent-defun)
         (list
          (buffer-substring-no-properties
           (point-min)
           (point-max))
          (point)))"##;
    let expect = expect![[
        r#"OK ("--------------------------------------------------------------------\n-- 12345 67890 12345 67890 12345 67890 12345 67890 12345 67890 12345\n-- 67890 23456 78901 23456 78901 23456 78901 23456 78901 23456 78901\n-- 34567 89012 34567 89012 34567 89012 34567 89012 34567 89012\n--------------------------------------------------------------------\n" 165)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_electric_else_completion_reindents_upstream_statement_case() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "procedure Test is\n"
          "begin\n"
          "   if X then\n"
          "      null;\n"
          "      els")
         (let ((ada-ts-mode-grammar-install
                nil)
               (electric-indent-mode
                t))
           (ada-ts-mode))
         (goto-char
          (point-max))
         (save-window-excursion
           (set-window-buffer
            nil
            (current-buffer))
           (execute-kbd-macro
            (kbd
             "e")))
         (list
          (buffer-string)
          (point)
          ada-ts-indent--electric-indent-check-needed))"##;
    let expect =
        expect![[r#"OK ("procedure Test is\nbegin\n   if X then\n      null;\n   else" 57 nil)"#]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_auto_case_minor_mode_toggle_hook_lighter_and_keymap_behavior_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "procedure hello is\nbegin\n   null;\nend hello;\n")
         (let ((ada-ts-mode-grammar-install
                nil)
               events)
           (ada-ts-mode)
           (add-hook
            'ada-ts-auto-case-mode-hook
            (lambda ()
              (push
               ada-ts-auto-case-mode
               events))
            nil
            t)
           (let ((enabled
                  (progn
                    (ada-ts-auto-case-mode
                     1)
                    (list
                     ada-ts-auto-case-mode
                     (assq
                      'ada-ts-auto-case-mode
                      minor-mode-alist)
                     (command-remapping
                      'self-insert-command)
                     (lookup-key
                      ada-ts-auto-case-mode-map
                      (kbd
                       "SPC"))))))
             (ada-ts-auto-case-mode
              -1)
             (list
              enabled
              ada-ts-auto-case-mode
              (nreverse
               events)))))"##;
    let expect = expect![[r#"OK ((t (ada-ts-auto-case-mode " Ada/c") nil nil) nil (t nil))"#]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_electric_keyword_matrix_covers_keyword_and_former_keyword_paths() {
    let elisp_form = r##"(cl-labels
         ((simulate
           (source key)
           (with-temp-buffer
             (insert
              source)
             (let ((ada-ts-mode-grammar-install
                    nil)
                   (electric-indent-mode
                    t))
               (ada-ts-mode))
             (goto-char
              (point-max))
             (save-window-excursion
               (set-window-buffer
                nil
                (current-buffer))
               (execute-kbd-macro
                (kbd
                 key)))
             (buffer-string))))
         (list
          (simulate
           "procedure Test is\n   begi"
           "n")
          (simulate
           "procedure Test is\nbegin\n   null;\n   en"
           "d")
          (simulate
           "procedure Test is\nbegin\n   case X is\n      when 1 =>\n         null;\n         whe"
           "n")
          (simulate
           "package Test is\n   type X is private;\n   privat"
           "e")
          (simulate
           "procedure Test is\nbegin\n   null;\n   exceptio"
           "n")
          (simulate
           "procedure Test is\nbegin\n   if X then\n      null;\n      elsi"
           "f")
          (simulate
           "procedure Test is\nbegin\n   select\n      Do_Something;\n      o"
           "r")
          (simulate
           "procedure Test is\nbegin\n   if X > 2\n        the"
           "n")
          (simulate
           "procedure Test is\nbegin"
           "n")
          (simulate
           "package Test is\nprivate"
           "e")))"##;
    let expect = expect![[
        r#"OK ("procedure Test is\nbegin" "procedure Test is\nbegin\n   null;\nend" "procedure Test is\nbegin\n   case X is\n      when 1 =>\n         null;\n      when" "package Test is\n   type X is private;\nprivate" "procedure Test is\nbegin\n   null;\nexception" "procedure Test is\nbegin\n   if X then\n      null;\n   elsif" "procedure Test is\nbegin\n   select\n      Do_Something;\n   or" "procedure Test is\nbegin\n   if X > 2\n   then" "procedure Test is\n   beginn" "package Test is\n   privatee")"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_electric_punctuation_predicate_covers_end_of_line_and_midline_boundaries() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (insert
              (nth
               0
               case))
             (let ((ada-ts-mode-grammar-install
                    nil))
               (ada-ts-mode))
             (goto-char
              (if
                  (nth
                   1
                   case)
                  (point-max)
                (1-
                 (point-max))))
             (list
              (nth
               2
               case)
              (ada-ts-indent--electric-indent-p)
              (treesit-node-type
               (treesit-node-at
                (1-
                 (point)))))))
         '(("procedure Test is\nbegin\n   null;"
            t
            semicolon-eol)
           ("procedure Test is\nbegin\n   Put (Value)"
            t
            paren-eol)
           ("Values : constant Array_Type := [1, 2]"
            t
            bracket-eol)
           ("Choice : Integer := (if Flag then 1 else 2),"
            t
            comma-eol)
           ("procedure Test is\nbegin\n   null; -- trailing"
            nil
            semicolon-midline)))"##;
    let expect = expect![[
        r#"OK ((semicolon-eol (";" . #1=(")" . #2=("]" "=>" (\, ",")))) ";") (paren-eol #1# ")") (bracket-eol #2# "]") (comma-eol nil ",") (semicolon-midline nil "comment"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_auto_case_real_input_formats_keywords_identifiers_attributes_and_subwords() {
    let elisp_form = r##"(cl-labels
         ((simulate
           (source key)
           (with-temp-buffer
             (insert
              source)
             (let ((ada-ts-mode-grammar-install
                    nil))
               (ada-ts-mode))
             (setq-local
              ada-ts-mode-case-formatting
              '((identifier
                 :formatter upcase-initials
                 :dictionary
                 ("ASCII"
                  "GNAT"
                  "IO"))
                (keyword
                 :formatter downcase)))
             (ada-ts-auto-case-mode
              1)
             (goto-char
              (point-max))
             (save-window-excursion
               (set-window-buffer
                nil
                (current-buffer))
               (execute-kbd-macro
                (kbd
                 key)))
             (buffer-string))))
         (list
          (simulate
           "PROCEDURE hello"
           "SPC")
          (simulate
           "procedure ascii_value"
           "SPC")
          (simulate
           "procedure Test is\n   X : Integer := Integer'ACCESS"
           ";")
          (simulate
           "procedure Test is\nbegin\n   ALL"
           "SPC")
          (simulate
           "procedure Test is\nbegin\n   RETURN"
           "_")))"##;
    let expect = expect![[
        r#"OK ("PROCEDURE Hello " "procedure ASCII_Value " "procedure Test is\n   X : Integer := Integer'ACCESS;" "procedure Test is\nbegin\n   all " "procedure Test is\nbegin\n   RETURN_")"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_case_commands_cover_at_point_explicit_region_and_dwim_region_selection() {
    let elisp_form = r##"(cl-labels
         ((prepare
           ()
           (insert
            "PROCEDURE hello_world IS\n"
            "BEGIN\n"
            "   ascii_value := 1;\n"
            "END hello_world;\n")
           (let ((ada-ts-mode-grammar-install
                  nil))
             (ada-ts-mode))
           (setq-local
            ada-ts-mode-case-formatting
            '((identifier
               :formatter upcase-initials
               :dictionary
               ("ASCII"))
              (keyword
               :formatter downcase)))))
         (list
          (with-temp-buffer
            (prepare)
            (goto-char
             (point-min))
            (search-forward
             "PROCEDURE")
            (ada-ts-mode-case-format-at-point)
            (buffer-string))
          (with-temp-buffer
            (prepare)
            (let ((beg
                   (progn
                     (goto-char
                      (point-min))
                     (search-forward
                      "hello_world")
                     (match-beginning
                      0)))
                  (end
                   (progn
                     (search-forward
                      "ascii_value")
                     (match-end
                      0))))
              (ada-ts-mode-case-format-region
               beg
               end))
            (buffer-string))
          (with-temp-buffer
            (prepare)
            (goto-char
             (point-min))
            (search-forward
             "ascii_value")
            (ada-ts-mode-case-format-dwim)
            (buffer-string))
          (with-temp-buffer
            (prepare)
            (goto-char
             (point-min))
            (push-mark
             (point-max)
             t
             t)
            (let ((transient-mark-mode
                   t))
              (ada-ts-mode-case-format-dwim))
            (buffer-string))))"##;
    let expect = expect![[
        r#"OK ("PROCEDURE hello_world IS\nBEGIN\n   ascii_value := 1;\nEND hello_world;\n" "PROCEDURE Hello_World is\nbegin\n   ASCII_Value := 1;\nEND hello_world;\n" "PROCEDURE hello_world IS\nBEGIN\n   ascii_value := 1;\nEND hello_world;\n" "procedure Hello_World is\nbegin\n   ASCII_Value := 1;\nend Hello_World;\n")"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_fill_reindent_non_comment_branch_indents_complete_defun_and_preserves_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "procedure Reindent is\n"
          "Value : Integer := 1;\n"
          "begin\n"
          "if Value > 0 then\n"
          "Value := Value - 1;\n"
          "end if;\n"
          "end Reindent;\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (goto-char
          (point-min))
         (search-forward
          "Value := Value - 1")
         (let ((point-before
                (point)))
           (ada-ts-mode-fill-reindent-defun)
           (list
            (buffer-string)
            point-before
            (point))))"##;
    let expect = expect![[
        r#"OK ("procedure Reindent is\n   Value : Integer := 1;\nbegin\n   if Value > 0 then\n      Value := Value - 1;\n   end if;\nend Reindent;\n" 87 99)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}
