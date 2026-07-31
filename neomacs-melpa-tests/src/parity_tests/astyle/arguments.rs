use super::assert_astyle_batch;
use expect_test::{Expect, expect};

#[test]
fn arguments_public_surface_batch() {
    assert_astyle_batch(&[
        (
            "format_args_use_style_c_mode_offset_and_complete_default_option_set",
            r##"
(with-temp-buffer
  (setq buffer-file-name
        (astyle-test-path
         "arguments/default/source.c")
        default-directory
        (file-name-as-directory
         (astyle-test-path
          "arguments/default"))
        c-basic-offset 3
        astyle-style "linux"
        astyle-indent nil
        astyle-custom-args nil)
  (make-directory default-directory t)
  (list
   (astyle--format-args)
   astyle-default-args
   (equal
    (cddr
     (astyle--format-args))
    astyle-default-args)))
"##,
            true,
            expect![[
        r#"OK (("--style=linux" "--indent=spaces=3" . #1=("--pad-oper" "--pad-header" "--break-blocks" "--delete-empty-lines" "--align-pointer=type" "--align-reference=name")) #1# t)"#
    ]],
        ),
        (
            "format_args_prefer_explicit_indent_and_custom_arguments_in_declared_order",
            r##"
(with-temp-buffer
  (setq buffer-file-name
        (astyle-test-path
         "arguments/custom/source.cpp")
        default-directory
        (file-name-as-directory
         (astyle-test-path
          "arguments/custom"))
        c-basic-offset 2
        astyle-style "allman"
        astyle-indent 8
        astyle-custom-args
        '("--suffix=none"
          "--convert-tabs"
          "--max-code-length=88"))
  (make-directory default-directory t)
  (astyle--format-args))
"##,
            true,
            expect![[
        r#"OK ("--style=allman" "--indent=spaces=8" "--suffix=none" "--convert-tabs" "--max-code-length=88")"#
    ]],
        ),
        (
            "format_args_are_recomputed_from_buffer_local_settings_for_each_buffer",
            r##"
(let ((first
       (generate-new-buffer
        " *astyle-args-first*"))
      (second
       (generate-new-buffer
        " *astyle-args-second*")))
  (unwind-protect
      (progn
        (with-current-buffer first
          (setq buffer-file-name
                (astyle-test-path
                 "arguments/local/first.c")
                default-directory
                (file-name-as-directory
                 (astyle-test-path
                  "arguments/local"))
                c-basic-offset 2)
          (setq-local
           astyle-style "google")
          (setq-local
           astyle-custom-args
           '("--first")))
        (with-current-buffer second
          (setq buffer-file-name
                (astyle-test-path
                 "arguments/local/second.c")
                default-directory
                (file-name-as-directory
                 (astyle-test-path
                  "arguments/local"))
                c-basic-offset 4)
          (setq-local
           astyle-style "java")
          (setq-local
           astyle-indent 6)
          (setq-local
           astyle-custom-args
           '("--second" "--third")))
        (make-directory
         (with-current-buffer first
           default-directory)
         t)
        (list
         (with-current-buffer first
           (astyle--format-args))
         (with-current-buffer second
           (astyle--format-args))))
    (kill-buffer first)
    (kill-buffer second)))
"##,
            true,
            expect![[
        r#"OK (("--style=google" "--indent=spaces=4" "--first") ("--style=java" "--indent=spaces=6" "--second" "--third"))"#
    ]],
        ),
        (
            "project_configuration_file_overrides_style_indent_and_custom_arguments",
            r##"
(let* ((project
        (file-name-as-directory
         (astyle-test-path
          "arguments/project")))
       (source-directory
        (expand-file-name
         "src/nested/"
         project))
       (configuration
        (expand-file-name
         ".astylerc"
         project)))
  (make-directory source-directory t)
  (with-temp-file configuration
    (insert
     "style=mozilla\nindent=spaces=7\n"))
  (with-temp-buffer
    (setq buffer-file-name
          (expand-file-name
           "widget.cpp"
           source-directory)
          default-directory
          source-directory
          c-basic-offset 2
          astyle-style "google"
          astyle-indent 4
          astyle-custom-args
          '("--suffix=none"))
    (list
     (astyle--format-args)
     (current-message)
     (file-truename
      configuration))))
"##,
            true,
            expect![[
        r#"OK (("--options=[ORACLE-SANDBOX]/arguments/project/.astylerc") nil "[ORACLE-SANDBOX]/arguments/project/.astylerc")"#
    ]],
        ),
        (
            "nearest_configuration_and_custom_rc_name_win_in_nested_projects",
            r##"
(let* ((root
        (file-name-as-directory
         (astyle-test-path
          "arguments/nearest")))
       (nested
        (expand-file-name
         "module/deep/"
         root))
       (root-config
        (expand-file-name
         "style.conf"
         root))
       (module-config
        (expand-file-name
         "style.conf"
         (expand-file-name
          "module/"
          root))))
  (make-directory nested t)
  (with-temp-file root-config
    (insert "style=linux\n"))
  (with-temp-file module-config
    (insert "style=java\n"))
  (with-temp-buffer
    (setq buffer-file-name
          (expand-file-name
           "deep.c"
           nested)
          default-directory nested
          c-basic-offset 2
          astyle-default-rc-name
          "style.conf")
    (list
     (astyle--format-args)
     (file-truename
      module-config)
     (file-truename
      root-config))))
"##,
            true,
            expect![[
        r#"OK (("--options=[ORACLE-SANDBOX]/arguments/nearest/module/style.conf") "[ORACLE-SANDBOX]/arguments/nearest/module/style.conf" "[ORACLE-SANDBOX]/arguments/nearest/style.conf")"#
    ]],
        ),
        (
            "unsaved_buffer_uses_default_arguments_when_no_configuration_is_addressable",
            r##"
(with-temp-buffer
  (setq default-directory
        (file-name-as-directory
         (astyle-test-path
          "arguments/unsaved"))
        buffer-file-name nil
        c-basic-offset 5
        astyle-style "stroustrup"
        astyle-indent nil
        astyle-custom-args
        '("--keep-one-line-blocks"))
  (make-directory default-directory t)
  (condition-case condition
      (list
       :value
       (astyle--format-args))
    (error
     (list
      :signal
      (car condition)
      (cadr condition)))))
"##,
            true,
            expect!["OK (:signal wrong-type-argument stringp)"],
        ),
        (
            "missing_indent_sources_report_the_exact_conversion_error",
            r##"
(with-temp-buffer
  (setq buffer-file-name
        (astyle-test-path
         "arguments/missing-indent/source.c")
        default-directory
        (file-name-as-directory
         (astyle-test-path
          "arguments/missing-indent"))
        c-basic-offset nil
        astyle-indent nil)
  (make-directory default-directory t)
  (condition-case condition
      (list
       :value
       (astyle--format-args))
    (error
     (list
      :signal
      (car condition)
      (cdr condition)))))
"##,
            true,
            expect!["OK (:signal wrong-type-argument (numberp nil))"],
        ),
    ]);
}
