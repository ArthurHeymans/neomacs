use expect_test::expect;

use super::{assert_ac_dcd_parity, assert_ac_dcd_signal_parity};

#[test]
fn ac_dcd_completion_parser_filters_pattern_merges_duplicates_and_keeps_properties() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "Pattern\tv\n"
                "alpha\tf\n"
                "alpha\tv\n"
                "beta\tc\n"
                "malformed\n"
                "gamma\tT\n")
               (let ((items
                      (ac-dcd-parse-output
                       "unused"
                       (current-buffer))))
                 (mapcar
                  (lambda (item)
                    (list
                     (substring-no-properties item)
                     (get-text-property
                      0 'ac-dcd-help item)
                     (text-properties-at 0 item)))
                  items)))"##;
    let expect = expect![[
        r#"OK (("gamma" "T" (ac-dcd-help "T")) ("beta" "c" (ac-dcd-help "c")) ("alpha" "f\nv" (ac-dcd-help "f\nv")))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_completion_parser_handles_empty_unknown_and_nonconsecutive_duplicates() {
    let elisp_form = r##"(list
               (with-temp-buffer
                 (ac-dcd-parse-output
                  nil
                  (current-buffer)))
               (with-temp-buffer
                 (insert
                  "one\tf\n"
                  "two\tv\n"
                  "one\tm\n"
                  "ignored\tx\n")
                 (mapcar
                  (lambda (item)
                    (cons
                     (substring-no-properties item)
                     (get-text-property
                      0 'ac-dcd-help item)))
                  (ac-dcd-parse-output
                   nil
                   (current-buffer)))))"##;
    let expect = expect![[r#"OK (nil (("one" . "m") ("two" . "v") ("one" . "f")))"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_calltip_helpers_distinguish_and_cleanup_functions_and_templates() {
    let elisp_form = r##"(mapcar
               (lambda (candidate)
                 (list
                  candidate
                  (ac-dcd-candidate-is-tempalte-p
                   candidate)
                  (condition-case error-data
                      (ac-dcd-cleanup-function-candidate
                       candidate)
                    (error
                     (cons :error error-data)))
                  (condition-case error-data
                      (ac-dcd-cleanup-template-candidate
                       candidate)
                    (error
                     (cons :error error-data)))))
               '("int foo(int x, string y)"
                 "pkg.Type bar()"
                 "void templ(T)(T value)"
                 "plain()"))"##;
    let expect = expect![[
        r#"OK (("int foo(int x, string y)" nil "foo(int x, string y)" "foo(int x, string y)") ("pkg.Type bar()" nil "bar()" "bar()") ("void templ(T)(T value)" t "templ(T)(T value)" "templ(T)(T value)") ("plain()" nil "plain()" "plain()"))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_calltip_parser_covers_normal_template_overload_and_ignore_policies() {
    let elisp_form = r##"(let ((fixture
                    (concat
                     "int foo(int x)\n"
                     "void templ(T)(T value)\n"
                     "string foo(string x)\n"
                     "not a calltip\n")))
               (list
                (with-temp-buffer
                  (insert fixture)
                  (let
                      ((ac-dcd-ignore-template-argument
                        t))
                    (ac-dcd-parse-calltips)))
                (with-temp-buffer
                  (insert fixture)
                  (let
                      ((ac-dcd-ignore-template-argument
                        nil))
                    (ac-dcd-parse-calltips)))))"##;
    let expect = expect![[
        r#"OK (("foo(string x)" "templ(T value)" "foo(int x)") ("foo(string x)" "templ!(T)(T value)" "templ(T value)" "foo(int x)"))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_calltip_formatter_preserves_argument_text_and_empty_argument_quirk() {
    let elisp_form = r##"(mapcar
               #'ac-dcd-format-calltips
               '("()"
                 "(int x)"
                 "(int x, string y)"
                 "(scope void delegate() dg, int[] xs)"))"##;
    let expect = expect![[
        r#"OK ("(${})" "(${int x})" "(${int x}, ${string y})" "(${scope void delegate() dg}, ${int[] xs})")"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_document_reformatter_decodes_newlines_but_preserves_escaped_source_text() {
    let elisp_form = r##"(let ((buffer
                    (get-buffer-create
                     ac-dcd-document-buffer-name)))
               (unwind-protect
                   (progn
                     (with-current-buffer buffer
                       (erase-buffer)
                       (insert
                        "first\\nsecond\\n\\nthird\\\\nlast"))
                     (ac-dcd-reformat-document)
                     (with-current-buffer buffer
                       (list
                        (buffer-string)
                        (point))))
                 (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ("first\nsecond\n\nthird\\nlast" 1)"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_symbol_location_parser_covers_found_not_found_and_unicode_paths() {
    let elisp_form = r##"(let ((buffer
                    (get-buffer-create
                     ac-dcd-output-buffer-name)))
               (unwind-protect
                   (list
                    (with-current-buffer buffer
                      (erase-buffer)
                      (insert
                       "/workspace/λ/module.d\t63946\n")
                      (ac-dcd-parse-output-for-get-symbol-declaration))
                    (with-current-buffer buffer
                      (erase-buffer)
                      (insert "Not found\n")
                      (ac-dcd-parse-output-for-get-symbol-declaration)))
                 (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK (("/workspace/λ/module.d" . "63946") (nil))"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_symbol_location_parser_signals_on_malformed_output() {
    let elisp_form = r##"(let ((buffer
                    (get-buffer-create
                     ac-dcd-output-buffer-name)))
               (unwind-protect
                   (with-current-buffer buffer
                     (erase-buffer)
                     (insert "malformed\n")
                     (ac-dcd-parse-output-for-get-symbol-declaration))
                 (kill-buffer buffer)))"##;
    let expect = expect![[r#"ERR (search-failed "\\(.*\\)\11\\(.*\\)\n")"#]];

    assert_ac_dcd_signal_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_version_parser_populates_and_reuses_the_numeric_cache() {
    let elisp_form = r##"(let ((ac-dcd-version nil)
                    calls)
               (cl-letf
                   (((symbol-function
                      'ac-dcd-call-process)
                     (lambda (args)
                       (push args calls)
                       (with-current-buffer
                           (get-buffer-create
                            ac-dcd-output-buffer-name)
                         (erase-buffer)
                         (insert
                          "DCD client v0.15.2-beta\n")))))
                 (unwind-protect
                     (list
                      (ac-dcd-get-version)
                      (ac-dcd-get-version)
                      ac-dcd-version
                      (nreverse calls))
                   (when
                       (get-buffer
                        ac-dcd-output-buffer-name)
                     (kill-buffer
                      ac-dcd-output-buffer-name)))))"##;
    let expect = expect![[r#"OK (0.15 0.15 0.15 (("--version")))"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}
