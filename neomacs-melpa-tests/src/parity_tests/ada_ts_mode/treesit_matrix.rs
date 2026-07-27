use expect_test::expect;

use super::assert_ada_ts_mode_parity;

#[test]
fn ada_ts_mode_real_defun_matrix_covers_declarations_bodies_tasks_protected_and_generics() {
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
              (point-min))
             (search-forward
              (nth
               1
               case))
             (let ((node
                    (treesit-defun-at-point)))
               (list
                (nth
                 2
                 case)
                (and
                 node
                 (treesit-node-type
                  node))
                (and
                 node
                 (ada-ts-mode--defun-name
                  node
                  'no-property))
                (and
                 node
                 (treesit-node-check
                  node
                  'has-error))))))
         '(("package Example is\nend Example;\n"
            "Example"
            package-declaration)
           ("procedure Run;\n"
            "Run"
            subprogram-declaration)
           ("procedure Stop is null;\n"
            "Stop"
            null-procedure)
           ("function Value return Integer is (42);\n"
            "42"
            expression-function)
           ("package Alias renames Example;\n"
            "Alias"
            package-renaming)
           ("package Instance is new Generic_Package;\n"
            "Instance"
            generic-instantiation)
           ("generic\n   type Item is private;\npackage Containers is\nend Containers;\n"
            "Containers"
            generic-package)
           ("task type Worker is\n   entry Start;\nend Worker;\n"
            "Worker"
            task-type)
           ("task body Worker is\nbegin\n   accept Start;\nend Worker;\n"
            "accept"
            task-body)
           ("protected type Guard is\n   procedure Lock;\nprivate\n   Busy : Boolean;\nend Guard;\n"
            "Guard"
            protected-type)
           ("protected body Guard is\n   procedure Lock is\n   begin\n      null;\n   end Lock;\nend Guard;\n"
            "null"
            protected-body)
           ("separate (Parent)\nprocedure Child is\nbegin\n   null;\nend Child;\n"
            "null"
            subunit)))"##;
    let expect = expect![[
        r#"OK ((package-declaration "package_declaration" "Example" nil) (subprogram-declaration "subprogram_declaration" "Run" nil) (null-procedure "null_procedure_declaration" "Stop" nil) (expression-function "expression_function_declaration" "Value" nil) (package-renaming "package_renaming_declaration" "Alias" nil) (generic-instantiation "generic_instantiation" "Instance" nil) (generic-package "generic_package_declaration" "Containers" nil) (task-type "task_type_declaration" "Worker" nil) (task-body "task_body" "Worker" nil) (protected-type "protected_type_declaration" "Guard" nil) (protected-body "subprogram_body" "Lock" nil) (subunit "subprogram_body" "Child" nil))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_imenu_matrix_includes_protected_task_and_nested_subprogram_categories() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "package body Outer is\n"
          "   protected type Guard is\n"
          "      procedure Lock;\n"
          "   private\n"
          "      Busy : Boolean;\n"
          "   end Guard;\n"
          "\n"
          "   task type Worker is\n"
          "      entry Start;\n"
          "   end Worker;\n"
          "\n"
          "   protected body Guard is\n"
          "      procedure Lock is\n"
          "      begin\n"
          "         null;\n"
          "      end Lock;\n"
          "   end Guard;\n"
          "\n"
          "   task body Worker is\n"
          "   begin\n"
          "      accept Start;\n"
          "   end Worker;\n"
          "end Outer;\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (cl-labels
             ((normalize
               (value)
               (cond
                ((markerp
                  value)
                 (marker-position
                  value))
                ((stringp
                  value)
                 (substring-no-properties
                  value))
                ((consp
                  value)
                 (cons
                  (normalize
                   (car
                    value))
                  (normalize
                   (cdr
                    value))))
                (t
                 value))))
           (normalize
            (funcall
             imenu-create-index-function))))"##;
    let expect = expect![[
        r#"OK (("Package" ("Outer" . 1)) ("Subprogram" ("Outer" ("Guard" ("Lock" . 56)) ("Guard" ("Lock" . 211)))) ("Protected" ("Outer" ("Guard" . 26) ("Guard" . 181))) ("Task" ("Outer" ("Worker" . 123) ("Worker" . 290))) ("Type Declaration" ("Outer" ("Guard" . 26) ("Worker" . 123))))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_indentation_matrix_changes_and_formats_diverse_upstream_constructs() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (with-temp-buffer
             (insert
              source)
             (let ((ada-ts-mode-grammar-install
                    nil))
               (ada-ts-mode))
             (setq-local
              indent-tabs-mode
              nil)
             (setq-local
              treesit-simple-indent-rules
              (copy-tree
               treesit-simple-indent-rules))
             (let* ((language-rules
                     (cdr
                      (assq
                       'ada
                       treesit-simple-indent-rules)))
                    (catch-all-rule
                     (car
                      (last
                       language-rules))))
               (unless
                   (eq
                    (car
                     catch-all-rule)
                    'catch-all)
                 (error
                  "Expected final Ada indentation rule to be catch-all"))
               (setcar
                (cdr
                 catch-all-rule)
                (lambda (&rest _)
                  (error
                   "Formal indentation matrix reached catch-all anchor for %s"
                   source)))
               (setcar
                (cddr
                 catch-all-rule)
                (lambda (&rest _)
                  (error
                   "Formal indentation matrix reached catch-all offset for %s"
                   source))))
             (let ((before
                    (buffer-string)))
               (indent-region
                (point-min)
                (point-max))
               (list
                (not
                 (equal
                  before
                  (buffer-string)))
                (buffer-string)))))
         '("package Records is\ntype Pair is record\nLeft : Integer;\nRight : Integer;\nend record;\nend Records;\n"
           "procedure Handle is\nbegin\nnull;\nexception\nwhen Constraint_Error =>\nnull;\nwhen others =>\nnull;\nend Handle;\n"
           "procedure Block_Test is\nbegin\ndeclare\nValue : Integer := 1;\nbegin\nValue := Value + 1;\nend;\nend Block_Test;\n"
           "procedure Select_Test is\nbegin\nselect\naccept Start;\nor\ndelay 1.0;\nelse\nnull;\nend select;\nend Select_Test;\n"
           "function All_Positive return Boolean is\n(for all Item of Values => Item > 0);\n"
           "Result : constant Pair :=\n(Left => 1,\nRight => 2);\n"
           "procedure Contracted (Value : Integer)\nwith Pre => Value > 0,\nPost => Value = Value'Old;\n"
           ))"##;
    let expect = expect![[
        r#"OK ((t "package Records is\n   type Pair is record\n      Left : Integer;\n      Right : Integer;\n   end record;\nend Records;\n") (t "procedure Handle is\nbegin\n   null;\nexception\n   when Constraint_Error =>\n      null;\n   when others =>\n      null;\nend Handle;\n") (t "procedure Block_Test is\nbegin\n   declare\n      Value : Integer := 1;\n   begin\n      Value := Value + 1;\n   end;\nend Block_Test;\n") (t "procedure Select_Test is\nbegin\n   select\n      accept Start;\n   or\n      delay 1.0;\n   else\n      null;\n   end select;\nend Select_Test;\n") (t "function All_Positive return Boolean is\n  (for all Item of Values => Item > 0);\n") (t "Result : constant Pair :=\n  (Left => 1,\n   Right => 2);\n") (t "procedure Contracted (Value : Integer)\n  with Pre => Value > 0,\n       Post => Value = Value'Old;\n"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_real_font_lock_matrix_covers_types_attributes_labels_calls_and_preprocessor() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "#if DEBUG then\n"
          "with Ada.Text_IO;\n"
          "#end if;\n"
          "procedure Matrix is\n"
          "   type Color is (Red, Green, Blue);\n"
          "   type Pair is record\n"
          "      Left  : Integer;\n"
          "      Right : Integer;\n"
          "   end record;\n"
          "   Value : Integer := Integer'First;\n"
          "   Label_Name : constant String := \"label\";\n"
          "begin\n"
          "   <<Again>>\n"
          "   Value := Value + 1;\n"
          "   Ada.Text_IO.Put_Line (Label_Name);\n"
          "   if Value in 1 .. 10 then\n"
          "      goto Again;\n"
          "   end if;\n"
          "exception\n"
          "   when Constraint_Error | Program_Error =>\n"
          "      raise Storage_Error;\n"
          "end Matrix;\n")
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
               (setq
                position
                next)))
           (nreverse
            runs)))"##;
    let expect = expect![[
        r##"OK (("#if" font-lock-preprocessor-face) ("then" font-lock-preprocessor-face) ("Ada" font-lock-warning-face) ("." font-lock-delimiter-face) ("Text_IO" (font-lock-function-call-face)) (";" font-lock-delimiter-face) ("#end" font-lock-preprocessor-face) ("if;" font-lock-preprocessor-face) ("procedure" font-lock-keyword-face) ("Matrix" (font-lock-function-name-face)) ("is" font-lock-keyword-face) ("type" font-lock-keyword-face) ("Color" font-lock-type-face) ("is" font-lock-keyword-face) ("(" font-lock-bracket-face) ("Red" font-lock-constant-face) ("," font-lock-delimiter-face) ("Green" font-lock-constant-face) ("," font-lock-delimiter-face) ("Blue" font-lock-constant-face) (")" font-lock-bracket-face) (";" font-lock-delimiter-face) ("type" font-lock-keyword-face) ("Pair" font-lock-type-face) ("is" font-lock-keyword-face) ("record" font-lock-keyword-face) ("Left" font-lock-property-name-face) (":" font-lock-delimiter-face) ("Integer" font-lock-type-face) (";" font-lock-delimiter-face) ("Right" font-lock-property-name-face) (":" font-lock-delimiter-face) ("Integer" font-lock-type-face) (";" font-lock-delimiter-face) ("end" font-lock-keyword-face) ("record" font-lock-keyword-face) (";" font-lock-delimiter-face) ("Value" (font-lock-variable-name-face)) (":" font-lock-delimiter-face) ("Integer" font-lock-type-face) (":=" (font-lock-operator-face)) ("First" font-lock-property-use-face) (";" font-lock-delimiter-face) ("Label_Name" (font-lock-constant-face font-lock-variable-name-face)) (":" font-lock-delimiter-face) ("constant" font-lock-keyword-face) ("String" font-lock-type-face) (":=" (font-lock-operator-face)) ("\"label\"" font-lock-string-face) (";" font-lock-delimiter-face) ("begin" font-lock-keyword-face) ("<<" (font-lock-operator-face)) ("Again" font-lock-constant-face) (">>" (font-lock-operator-face)) ("Value" font-lock-variable-use-face) (":=" (font-lock-operator-face)) ("+" (font-lock-operator-face)) ("1" font-lock-number-face) (";" font-lock-delimiter-face) ("." font-lock-delimiter-face) ("." font-lock-delimiter-face) ("Put_Line" (font-lock-function-call-face)) ("(" font-lock-bracket-face) (")" font-lock-bracket-face) (";" font-lock-delimiter-face) ("if" font-lock-keyword-face) ("in" (font-lock-operator-face font-lock-keyword-face)) ("1" font-lock-number-face) (".." (font-lock-operator-face)) ("10" font-lock-number-face) ("then" font-lock-keyword-face) ("goto" (font-lock-operator-face font-lock-keyword-face)) ("Again" font-lock-constant-face) (";" font-lock-delimiter-face) ("end" font-lock-keyword-face) ("if" font-lock-keyword-face) (";" font-lock-delimiter-face) ("exception" font-lock-keyword-face) ("when" font-lock-keyword-face) ("Constraint_Error" font-lock-type-face) ("|" (font-lock-operator-face)) ("Program_Error" font-lock-type-face) ("=>" (font-lock-operator-face)) ("raise" (font-lock-operator-face font-lock-keyword-face)) ("Storage_Error" (font-lock-type-face)) (";" font-lock-delimiter-face) ("end" font-lock-keyword-face) ("Matrix" (font-lock-function-name-face)) (";" font-lock-delimiter-face))"##
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}
