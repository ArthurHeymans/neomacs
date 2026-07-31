use expect_test::expect;

use super::assert_auth_source_keytar_batch;

#[test]
fn parsing_public_surface_batch() {
    assert_auth_source_keytar_batch(&[
        (
            "auth_source_keytar_read_password_extracts_real_keytar_credential_rendering",
            r##"(mapcar
          #'auth-source-keytar--read-password
          '("{ account: 'alice', password: 'correct horse battery staple' }"
            "{ account: 'build-bot', password: 'token-123_ABC' }"
            "{ account: 'unicode', password: 'pässwörd-密钥' }"))"##,
            true,
            expect![[r#"OK ("correct horse battery staple" "token-123_ABC" "pässwörd-密钥")"#]],
        ),
        (
            "auth_source_keytar_read_password_trims_outer_whitespace_but_preserves_internal_content",
            r##"(mapcar
          #'auth-source-keytar--read-password
          '("password: '   surrounded   ' }"
            "prefix password: 'two  internal  spaces' } suffix"
            "\tpassword: '\tTabbed Secret\t' }\t"))"##,
            true,
            expect![[r#"OK ("surrounded" "two  internal  spaces suffix" "Tabbed Secret")"#]],
        ),
        (
            "auth_source_keytar_read_password_uses_first_marker_and_globally_removes_closing_fragment",
            r##"(mapcar
          #'auth-source-keytar--read-password
          '("password: 'first' } password: 'second' }"
            "password: 'left' }middle' }right' }"
            "prefix password: 'one' }\npassword: 'two' }"))"##,
            true,
            expect![[r#"OK ("first" "leftmiddleright" "one")"#]],
        ),
        (
            "auth_source_keytar_read_password_reports_exact_malformed_and_missing_marker_errors",
            r##"(mapcar
          (lambda (secret)
            (list
             secret
             (auth-source-keytar-test-error-data
              (lambda ()
                (auth-source-keytar--read-password secret)))))
          '(""
            "password"
            "password:"
            "password: no quote"
            "{ account: 'alice' }"
            "PASSWORD: 'uppercase' }"))"##,
            true,
            expect![[
        r#"OK (("" (:error wrong-type-argument (arrayp nil))) ("password" (:error wrong-type-argument (arrayp nil))) ("password:" (:error wrong-type-argument (arrayp nil))) ("password: no quote" (:error wrong-type-argument (arrayp nil))) ("{ account: 'alice' }" (:error wrong-type-argument (arrayp nil))) ("PASSWORD: 'uppercase' }" (:ok "uppercase")))"#
    ]],
        ),
        (
            "auth_source_keytar_read_password_rejects_non_string_secrets_with_exact_signals",
            r##"(mapcar
          (lambda (secret)
            (list
             secret
             (auth-source-keytar-test-error-data
              (lambda ()
                (auth-source-keytar--read-password secret)))))
          '(nil
            password-symbol
            42
            ("password: 'nested' }")
            [112 97 115 115]))"##,
            true,
            expect![[
        r#"OK ((nil (:error wrong-type-argument (stringp nil))) (password-symbol (:error wrong-type-argument (sequencep password-symbol))) (42 (:error wrong-type-argument (sequencep 42))) (#1=("password: 'nested' }") (:error wrong-type-argument (stringp #1#))) (#2=[112 97 115 115] (:error wrong-type-argument (stringp #2#))))"#
    ]],
        ),
        (
            "auth_source_keytar_build_result_parses_multiline_keytar_output_and_reverses_provider_order",
            r##"(let (calls)
          (cl-letf
              (((symbol-function 'keytar-find-credentials)
                (lambda (service)
                  (push service calls)
                  "[\n  { account: 'alice', password: 'alpha secret' },\n  { account: 'bob', password: 'beta-secret' },\n  { account: 'ci', password: 'token-三' }\n]")))
            (list
             (auth-source-keytar--build-result
              "production/api")
             (nreverse calls))))"##,
            true,
            expect![[
        r#"OK (((:secret "token-三") (:secret "beta-secret") (:secret "alpha secret")) ("production/api"))"#
    ]],
        ),
        (
            "auth_source_keytar_build_result_empty_and_whitespace_outputs_have_exact_nil_or_error_contracts",
            r##"(mapcar
          (lambda (output)
            (cl-letf
                (((symbol-function 'keytar-find-credentials)
                  (lambda (_)
                    output)))
              (list
               output
               (auth-source-keytar-test-error-data
                (lambda ()
                  (auth-source-keytar--build-result
                   "empty-service"))))))
          '("[]"
            "[\n]"
            ""
            "\n\n"
            "[ \n \t\n ]"))"##,
            true,
            expect![[
        r#"OK (("[]" (:ok nil)) ("[\n]" (:ok nil)) ("" (:ok nil)) ("\n\n" (:ok nil)) ("[ \n \11\n ]" (:error wrong-type-argument (arrayp nil))))"#
    ]],
        ),
        (
            "auth_source_keytar_build_result_preserves_empty_unicode_and_shell_like_passwords_as_data",
            r##"(cl-letf
          (((symbol-function 'keytar-find-credentials)
            (lambda (_)
              "[\n{ account: 'empty', password: '' },\n{ account: 'unicode', password: '密钥🔐' },\n{ account: 'shell', password: '$(touch nope); $HOME & spaces' }\n]")))
          (auth-source-keytar--build-result
           "special-secrets"))"##,
            true,
            expect![[
        r#"OK ((:secret "$(touch nope); $HOME & spaces") (:secret "密钥🔐") (:secret ""))"#
    ]],
        ),
        (
            "auth_source_keytar_build_result_single_line_provider_output_yields_only_first_embedded_password",
            r##"(cl-letf
          (((symbol-function 'keytar-find-credentials)
            (lambda (_)
              "[{ account: 'one', password: 'first' }, { account: 'two', password: 'second' }]")))
          (auth-source-keytar--build-result
           "single-line"))"##,
            true,
            expect![[r#"OK ((:secret "first, { account: 'two',"))"#]],
        ),
        (
            "auth_source_keytar_build_result_blank_lines_surface_failure_before_trailing_comma_cleanup",
            r##"(cl-letf
          (((symbol-function 'keytar-find-credentials)
            (lambda (_)
              "[\n\n { account: 'one', password: 'first,inside' },\n\t\n { account: 'two', password: 'second' },   \n\n]")))
          (auth-source-keytar-test-error-data
           (lambda ()
             (auth-source-keytar--build-result
              "formatting"))))"##,
            true,
            expect!["OK (:error wrong-type-argument (arrayp nil))"],
        ),
        (
            "auth_source_keytar_build_result_propagates_provider_failures_and_non_string_results",
            r##"(mapcar
          (lambda (case)
            (cl-letf
                (((symbol-function 'keytar-find-credentials)
                  (lambda (_)
                    (pcase case
                      ('provider-error
                       (error "fixture provider failed"))
                      (_ case)))))
              (list
               case
               (auth-source-keytar-test-error-data
                (lambda ()
                  (auth-source-keytar--build-result
                   "service"))))))
          '(provider-error
            nil
            17
            credential-symbol
            ("list-result")))"##,
            true,
            expect![[
        r#"OK ((provider-error (:error error ("fixture provider failed"))) (nil (:error wrong-type-argument (arrayp nil))) (17 (:error wrong-type-argument (sequencep 17))) (credential-symbol (:error wrong-type-argument (sequencep credential-symbol))) (#1=("list-result") (:error wrong-type-argument (stringp #1#))))"#
    ]],
        ),
    ]);
}
