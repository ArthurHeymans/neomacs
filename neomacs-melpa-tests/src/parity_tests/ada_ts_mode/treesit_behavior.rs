use expect_test::expect;

use super::assert_ada_ts_mode_parity;

#[test]
fn ada_ts_mode_initializes_real_parser_and_complete_buffer_local_mode_state() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "procedure Hello is\nbegin\n   null;\nend Hello;\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (list
          major-mode
          mode-name
          (mapcar
           #'treesit-parser-language
           (treesit-parser-list))
          comment-start
          comment-end
          comment-start-skip
          syntax-propertize-function
          treesit-defun-name-function
          imenu-create-index-function
          indent-line-function
          indent-region-function
          treesit-outline-predicate
          editorconfig-indent-size-vars
          ff-other-file-alist
          treesit-font-lock-feature-list
          (local-variable-p
           'treesit-simple-indent-rules)
          (memq
           'ada-ts-indent--after-change
           after-change-functions)
          (memq
           'ada-ts-indent--maybe-electric-indent
           post-command-hook)))"##;
    let expect = expect![[
        r#"OK (ada-ts-mode "Ada" (ada) "--" "" "---*\\s-*" ada-ts-mode--syntax-propertize ada-ts-mode--defun-name ada-ts-imenu ada-ts-mode--indent-line ada-ts-mode--indent-region ada-ts-mode--defun-p (ada-ts-mode-indent-offset) ada-ts-mode-other-file-alist ((comment definition) (keyword preprocessor string type) (attribute assignment constant control function number operator) (bracket delimiter error label)) t (ada-ts-indent--after-change t) (ada-ts-indent--maybe-electric-indent t))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_parser_tree_shape_matches_for_representative_compilation_unit() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "with Ada.Text_IO;\n"
          "procedure Hello is\n"
          "   Count : Integer := 2;\n"
          "begin\n"
          "   if Count > 0 then\n"
          "      Ada.Text_IO.Put_Line (\"Hello\");\n"
          "   end if;\n"
          "end Hello;\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (let ((root
                (treesit-buffer-root-node
                 'ada)))
           (list
            (treesit-node-type
             root)
            (treesit-node-start
             root)
            (treesit-node-end
             root)
            (treesit-node-child-count
             root)
            (treesit-node-check
             root
             'has-error)
            (treesit-node-string
             root))))"##;
    let expect = expect![[
        r#"OK ("compilation" 1 150 2 nil "(compilation (compilation_unit (with_clause (selected_component prefix: (identifier) selector_name: (identifier)))) (compilation_unit (subprogram_body (procedure_specification name: (identifier)) (non_empty_declarative_part (object_declaration name: (identifier) subtype_mark: (identifier) (expression (term (numeric_literal))))) (handled_sequence_of_statements (if_statement condition: (expression (term name: (identifier)) (relational_operator) (term (numeric_literal))) statements: (procedure_call_statement name: (selected_component prefix: (selected_component prefix: (identifier) selector_name: (identifier)) selector_name: (identifier)) (actual_parameter_part (parameter_association (expression (term name: (string_literal)))))))) endname: (identifier))))")"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_tree_sitter_defun_navigation_and_names_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "package body Outer is\n"
          "   procedure First is\n"
          "   begin\n"
          "      null;\n"
          "   end First;\n"
          "\n"
          "   function Second return Integer is (2);\n"
          "end Outer;\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (mapcar
          (lambda (needle)
            (goto-char
             (point-min))
            (search-forward
             needle)
            (let ((node
                   (treesit-defun-at-point)))
              (list
               needle
               (treesit-node-type
                node)
               (treesit-node-start
                node)
               (treesit-node-end
                node)
               (ada-ts-mode--defun-name
                node
                'no-property))))
          '("null"
            "(2)"
            "end Outer")))"##;
    let expect = expect![[
        r#"OK (("null" "subprogram_body" 26 79 "First") ("(2)" "expression_function_declaration" 84 122 "Second") ("end Outer" "package_body" 1 133 "Outer"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_imenu_categories_nesting_names_and_markers_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "with Ada.Text_IO;\n"
          "package body Outer is\n"
          "   type Counter is range 0 .. 10;\n"
          "   procedure First is\n"
          "   begin\n"
          "      null;\n"
          "   end First;\n"
          "end Outer;\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (cl-labels
             ((normalize
               (item)
               (cond
                ((markerp item)
                 (marker-position
                  item))
                ((consp item)
                 (cons
                  (normalize
                   (car item))
                  (normalize
                   (cdr item))))
                (t item))))
           (normalize
            (funcall
             imenu-create-index-function))))"##;
    let expect = expect![[
        r#"OK (("Package" (#("Outer" 0 5 (face (:foreground #1="unspecified-fg"))) . 19)) ("Subprogram" (#("Outer" 0 5 (face (:foreground #1#))) (#("First" 0 5 (face (font-lock-function-name-face))) . 78))) ("Type Declaration" (#("Outer" 0 5 (face (:foreground #1#))) (#("Counter" 0 7 (face font-lock-type-face)) . 44))) ("With Clause" (#("Ada.Text_IO" 0 3 (face (:foreground #1#)) 4 11 (face (:foreground #1#))) . 6)))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_tree_sitter_indentation_matches_across_constructs() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (with-temp-buffer
             (insert source)
             (let ((ada-ts-mode-grammar-install
                    nil))
               (ada-ts-mode))
             (setq-local
              indent-tabs-mode
              nil)
             (indent-region
              (point-min)
              (point-max))
             (buffer-string)))
         '("procedure Hello is\nX : Integer := 1;\nbegin\nif X > 0 then\nX := X - 1;\nelse\nX := 0;\nend if;\nend Hello;\n"
           "package body Outer is\nprocedure Run is\nbegin\ncase Value is\nwhen 1 =>\nnull;\nwhen others =>\nnull;\nend case;\nend Run;\nend Outer;\n"
           "procedure Loops is\nbegin\nfor Index in 1 .. 3 loop\nif Index = 2 then\nexit;\nend if;\nend loop;\nend Loops;\n"))"##;
    let expect = expect![[
        r#"OK ("procedure Hello is\n   X : Integer := 1;\nbegin\n   if X > 0 then\n      X := X - 1;\n   else\n      X := 0;\n   end if;\nend Hello;\n" "package body Outer is\n   procedure Run is\n   begin\n      case Value is\n         when 1 =>\n            null;\n         when others =>\n            null;\n      end case;\n   end Run;\nend Outer;\n" "procedure Loops is\nbegin\n   for Index in 1 .. 3 loop\n      if Index = 2 then\n         exit;\n      end if;\n   end loop;\nend Loops;\n")"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_tree_sitter_case_formatting_matches_keywords_and_identifiers() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "PROCEDURE hello_world IS\n"
          "   ascii_value : INTEGER := 1;\n"
          "BEGIN\n"
          "   IF ascii_value > 0 THEN\n"
          "      NULL;\n"
          "   END IF;\n"
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
             :formatter downcase)))
         (ada-ts-mode-case-format-buffer)
         (list
          (buffer-string)
          (point)
          (buffer-modified-p)))"##;
    let expect = expect![[
        r#"OK ("procedure Hello_World is\n   ASCII_Value : INTEGER := 1;\nbegin\n   if ASCII_Value > 0 then\n      null;\n   end if;\nend Hello_World;\n" 130 t)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_tree_sitter_font_lock_faces_match_strictly() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "procedure Hello (Value : in Integer) is\n"
          "   Count : constant Integer := 16#FF#;\n"
          "begin\n"
          "   Ada.Text_IO.Put_Line (\"Hello\"); -- note\n"
          "   raise Constraint_Error;\n"
          "end Hello;\n")
         (let ((ada-ts-mode-grammar-install
                nil)
               (treesit-font-lock-level
                4))
           (ada-ts-mode))
         (font-lock-ensure
          (point-min)
          (point-max))
         (let ((position
                (point-min))
               runs)
           (while (< position
                     (point-max))
             (let* ((next
                     (next-single-property-change
                      position
                      'face
                      nil
                      (point-max)))
                    (face
                     (get-text-property
                      position
                      'face)))
               (when face
                 (push
                  (list
                   (buffer-substring-no-properties
                    position
                    next)
                   face)
                  runs))
               (setq position
                     next)))
           (nreverse
            runs)))"##;
    let expect = expect![[
        r#"OK (("procedure" font-lock-keyword-face) ("Hello" (font-lock-function-name-face)) ("(" font-lock-bracket-face) ("Value" (font-lock-constant-face font-lock-variable-name-face)) (":" font-lock-delimiter-face) ("in" font-lock-keyword-face) ("Integer" font-lock-type-face) (")" font-lock-bracket-face) ("is" font-lock-keyword-face) ("Count" (font-lock-constant-face font-lock-variable-name-face)) (":" font-lock-delimiter-face) ("constant" font-lock-keyword-face) ("Integer" font-lock-type-face) (":=" (font-lock-operator-face)) ("16#FF#" font-lock-number-face) (";" font-lock-delimiter-face) ("begin" font-lock-keyword-face) ("." font-lock-delimiter-face) ("." font-lock-delimiter-face) ("Put_Line" (font-lock-function-call-face)) ("(" font-lock-bracket-face) ("\"Hello\"" font-lock-string-face) (")" font-lock-bracket-face) (";" font-lock-delimiter-face) ("-- note" font-lock-comment-face) ("raise" (font-lock-operator-face font-lock-keyword-face)) ("Constraint_Error" (font-lock-type-face)) (";" font-lock-delimiter-face) ("end" font-lock-keyword-face) ("Hello" (font-lock-function-name-face)) (";" font-lock-delimiter-face))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_character_literal_string_and_comment_syntax_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "Quote : Character := '\"';\n"
          "Text : String := \"Ada\"; -- note\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (syntax-propertize
          (point-max))
         (mapcar
          (lambda (needle)
            (goto-char
             (point-min))
            (search-forward
             needle)
            (let ((state
                   (syntax-ppss
                    (1-
                     (point)))))
              (list
               needle
               (nth 3 state)
               (nth 4 state)
               (nth 8 state)
               (get-text-property
                (match-beginning 0)
                'syntax-table))))
          '("'\"'"
            "Ada"
            "note")))"##;
    let expect =
        expect![[r#"OK (("'\"'" 39 nil 22 (7)) ("Ada" 34 nil 44 nil) ("note" nil t 51 nil))"#]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_automatic_file_registration_selects_mode_with_real_grammar() {
    let elisp_form = r##"(mapcar
         (lambda (filename)
           (with-temp-buffer
             (setq
              buffer-file-name
              filename)
             (insert
              "procedure Sample is begin null; end Sample;")
             (let ((ada-ts-mode-grammar-install
                    nil))
               (set-auto-mode))
             (list
              filename
              major-mode
              mode-name
              (mapcar
               #'treesit-parser-language
               (treesit-parser-list)))))
         '("/workspace/sample.ada"
           "/workspace/sample.adb"
           "/workspace/sample.ads"
           "/workspace/sample.adc"))"##;
    let expect = expect![[
        r#"OK (("/workspace/sample.ada" ada-ts-mode "Ada" (ada)) ("/workspace/sample.adb" ada-ts-mode "Ada" (ada)) ("/workspace/sample.ads" ada-ts-mode "Ada" (ada)) ("/workspace/sample.adc" ada-ts-mode "Ada" (ada)))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}
