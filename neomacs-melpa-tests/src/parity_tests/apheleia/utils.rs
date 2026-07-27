use expect_test::expect;

use super::assert_apheleia_parity;

#[test]
fn apheleia_formatter_indent_respects_tabs_explicit_offsets_and_real_mode_defaults() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (setq major-mode (nth 0 case))
             (setq-local indent-tabs-mode (nth 1 case))
             (let ((apheleia-formatters-respect-indent-level
                    (nth 2 case)))
               (when (nth 3 case)
                 (set (nth 3 case) (nth 4 case)))
               (list
                case
                (apheleia-formatters-indent
                 "--tabs"
                 "--indent")
                (apheleia-formatters-indent
                 "--tabs"
                 "--indent"
                 (nth 3 case))))))
         '((python-mode nil t python-indent-offset 2)
           (js-mode nil t js-indent-level 4)
           (html-mode nil t sgml-basic-offset 3)
           (fundamental-mode nil t apheleia-test-indent 7)
           (fundamental-mode nil t apheleia-test-unbound nil)
           (python-mode t t python-indent-offset 8)
           (python-mode nil nil python-indent-offset 8)))"##;
    let expect = expect![[
        r#"OK (((python-mode nil t python-indent-offset 2) ("--indent" "2") ("--indent" "2")) ((js-mode nil t js-indent-level 4) ("--indent" "4") ("--indent" "4")) ((html-mode nil t sgml-basic-offset 3) ("--indent" "3") ("--indent" "3")) ((fundamental-mode nil t apheleia-test-indent 7) nil ("--indent" "7")) ((fundamental-mode nil t apheleia-test-unbound nil) nil nil) ((python-mode t t python-indent-offset 8) "--tabs" "--tabs") ((python-mode nil nil python-indent-offset 8) nil nil))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_formatter_fill_column_distinguishes_disabled_nil_zero_and_real_columns() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (let ((apheleia-formatters-respect-fill-column
                    (car case)))
               (setq-local fill-column (cadr case))
               (list
                case
                (apheleia-formatters-fill-column
                 "--line-length")))))
         '((nil 88)
           (t nil)
           (t 0)
           (t 72)
           (t 120)))"##;
    let expect = expect![[
        r#"OK (((nil 88) nil) ((t nil) nil) ((t 0) ("--line-length" "0")) ((t 72) ("--line-length" "72")) ((t 120) ("--line-length" "120")))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_formatter_locate_file_walks_nested_projects_and_returns_exact_flagged_paths() {
    let elisp_form = r##"(let* ((root
                  (make-temp-file
                   "apheleia-locate-"
                   t))
                 (project
                  (expand-file-name
                   "workspace/"
                   root))
                 (nested
                  (expand-file-name
                   "src/deep/"
                   project))
                 (other
                  (expand-file-name
                   "other/"
                   root))
                 (config
                  (expand-file-name
                   ".formatter"
                   project)))
         (unwind-protect
             (progn
               (make-directory nested t)
               (make-directory other t)
               (with-temp-file config
                 (insert "indent=2\n"))
               (list
                (let ((default-directory nested))
                  (let ((result
                         (apheleia-formatters-locate-file
                          "--config"
                          ".formatter")))
                    (list
                     (car result)
                     (file-relative-name
                      (cadr result)
                      root))))
                (let ((default-directory project))
                  (let ((result
                         (apheleia-formatters-locate-file
                          "--config"
                          ".formatter")))
                    (list
                     (car result)
                     (file-relative-name
                      (cadr result)
                      root))))
                (let ((default-directory other))
                  (apheleia-formatters-locate-file
                   "--config"
                   ".formatter"))
                (file-exists-p config)))
           (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("--config" "workspace/.formatter") ("--config" "workspace/.formatter") nil t)"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_formatter_extension_predicate_handles_case_multiple_dots_and_missing_files() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (setq-local buffer-file-name
                         (car case))
             (list
              case
              (apply
               #'apheleia-formatters-extension-p
               (cadr case)))))
         '((nil ("js" "ts"))
           ("/project/file" ("js"))
           ("/project/.config" ("config" nil))
           ("/project/archive.tar.gz" ("tar" "gz"))
           ("/project/archive.tar.gz" ("gz" "tar"))
           ("/project/component.TS" ("ts" "TS"))
           ("/project/component.tsx" ("ts" "tsx"))))"##;
    let expect = expect![[
        r#"OK (((nil ("js" "ts")) nil) (("/project/file" ("js")) nil) (("/project/.config" ("config" nil)) nil) (("/project/archive.tar.gz" ("tar" "gz")) "gz") (("/project/archive.tar.gz" ("gz" "tar")) "gz") (("/project/component.TS" ("ts" "TS")) "TS") (("/project/component.tsx" ("ts" "tsx")) "tsx"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_formatter_mode_extension_covers_default_and_custom_mode_mappings() {
    let elisp_form = r##"(let ((apheleia-formatters-mode-extension-assoc
                (append
                 apheleia-formatters-mode-extension-assoc
                 '((apheleia-test-mode . ".demo")))))
         (mapcar
          (lambda (mode)
            (with-temp-buffer
              (setq major-mode mode)
              (list
               mode
               (apheleia-formatters-mode-extension)
               (apheleia-formatters-mode-extension
                "--assume-filename"))))
          '(c-mode
            c++-mode
            glsl-mode
            java-mode
            apheleia-test-mode
            fundamental-mode)))"##;
    let expect = expect![[
        r#"OK ((c-mode ".c" ("--assume-filename" ".c")) (c++-mode ".cpp" ("--assume-filename" ".cpp")) (glsl-mode ".glsl" ("--assume-filename" ".glsl")) (java-mode ".java" ("--assume-filename" ".java")) (apheleia-test-mode ".demo" ("--assume-filename" ".demo")) (fundamental-mode nil nil))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_local_filename_safe_name_and_symbol_replacement_cover_real_command_inputs() {
    let elisp_form = r##"(list
         (mapcar
          #'apheleia-formatters-local-buffer-file-name
          '("/workspace/source.el"
            "/ssh:user@example.test:/srv/project/source.el"
            "/sudo:root@localhost:/etc/config"
            nil))
         (with-temp-buffer
           (rename-buffer
            "*bad/<name>:with\\characters?|*"
            t)
           (apheleia--safe-buffer-name))
         (apheleia--replq
          '("formatter" input "--out" output input)
          '(input output)
          "/fixed/path")
         (let ((original
                '("formatter" file "--check")))
           (list
            (apheleia--replq
             original
             'file
             "/source")
            original)))"##;
    let expect = expect![[
        r#"OK (("/workspace/source.el" "/srv/project/source.el" "/etc/config" nil) "badnamewithcharacters" ("formatter" "/fixed/path" "--out" "/fixed/path" "/fixed/path") (("formatter" "/source" "--check") ("formatter" file "--check")))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}
