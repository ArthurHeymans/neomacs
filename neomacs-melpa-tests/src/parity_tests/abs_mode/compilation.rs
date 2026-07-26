use expect_test::expect;

use super::assert_abs_mode_parity;

#[test]
fn abs_mode_file_date_comparison_orders_seconds_then_subseconds_with_equal_identity() {
    let elisp_form = r##"(mapcar
               (lambda (pair)
                 (abs--file-date-< (car pair) (cadr pair)))
               '(((1 2) (1 3))
                 ((1 3) (1 2))
                 ((1 2) (2 0))
                 ((2 0) (1 9))
                 ((1 2) (1 2))))"##;
    let expect = expect!["OK (t nil t nil nil)"];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_input_and_output_filename_helpers_cover_overrides_fallbacks_and_backends() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/workspace/model/main.abs")
               (cl-letf
                   (((symbol-function 'abs--calculate-input-files)
                     (lambda () '("calculated.abs"))))
                 (list
                  (let ((abs-input-files '("one.abs" "two.abs")))
                    (list
                     (abs--input-files)
                     (abs--maude-filename)))
                  (let ((abs-input-files nil)
                        (abs-maude-output-file "custom.maude"))
                    (list
                     (abs--input-files)
                     (abs--maude-filename)))
                  (abs--absolutify-filename "relative.abs")
                  (abs--absolutify-filename
                   "/absolute/model.abs")
                  (mapcar
                   (lambda (backend)
                     (let ((abs-backend backend)
                           (abs-output-directory nil))
                       (list
                        backend
                        (abs--real-output-directory))))
                   abs--backends)
                  (let ((abs-output-directory "build"))
                    (abs--real-output-directory)))))"##;
    let expect = expect![[
        r#"OK ((("one.abs" "two.abs") "one.maude") (("calculated.abs") "custom.maude") "/workspace/model/relative.abs" "/absolute/model.abs" ((java "gen/") (erlang "gen/erl/") (maude "./") (prolog "./")) "build/")"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_guess_module_selects_the_last_matching_module_and_supports_dots_underscores() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "module First;\n"
                "  module Nested.Name_2;\n"
                "// module Commented;\n")
               (set-syntax-table abs-mode-syntax-table)
               (abs--guess-module))"##;
    let expect = expect![[r#"OK "Nested.Name_2""#]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_compile_command_honors_explicit_and_makefile_precedence() {
    let elisp_form = r##"(let ((abs-backend 'java)
                    (abs-compile-command "custom compile")
                    (compile-command "make -k"))
               (list
                (abs--calculate-compile-command)
                (let ((abs-compile-command nil))
                  (cl-letf
                      (((symbol-function 'file-exists-p)
                        (lambda (path)
                          (equal path "Makefile"))))
                    (abs--calculate-compile-command)))))"##;
    let expect = expect![[r#"OK ("custom compile" "make -k")"#]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_compile_command_builds_exact_backend_options_products_outputs_and_link_chain() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/workspace/main.abs")
               (cl-letf
                   (((symbol-function 'file-exists-p)
                     (lambda (&rest _) nil))
                    ((symbol-function 'local-variable-p)
                     (lambda (variable)
                       (eq variable 'abs-clock-limit))))
                 (let ((abs-compiler-program "compiler")
                       (abs-input-files
                        '("main.abs" "lib file.abs"))
                       (abs-modelapi-index-file "index.html")
                       (abs-modelapi-static-dir "static files")
                       (abs-product-name "Product")
                       (abs-output-directory "output dir")
                       (abs-maude-output-file "model.maude")
                       (abs-java-output-jar-file "model.jar")
                       (abs-use-timed-interpreter nil)
                       (abs-clock-limit nil)
                       (abs-default-resourcecost 7)
                       (abs-compile-with-coverage-info t)
                       (abs-link-source-path
                        "/runtime source"))
                   (mapcar
                    (lambda (backend)
                      (let ((abs-backend backend))
                        (list
                         backend
                         (abs--calculate-compile-command))))
                    abs--backends))))"##;
    let expect = expect![[
        r#"OK ((java "compiler --java \"main.abs\" \"lib file.abs\" --modelapi-index-file \"index.html\" --modelapi-static-dir \"static files\" -o \"model.jar\" -d \"output dir\" --product Product ") (erlang "compiler --erlang \"main.abs\" \"lib file.abs\" --modelapi-index-file \"index.html\" --modelapi-static-dir \"static files\" -d \"output dir\" --product Product --debuginfo && cd \"output dir/\" && ./link_sources /runtime source ") (maude "compiler --maude \"main.abs\" \"lib file.abs\" --modelapi-index-file \"index.html\" --modelapi-static-dir \"static files\" -o \"model.maude\" -d \"output dir\" --product Product --timed --limit=100 --defaultcost 7 ") (prolog "compiler --prolog \"main.abs\" \"lib file.abs\" --modelapi-index-file \"index.html\" --modelapi-static-dir \"static files\" -d \"output dir\" --product Product "))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_compile_command_timed_maude_defaults_limit_and_erlang_flags_are_conditional() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/workspace/main.abs")
               (cl-letf
                   (((symbol-function 'file-exists-p)
                     (lambda (&rest _) nil))
                    ((symbol-function 'local-variable-p)
                     (lambda (&rest _) nil)))
                 (list
                  (let ((abs-backend 'maude)
                        (abs-input-files '("main.abs"))
                        (abs-use-timed-interpreter t)
                        (abs-clock-limit nil)
                        (abs-default-resourcecost 0))
                    (abs--calculate-compile-command))
                  (let ((abs-backend 'maude)
                        (abs-input-files '("main.abs"))
                        (abs-use-timed-interpreter nil)
                        (abs-clock-limit 0)
                        (abs-default-resourcecost 0))
                    (abs--calculate-compile-command))
                  (let ((abs-backend 'erlang)
                        (abs-input-files '("main.abs"))
                        (abs-compile-with-coverage-info nil)
                        (abs-link-source-path nil))
                    (abs--calculate-compile-command)))))"##;
    let expect = expect![[
        r#"OK ("absc --maude \"main.abs\" -o \"main.maude\" --timed --limit=100 " "absc --maude \"main.abs\" -o \"main.maude\" " "absc --erlang \"main.abs\" ")"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_needs_compilation_checks_backend_output_times_and_modified_buffer_state() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/workspace/main.abs")
               (let ((abs-output-directory "/workspace/out")
                     (answers
                      '((nil nil nil nil nil (1 0))
                        nil
                        (nil nil nil nil nil (3 0))
                        (nil nil nil nil nil (2 0))
                        (nil nil nil nil nil (1 0))
                        (nil nil nil nil nil (2 0))
                        (nil nil nil nil nil (1 0))
                        (nil nil nil nil nil (2 0))))
                     (modified '(nil t))
                     events)
                 (cl-letf
                     (((symbol-function 'file-attributes)
                       (lambda (path)
                         (push (list 'attributes path) events)
                         (pop answers)))
                      ((symbol-function 'buffer-modified-p)
                       (lambda ()
                         (let ((value (pop modified)))
                           (push (list 'modified value) events)
                           value))))
                   (list
                    (let ((abs-backend 'maude)
                          (abs-maude-output-file "main.maude"))
                      (abs--needs-compilation))
                    (let ((abs-backend 'erlang))
                      (abs--needs-compilation))
                    (let ((abs-backend 'java))
                      (abs--needs-compilation))
                    (let ((abs-backend 'prolog))
                      (abs--needs-compilation))
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (t t nil t ((attributes "/workspace/main.abs") (attributes "/workspace/main.maude") (attributes "/workspace/main.abs") (attributes "/workspace/out/absmodel/Emakefile") (attributes "/workspace/main.abs") (attributes "/workspace/out/ABS/StdLib/Bool.java") (modified nil) (attributes "/workspace/main.abs") (attributes "/workspace/abs.pl") (modified t)))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_compile_entry_points_forward_interactive_and_direct_commands_exactly() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'abs--calculate-compile-command)
                     (lambda ()
                       (push '(calculate) events)
                       "compile model"))
                    ((symbol-function 'call-interactively)
                     (lambda (command)
                       (push (list 'interactive command) events)
                       'interactive-result))
                    ((symbol-function 'compile)
                     (lambda (command)
                       (push (list 'compile command) events)
                       'compile-result)))
                 (list
                  (abs--compile-model)
                  (abs--compile-model-no-prompt)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (interactive-result compile-result (#1=(calculate) (interactive compile) #1# (compile "compile model")))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_flymake_init_builds_temp_first_and_removes_the_current_file_from_dependencies() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/workspace/main.abs")
               (let ((abs-compiler-program "absc")
                     events)
                 (cl-letf
                     (((symbol-function
                        'abs--calculate-input-files)
                       (lambda ()
                         '("main.abs" "lib.abs" "main.abs")))
                      ((symbol-function
                        'flymake-proc-init-create-temp-buffer-copy)
                       (lambda (creator)
                         (push (list 'temp creator) events)
                         "/workspace/main_flymake.abs")))
                   (list
                    (abs-flymake-init)
                    (nreverse events)
                    (let ((abs-compiler-program nil))
                      (abs-flymake-init))))))"##;
    let expect = expect![[
        r#"OK (("absc" ("/workspace/main_flymake.abs" "lib.abs")) ((temp flymake-proc-create-temp-inplace)) nil)"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}
