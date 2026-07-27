use expect_test::expect;

use super::assert_affe_parity;

#[test]
fn affe_command_appends_paths_when_command_has_no_placeholder() {
    let elisp_form = r##"(list
               (affe--command
                "rg --color=never --files"
                '("src" "docs/space name"))
               (affe--command "find -type f" nil)
               (affe--command "printf \"%s\\n\""
                              '("α" "β")))"##;
    let expect = expect![[
        r#"OK (("rg" "--color=never" "--files" "src" "docs/space name") ("find" "-type" "f") ("printf" "%s\n" "α" "β"))"#
    ]];
    assert_affe_parity(elisp_form, expect);
}

#[test]
fn affe_command_expands_every_dot_placeholder_and_preserves_quoted_arguments() {
    let elisp_form = r##"(list
               (affe--command
                "rg --glob \"space name\" . --and ."
                '("one" "two words"))
               (affe--command "."
                              '("a" "b"))
               (affe--command
                "tool --literal=./child ."
                '("root")))"##;
    let expect = expect![[
        r#"OK (("rg" "--glob" "space name" "one" "two words" "--and" "one" "two words") ("a" "b") ("tool" "--literal=./child" "root"))"#
    ]];
    assert_affe_parity(elisp_form, expect);
}

#[test]
fn affe_command_reports_split_errors_and_boundary_argument_types() {
    let elisp_form = r##"(mapcar
               (lambda (case)
                 (condition-case error-data
                     (apply #'affe--command case)
                   (error
                    (list 'signal
                          (car error-data)
                          (cdr error-data)))))
               '(("unterminated \"quote" ("root"))
                 (nil ("root"))
                 ("rg ." nil)))"##;
    let expect = expect![[
        r#"OK ((signal end-of-file nil) (signal wrong-type-argument (stringp nil)) ("rg"))"#
    ]];
    assert_affe_parity(elisp_form, expect);
}
