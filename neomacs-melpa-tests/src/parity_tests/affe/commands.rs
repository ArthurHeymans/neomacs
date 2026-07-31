use expect_test::expect;

use super::assert_affe_batch;

#[test]
fn commands_public_surface_batch() {
    assert_affe_batch(&[
        (
            "affe_command_appends_paths_when_command_has_no_placeholder",
            r##"(list
               (affe--command
                "rg --color=never --files"
                '("src" "docs/space name"))
               (affe--command "find -type f" nil)
               (affe--command "printf \"%s\\n\""
                              '("α" "β")))"##,
            true,
            expect![[
        r#"OK (("rg" "--color=never" "--files" "src" "docs/space name") ("find" "-type" "f") ("printf" "%s\n" "α" "β"))"#
    ]],
        ),
        (
            "affe_command_expands_every_dot_placeholder_and_preserves_quoted_arguments",
            r##"(list
               (affe--command
                "rg --glob \"space name\" . --and ."
                '("one" "two words"))
               (affe--command "."
                              '("a" "b"))
               (affe--command
                "tool --literal=./child ."
                '("root")))"##,
            true,
            expect![[
        r#"OK (("rg" "--glob" "space name" "one" "two words" "--and" "one" "two words") ("a" "b") ("tool" "--literal=./child" "root"))"#
    ]],
        ),
        (
            "affe_command_reports_split_errors_and_boundary_argument_types",
            r##"(mapcar
               (lambda (case)
                 (condition-case error-data
                     (apply #'affe--command case)
                   (error
                    (list 'signal
                          (car error-data)
                          (cdr error-data)))))
               '(("unterminated \"quote" ("root"))
                 (nil ("root"))
                 ("rg ." nil)))"##,
            true,
            expect![[
        r#"OK ((signal end-of-file nil) (signal wrong-type-argument (stringp nil)) ("rg"))"#
    ]],
        ),
    ]);
}
