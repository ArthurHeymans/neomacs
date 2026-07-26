use expect_test::expect;

use super::assert_abs_mode_parity;

#[test]
fn abs_mode_referenced_modules_parses_anchored_imports_and_uses_with_duplicate_removal() {
    let elisp_form = r##"(with-temp-buffer
               (set-syntax-table abs-mode-syntax-table)
               (insert
                "import * from Alpha.Core;\n"
                " import Beta from Beta.Mod ;\n"
                "uses Gamma.Feature;\n"
                "uses Alpha.Core;\n"
                "// import * from Commented.Out;\n"
                "value = \"import * from String.Out;\";\n"
                "import * from Alpha.Core;\n"
                "notimport * from Wrong.Prefix;\n")
               (list
                (abs--current-buffer-referenced-modules)
                (point)))"##;
    let expect = expect![[r#"OK (("Alpha.Core" "Beta.Mod" "Gamma.Feature") 219)"#]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_module_definitions_match_only_valid_anchored_declarations_and_preserve_order() {
    let elisp_form = r##"(with-temp-buffer
               (set-syntax-table abs-mode-syntax-table)
               (insert
                "module First;\n"
                " module Nested.Name ;\n"
                "// module Commented;\n"
                "xmodule Wrong;\n"
                "module With_Underscore;\n"
                "module First;\n")
               (list
                (abs--current-buffer-module-definitions)
                (point)))"##;
    let expect = expect![[r#"OK (("First" "Nested.Name" "First") 111)"#]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_file_parsers_read_workspace_local_files_without_changing_the_current_buffer() {
    let elisp_form = r##"(let* ((root
                      (expand-file-name
                       "abs-mode-file-parsers"
                       (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                     (file (expand-file-name "library.abs" root)))
                (make-directory root t)
                (with-temp-file file
                  (insert
                   "module Library.Core;\n"
                   "import * from Dependency.One;\n"
                   "uses Dependency.Two;\n"))
                (with-temp-buffer
                  (insert "sentinel")
                  (goto-char 4)
                  (list
                   (abs--file-imports file)
                   (abs--file-module-definitions file)
                   (buffer-string)
                   (point))))"##;
    let expect =
        expect![[r#"OK (("Dependency.One" "Dependency.Two") ("Library.Core") "sentinel" 4)"#]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_module_file_alist_indexes_each_definition_and_retains_duplicate_modules() {
    let elisp_form = r##"(let* ((root
                      (expand-file-name
                       "abs-mode-module-index"
                       (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                     (default-directory
                      (file-name-as-directory root)))
                (make-directory root t)
                (with-temp-file
                    (expand-file-name "a.abs" root)
                  (insert
                   "module Shared;\n"
                   "module Alpha;\n"))
                (with-temp-file
                    (expand-file-name "b.abs" root)
                  (insert
                   "module Shared;\n"
                   "module Beta;\n"))
                (with-temp-file
                    (expand-file-name "ignored.txt" root)
                  (insert "module Ignored;\n"))
                (sort
                 (abs--module-file-alist)
                 (lambda (left right)
                   (if (equal (car left) (car right))
                       (string< (cdr left) (cdr right))
                     (string< (car left) (car right))))))"##;
    let expect = expect![[
        r#"OK (("Alpha" . "a.abs") ("Beta" . "b.abs") ("Shared" . "a.abs") ("Shared" . "b.abs"))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_calculate_input_files_adds_direct_available_modules_once_and_keeps_main_first() {
    let elisp_form = r##"(let* ((root
                      (expand-file-name
                       "abs-mode-input-closure"
                       (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                     (default-directory
                      (file-name-as-directory root))
                     (main (expand-file-name "main.abs" root)))
                (make-directory root t)
                (with-temp-file main
                  (insert
                   "module Main;\n"
                   "import * from Alpha;\n"
                   "uses Beta;\n"
                   "import * from Missing;\n"))
                (with-temp-file
                    (expand-file-name "alpha.abs" root)
                  (insert
                   "module Alpha;\n"
                   "import * from Transitive;\n"))
                (with-temp-file
                    (expand-file-name "beta.abs" root)
                  (insert "module Beta;\n"))
                (with-temp-file
                    (expand-file-name "transitive.abs" root)
                  (insert "module Transitive;\n"))
                (with-temp-buffer
                  (setq buffer-file-name main)
                  (insert-file-contents main)
                  (abs--calculate-input-files)))"##;
    let expect = expect![[r#"OK ("main.abs" "alpha.abs" "beta.abs")"#]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_calculate_input_files_uses_one_arbitrary_location_for_duplicate_modules() {
    let elisp_form = r##"(let (events)
               (with-temp-buffer
                 (setq buffer-file-name "/project/main.abs")
                 (cl-letf
                     (((symbol-function 'abs--module-file-alist)
                       (lambda ()
                         '(("Shared" . "first.abs")
                           ("Shared" . "second.abs"))))
                      ((symbol-function
                        'abs--file-module-definitions)
                       (lambda (file)
                         (push (list 'definitions file) events)
                         '("Main")))
                      ((symbol-function
                        'abs--current-buffer-referenced-modules)
                       (lambda ()
                         (push '(references) events)
                         '("Shared" "Shared"))))
                   (list
                    (abs--calculate-input-files)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (("main.abs" "first.abs") ((definitions "/project/main.abs") (references)))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}
