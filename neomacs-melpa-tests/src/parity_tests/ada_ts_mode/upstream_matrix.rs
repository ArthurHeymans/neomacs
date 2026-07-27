use expect_test::expect;

use super::assert_ada_ts_mode_parity;

#[test]
fn ada_ts_mode_upstream_defun_inventory_resolves_every_supported_node_type_and_name() {
    let elisp_form = r##"(progn
         (require
          'which-func)
         (mapcar
         (lambda (case)
           (with-temp-buffer
             (insert
              (nth
               1
               case))
             (let ((ada-ts-mode-grammar-install
                    nil))
               (ada-ts-mode))
             (goto-char
              (point-min))
             (search-forward
              (nth
               2
               case))
             (let* ((node
                     (treesit-defun-at-point))
                    (actual-type
                     (and
                      node
                      (treesit-node-type
                       node)))
                    (actual-name
                     (which-function)))
               (list
                (nth
                 0
                 case)
                (nth
                 3
                 case)
                actual-type
                (equal
                 (nth
                  3
                  case)
                 actual-type)
                (nth
                 4
                 case)
                actual-name
                (equal
                 (nth
                  4
                  case)
                 actual-name)))))
         '((broken-package
            "package\n  XYZ\n  .\n  ABC\nis\nend\n  XYZ\n  .\n  ABC\n;\n"
            "ABC"
            "package_declaration"
            "XYZ.ABC")
           (entry-body
            "package body Test is\n   protected body XYZ is\n      entry ABC\n        when X > Y\n      is\n      begin\n         null;\n      end ABC;\n   end XYZ;\nend Test;\n"
            "entry ABC"
            "entry_body"
            "Test.XYZ.ABC")
           (entry-declaration
            "package body Test is\n   protected type XYZ is\n      entry ABC;\n   end XYZ;\nend Test;\n"
            "entry ABC"
            "entry_declaration"
            "Test.XYZ.ABC")
           (expression-function
            "package Test is\n   function ABC (X : Integer) return Integer is (X + 1);\nend Test;\n"
            "function ABC"
            "expression_function_declaration"
            "Test.ABC")
           (formal-abstract-subprogram
            "generic\n   with procedure DEF is abstract;\npackage ABC is\nend ABC;\n"
            "procedure DEF"
            "formal_abstract_subprogram_declaration"
            "ABC.DEF")
           (formal-concrete-subprogram
            "generic\n   with procedure DEF;\npackage ABC is\nend ABC;\n"
            "procedure DEF"
            "formal_concrete_subprogram_declaration"
            "ABC.DEF")
           (formal-package
            "generic\n   with package DEF is new GHI;\npackage ABC is\nend ABC;\n"
            "package DEF"
            "formal_package_declaration"
            "ABC.DEF")
           (generic-instantiation
            "package ABC is new GHI;\n"
            "package ABC"
            "generic_instantiation"
            "ABC")
           (generic-package
            "generic\npackage ABC is\nend ABC;\n"
            "package ABC"
            "generic_package_declaration"
            "ABC")
           (generic-renaming
            "generic package ABC renames GHI;\n"
            "package ABC"
            "generic_renaming_declaration"
            "ABC")
           (generic-subprogram
            "generic\nprocedure ABC;\n"
            "procedure ABC"
            "generic_subprogram_declaration"
            "ABC")
           (null-procedure
            "package Test is\n   procedure ABC is null;\nend Test;\n"
            "procedure ABC"
            "null_procedure_declaration"
            "Test.ABC")
           (package-body
            "package body ABC is\nbegin\n   null;\nend ABC;\n"
            "package body ABC"
            "package_body"
            "ABC")
           (package-body-stub
            "package body Test is\n   package body ABC is separate;\nend Test;\n"
            "package body ABC"
            "package_body_stub"
            "Test.ABC")
           (package-declaration
            "package ABC is\nend ABC;\n"
            "package ABC"
            "package_declaration"
            "ABC")
           (package-renaming
            "package ABC renames DEF.GHI;\n"
            "package ABC"
            "package_renaming_declaration"
            "ABC")
           (protected-body
            "package body Test is\n   protected body ABC is\n   end ABC;\nend Test;\n"
            "protected body ABC"
            "protected_body"
            "Test.ABC")
           (protected-body-stub
            "package body Test is\n   protected body ABC is separate;\nend Test;\n"
            "protected body ABC"
            "protected_body_stub"
            "Test.ABC")
           (protected-type
            "package Test is\n   protected type ABC is\n   end ABC;\nend Test;\n"
            "protected type ABC"
            "protected_type_declaration"
            "Test.ABC")
           (single-protected
            "package Test is\n   protected ABC is\n   end ABC;\nend Test;\n"
            "protected ABC"
            "single_protected_declaration"
            "Test.ABC")
           (single-task
            "package Test is\n   task ABC;\nend Test;\n"
            "task ABC"
            "single_task_declaration"
            "Test.ABC")
           (subprogram-body
            "procedure ABC is\nbegin\n   null;\nend ABC;\n"
            "procedure ABC"
            "subprogram_body"
            "ABC")
           (subprogram-body-stub
            "package body Test is\n   procedure ABC is separate;\nend Test;\n"
            "procedure ABC"
            "subprogram_body_stub"
            "Test.ABC")
           (subprogram-declaration
            "procedure ABC;\n"
            "procedure ABC"
            "subprogram_declaration"
            "ABC")
           (subprogram-renaming
            "procedure ABC renames DEF.GHI;\n"
            "procedure ABC"
            "subprogram_renaming_declaration"
            "ABC")
           (subunit
            "separate (XYZ)\nprocedure ABC is\nbegin\n   null;\nend ABC;\n"
            "separate (XYZ)"
            "subunit"
            "XYZ")
           (task-body
            "package body Test is\n   task body ABC is\n   begin\n      null;\n   end ABC;\nend Test;\n"
            "task body ABC"
            "task_body"
            "Test.ABC")
           (task-body-stub
            "package body Test is\n   task body ABC is separate;\nend Test;\n"
            "task body ABC"
            "task_body_stub"
            "Test.ABC")
           (task-type
            "package Test is\n   task type ABC;\nend Test;\n"
            "task type ABC"
            "task_type_declaration"
            "Test.ABC"))))"##;
    let expect = expect![[
        r#"OK ((broken-package "package_declaration" "package_declaration" t "XYZ.ABC" #("XYZ.ABC" 0 3 (face (:foreground #1="unspecified-fg")) 4 7 (face (:foreground #1#))) t) (entry-body "entry_body" "entry_body" t "Test.XYZ.ABC" #("Test.XYZ.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-variable-name-face)) 9 12 (face (font-lock-function-name-face))) t) (entry-declaration "entry_declaration" "entry_declaration" t "Test.XYZ.ABC" #("Test.XYZ.ABC" 0 4 (face (:foreground #1#)) 5 8 (face font-lock-type-face) 9 12 (face (font-lock-function-name-face))) t) (expression-function "expression_function_declaration" "expression_function_declaration" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-function-name-face))) t) (formal-abstract-subprogram "formal_abstract_subprogram_declaration" "formal_abstract_subprogram_declaration" t "ABC.DEF" #("ABC.DEF" 0 3 (face (:foreground #1#)) 4 7 (face (font-lock-function-name-face))) t) (formal-concrete-subprogram "formal_concrete_subprogram_declaration" "formal_concrete_subprogram_declaration" t "ABC.DEF" #("ABC.DEF" 0 3 (face (:foreground #1#)) 4 7 (face (font-lock-function-name-face))) t) (formal-package "formal_package_declaration" "formal_package_declaration" t "ABC.DEF" #("ABC.DEF" 0 3 (face (:foreground #1#)) 4 7 (face (:foreground #1#))) t) (generic-instantiation "generic_instantiation" "generic_instantiation" t "ABC" #("ABC" 0 3 (face (:foreground #1#))) t) (generic-package "generic_package_declaration" "generic_package_declaration" t "ABC" #("ABC" 0 3 (face (:foreground #1#))) t) (generic-renaming "generic_renaming_declaration" "generic_renaming_declaration" t "ABC" #("ABC" 0 3 (face (:foreground #1#))) t) (generic-subprogram "generic_subprogram_declaration" "generic_subprogram_declaration" t "ABC" #("ABC" 0 3 (face (font-lock-function-name-face))) t) (null-procedure "null_procedure_declaration" "null_procedure_declaration" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-function-name-face))) t) (package-body "package_body" "package_body" t "ABC" #("ABC" 0 3 (face (:foreground #1#))) t) (package-body-stub "package_body_stub" "package_body_stub" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (:foreground #1#))) t) (package-declaration "package_declaration" "package_declaration" t "ABC" #("ABC" 0 3 (face (:foreground #1#))) t) (package-renaming "package_renaming_declaration" "package_renaming_declaration" t "ABC" #("ABC" 0 3 (face (:foreground #1#))) t) (protected-body "protected_body" "protected_body" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-variable-name-face))) t) (protected-body-stub "protected_body_stub" "protected_body_stub" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-variable-name-face))) t) (protected-type "protected_type_declaration" "protected_type_declaration" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face font-lock-type-face)) t) (single-protected "single_protected_declaration" "single_protected_declaration" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-variable-name-face))) t) (single-task "single_task_declaration" "single_task_declaration" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-variable-name-face))) t) (subprogram-body "subprogram_body" "subprogram_body" t "ABC" #("ABC" 0 3 (face (font-lock-function-name-face))) t) (subprogram-body-stub "subprogram_body_stub" "subprogram_body_stub" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-function-name-face))) t) (subprogram-declaration "subprogram_declaration" "subprogram_declaration" t "ABC" #("ABC" 0 3 (face (font-lock-function-name-face))) t) (subprogram-renaming "subprogram_renaming_declaration" "subprogram_renaming_declaration" t "ABC" #("ABC" 0 3 (face (font-lock-function-name-face))) t) (subunit "subunit" "subunit" t "XYZ" #("XYZ" 0 3 (face (:foreground #1#))) t) (task-body "task_body" "task_body" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-variable-name-face))) t) (task-body-stub "task_body_stub" "task_body_stub" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face (font-lock-variable-name-face))) t) (task-type "task_type_declaration" "task_type_declaration" t "Test.ABC" #("Test.ABC" 0 4 (face (:foreground #1#)) 5 8 (face font-lock-type-face)) t))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_tree_sitter_beginning_and_end_of_defun_navigate_nested_units() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "package body Outer is\n"
          "   protected body Guard is\n"
          "      procedure Lock is\n"
          "      begin\n"
          "         null;\n"
          "      end Lock;\n"
          "   end Guard;\n"
          "\n"
          "   task body Worker is\n"
          "   begin\n"
          "      null;\n"
          "   end Worker;\n"
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
               (ada-ts-mode--defun-name
                node
                'no-property)
               (save-excursion
                 (beginning-of-defun)
                 (list
                  (point)
                  (line-number-at-pos)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position))))
               (save-excursion
                 (end-of-defun)
                 (list
                  (point)
                  (line-number-at-pos)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position))))
               (save-excursion
                 (beginning-of-defun
                  2)
                 (list
                  (point)
                  (line-number-at-pos)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position)))))))
          '("null;"
            "end Guard"
            "end Worker")))"##;
    let expect = expect![[
        r#"OK (("null;" "subprogram_body" "Lock" (50 3 "      procedure Lock is") (117 7 "   end Guard;") (23 2 "   protected body Guard is")) ("end Guard" "protected_body" "Guard" (50 3 "      procedure Lock is") (131 8 "") (23 2 "   protected body Guard is")) ("end Worker" "task_body" "Worker" (132 9 "   task body Worker is") (191 13 "end Outer;") (23 2 "   protected body Guard is")))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_upstream_font_lock_special_query_families_match_exact_faces_and_nodes() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (insert
              (nth
               1
               case))
             (let ((ada-ts-mode-grammar-install
                    nil)
                   (treesit-font-lock-level
                    4))
               (ada-ts-mode))
             (font-lock-ensure
              (point-min)
              (point-max))
             (goto-char
              (point-min))
             (search-forward
              (nth
               2
               case))
             (let* ((position
                     (match-beginning
                      0))
                    (actual-face
                     (get-text-property
                      position
                      'face))
                    (node
                     (treesit-node-at
                      position)))
               (list
                (nth
                 0
                 case)
                (nth
                 2
                 case)
                (nth
                 3
                 case)
                actual-face
                (equal
                 (nth
                  3
                  case)
                 actual-face)
                (treesit-node-type
                 node)))))
         '((allocator
            "procedure P is\nbegin\n   A := new Allocated_Type;\nend P;\n"
            "Allocated_Type"
            font-lock-type-face)
           (derived-type
            "package P is\n   type Child is new Parent_Type;\nend P;\n"
            "Parent_Type"
            font-lock-type-face)
           (formal-derived-type
            "generic\n   type Child is new Parent_Type with private;\npackage P is\nend P;\n"
            "Parent_Type"
            font-lock-type-face)
           (interface-type
            "package P is\n   type Worker is interface and Parent_Interface;\nend P;\n"
            "Parent_Interface"
            font-lock-type-face)
           (private-extension
            "package P is\n   type Child is new Private_Base with private;\nend P;\n"
            "Private_Base"
            font-lock-type-face)
           (generic-instantiation
            "procedure Build is new Generic_Template;\n"
            "Generic_Template"
            (font-lock-function-name-face))
           (operator-symbol
            "function \"+\" (Left, Right : Integer) return Integer;\n"
            "\"+\""
            (font-lock-function-name-face))
           (primary-null
            "package body P is\n   X : access Integer := null;\nend P;\n"
            "null"
            font-lock-constant-face)
           (protected-unit
            "package body P is\n   protected body Protected_Unit is\n   end Protected_Unit;\nend P;\n"
            "Protected_Unit"
            (font-lock-variable-name-face))
           (task-unit
            "package body P is\n   task body Task_Unit is\n   begin\n      null;\n   end Task_Unit;\nend P;\n"
            "Task_Unit"
            (font-lock-variable-name-face))
           (range-attribute
            "procedure P is\n   A : array (1 .. 3) of Integer;\nbegin\n   for I in A'Range loop\n      null;\n   end loop;\nend P;\n"
            "Range"
            font-lock-property-use-face)
           (reduction-attribute
            "function Sum return Integer is\n  (Values'Reduce (\"+\", 0));\n"
            "Reduce"
            (font-lock-function-call-face font-lock-property-use-face))
           (record-representation-type
            "package P is\n   for Record_Type use record\n      Field at 0 range 0 .. 31;\n   end record Record_Type;\nend P;\n"
            "Record_Type"
            font-lock-type-face)
           (record-representation-component
            "package P is\n   for Record_Type use record\n      Field at 0 range 0 .. 31;\n   end record Record_Type;\nend P;\n"
            "Field"
            font-lock-property-name-face)
           (function-call
            "function P return Integer is\n  (Compute (1));\n"
            "Compute"
            (font-lock-function-call-face))
           (procedure-call
            "procedure P is\nbegin\n   Run (1);\nend P;\n"
            "Run"
            (font-lock-function-call-face))))"##;
    let expect = expect![[
        r#"OK ((allocator "Allocated_Type" font-lock-type-face font-lock-type-face t "identifier") (derived-type "Parent_Type" font-lock-type-face font-lock-type-face t "identifier") (formal-derived-type "Parent_Type" font-lock-type-face font-lock-type-face t "identifier") (interface-type "Parent_Interface" font-lock-type-face font-lock-type-face t "identifier") (private-extension "Private_Base" font-lock-type-face font-lock-type-face t "identifier") (generic-instantiation "Generic_Template" (font-lock-function-name-face) (font-lock-function-name-face) t "identifier") (operator-symbol "\"+\"" (font-lock-function-name-face) (font-lock-function-name-face) t "string_literal") (primary-null "null" font-lock-constant-face font-lock-constant-face t "null") (protected-unit "Protected_Unit" (font-lock-variable-name-face) (font-lock-variable-name-face) t "identifier") (task-unit "Task_Unit" (font-lock-variable-name-face) (font-lock-variable-name-face) t "identifier") (range-attribute "Range" font-lock-property-use-face font-lock-property-use-face t "range") (reduction-attribute "Reduce" (font-lock-function-call-face font-lock-property-use-face) (font-lock-function-call-face font-lock-property-use-face) t "identifier") (record-representation-type "Record_Type" font-lock-type-face font-lock-type-face t "identifier") (record-representation-component "Field" font-lock-property-name-face font-lock-property-name-face t "identifier") (function-call "Compute" (font-lock-function-call-face) (font-lock-function-call-face) t "identifier") (procedure-call "Run" (font-lock-function-call-face) (font-lock-function-call-face) t "identifier"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_upstream_indentation_families_reindent_without_catch_all_fallback() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (insert
              (nth
               1
               case))
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
                   "Upstream indentation matrix reached catch-all anchor for %s"
                   (nth
                    0
                    case))))
               (setcar
                (cddr
                 catch-all-rule)
                (lambda (&rest _)
                  (error
                   "Upstream indentation matrix reached catch-all offset for %s"
                   (nth
                    0
                    case)))))
             (let ((before
                    (buffer-string)))
               (indent-region
                (point-min)
                (point-max))
               (list
                (nth
                 0
                 case)
                (not
                 (equal
                  before
                  (buffer-string)))
                (buffer-string)))))
         '((package-body
            "package body Outer is\nprocedure Run is\nbegin\nnull;\nend Run;\nend Outer;\n")
           (package-body-stub
            "package body Outer is\npackage body Child is separate;\nend Outer;\n")
           (protected-body
            "package body Outer is\nprotected body Guard is\nprocedure Lock is\nbegin\nnull;\nend Lock;\nend Guard;\nend Outer;\n")
           (protected-body-stub
            "package body Outer is\nprotected body Guard is separate;\nend Outer;\n")
           (protected-type
            "package Outer is\nprotected type Guard is\nprocedure Lock;\nprivate\nBusy : Boolean;\nend Guard;\nend Outer;\n")
           (single-protected
            "package Outer is\nprotected Guard is\nprocedure Lock;\nprivate\nBusy : Boolean;\nend Guard;\nend Outer;\n")
           (task-body
            "package body Outer is\ntask body Worker is\nbegin\nnull;\nend Worker;\nend Outer;\n")
           (task-body-stub
            "package body Outer is\ntask body Worker is separate;\nend Outer;\n")
           (task-type
            "package Outer is\ntask type Worker is\nentry Start;\nend Worker;\nend Outer;\n")
           (single-task
            "package Outer is\ntask Worker is\nentry Start;\nend Worker;\nend Outer;\n")
           (subprogram-body
            "procedure Run is\nValue : Integer := 1;\nbegin\nValue := Value + 1;\nend Run;\n")
           (subprogram-body-stub
            "package body Outer is\nprocedure Run is separate;\nend Outer;\n")
           (abstract-subprogram
            "package Outer is\nprocedure Run\nis\nabstract;\nend Outer;\n")
           (null-procedure
            "package Outer is\nprocedure Stop\nis\nnull;\nend Outer;\n")
           (expression-function
            "package Outer is\nfunction Value return Integer\nis\n(42);\nend Outer;\n")
           (array-delta
            "package Outer is\ntype Vector is array (1 .. 2) of Integer;\nBase : Vector := (1, 2);\nChanged : Vector :=\n(Base with delta\n1 => 3,\n2 => 4);\nend Outer;\n")
           (record-delta
            "package Outer is\ntype Pair is record\nLeft : Integer;\nRight : Integer;\nend record;\nBase : Pair := (1, 2);\nChanged : Pair :=\n(Base with delta\nLeft => 3,\nRight => 4);\nend Outer;\n")
           (enumeration-representation
            "package Outer is\ntype Color is (Red, Green, Blue);\nfor Color use\n(Red => 1,\nGreen => 2,\nBlue => 3);\nend Outer;\n")
           (record-representation
            "package Outer is\nfor Pair use record\nLeft at 0 range 0 .. 31;\nRight at 4 range 0 .. 31;\nend record;\nend Outer;\n")
           (case-expression
            "package body Outer is\nValue : Integer :=\n(case Choice is\nwhen 1 => 10,\nwhen others => 20);\nend Outer;\n")
           (if-expression
            "package body Outer is\nValue : Integer :=\n(if Choice then\n10\nelse\n20);\nend Outer;\n")
           (declare-expression
            "package body Outer is\nfunction Add return Integer is\n(declare\nValue : constant Integer := 1;\nbegin\nValue + 2);\nend Outer;\n")
           (extended-return
            "function Build return Integer is\nbegin\nreturn Value : Integer do\nValue := 7;\nend return;\nend Build;\n")
           (context-clauses
            "with\nAda.Text_IO,\nSystem;\nuse\nAda.Text_IO,\nSystem;\nprocedure Run is\nbegin\nnull;\nend Run;\n")))"##;
    let expect = expect![[
        r#"OK ((package-body t "package body Outer is\n   procedure Run is\n   begin\n      null;\n   end Run;\nend Outer;\n") (package-body-stub t "package body Outer is\n   package body Child is separate;\nend Outer;\n") (protected-body t "package body Outer is\n   protected body Guard is\n      procedure Lock is\n      begin\n         null;\n      end Lock;\n   end Guard;\nend Outer;\n") (protected-body-stub t "package body Outer is\n   protected body Guard is separate;\nend Outer;\n") (protected-type t "package Outer is\n   protected type Guard is\n      procedure Lock;\n   private\n      Busy : Boolean;\n   end Guard;\nend Outer;\n") (single-protected t "package Outer is\n   protected Guard is\n      procedure Lock;\n   private\n      Busy : Boolean;\n   end Guard;\nend Outer;\n") (task-body t "package body Outer is\n   task body Worker is\n   begin\n      null;\n   end Worker;\nend Outer;\n") (task-body-stub t "package body Outer is\n   task body Worker is separate;\nend Outer;\n") (task-type t "package Outer is\n   task type Worker is\n      entry Start;\n   end Worker;\nend Outer;\n") (single-task t "package Outer is\n   task Worker is\n      entry Start;\n   end Worker;\nend Outer;\n") (subprogram-body t "procedure Run is\n   Value : Integer := 1;\nbegin\n   Value := Value + 1;\nend Run;\n") (subprogram-body-stub t "package body Outer is\n   procedure Run is separate;\nend Outer;\n") (abstract-subprogram t "package Outer is\n   procedure Run\n     is\n     abstract;\nend Outer;\n") (null-procedure t "package Outer is\n   procedure Stop\n     is\n     null;\nend Outer;\n") (expression-function t "package Outer is\n   function Value return Integer\n     is\n     (42);\nend Outer;\n") (array-delta t "package Outer is\n   type Vector is array (1 .. 2) of Integer;\n   Base : Vector := (1, 2);\n   Changed : Vector :=\n     (Base with delta\n        1 => 3,\n        2 => 4);\nend Outer;\n") (record-delta t "package Outer is\n   type Pair is record\n      Left : Integer;\n      Right : Integer;\n   end record;\n   Base : Pair := (1, 2);\n   Changed : Pair :=\n     (Base with delta\n        Left => 3,\n        Right => 4);\nend Outer;\n") (enumeration-representation t "package Outer is\n   type Color is (Red, Green, Blue);\n   for Color use\n     (Red => 1,\n      Green => 2,\n      Blue => 3);\nend Outer;\n") (record-representation t "package Outer is\n   for Pair use record\n      Left at 0 range 0 .. 31;\n      Right at 4 range 0 .. 31;\n   end record;\nend Outer;\n") (case-expression t "package body Outer is\n   Value : Integer :=\n     (case Choice is\n         when 1 => 10,\n         when others => 20);\nend Outer;\n") (if-expression t "package body Outer is\n   Value : Integer :=\n     (if Choice then\n         10\n      else\n         20);\nend Outer;\n") (declare-expression t "package body Outer is\n   function Add return Integer is\n     (declare\n         Value : constant Integer := 1;\n      begin\n         Value + 2);\nend Outer;\n") (extended-return t "function Build return Integer is\nbegin\n   return Value : Integer do\n      Value := 7;\n   end return;\nend Build;\n") (context-clauses t "with\n  Ada.Text_IO,\n  System;\nuse\n  Ada.Text_IO,\n  System;\nprocedure Run is\nbegin\n   null;\nend Run;\n"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_upstream_shebang_is_preserved_while_following_code_is_indented() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "#! /usr/bin/env hac\n"
          "\n"
          "with Ada.Text_IO; use Ada.Text_IO;\n"
          "\n"
          "procedure Hello_World is\n"
          "begin\n"
          "Put_Line (\"Hello, world!\");\n"
          "end Hello_World;\n")
         (let ((ada-ts-mode-grammar-install
                nil))
           (ada-ts-mode))
         (setq-local
          indent-tabs-mode
          nil)
         (condition-case error-data
             (progn
               (indent-region
                (point-min)
                (point-max))
               (list
                'ok
                (buffer-string)))
           (error
            (list
             'error
             (car
              error-data)
             (error-message-string
              error-data)
             (buffer-string)))))"##;
    let expect = expect![[
        r##"OK (ok "#! /usr/bin/env hac\n\nwith Ada.Text_IO; use Ada.Text_IO;\n\nprocedure Hello_World is\nbegin\n   Put_Line (\"Hello, world!\");\nend Hello_World;\n")"##
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}
