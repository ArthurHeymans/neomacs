use expect_test::expect;

use super::assert_auth_source_keytar_parity;

#[test]
fn auth_source_keytar_backend_parse_builds_exact_keytar_backend_contract() {
    let elisp_form = r##"(let ((backend
                                (auth-source-keytar-backend-parse
                                 'keytar)))
          (list
           (auth-source-keytar-test-backend-data
            backend)
           (auth-source-backend-p backend)
           (eq
            (slot-value backend 'search-function)
            #'auth-source-keytar-search)))"##;
    let expect = expect![[r#"OK (("Keytar" keytar auth-source-keytar-search) t t)"#]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_backend_parse_rejects_every_non_keytar_entry_without_side_effects() {
    let elisp_form = r##"(let (calls)
          (cl-letf
              (((symbol-function
                 'auth-source-backend-parse-parameters)
                (lambda (&rest arguments)
                  (push arguments calls)
                  :unexpected)))
            (list
             (mapcar
              (lambda (entry)
                (list
                 entry
                 (auth-source-keytar-backend-parse
                  entry)))
              '(nil
                "keytar"
                KEYTAR
                (keytar)
                (:source keytar)
                keytar-config
                0))
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (((nil nil) ("keytar" nil) (KEYTAR nil) ((keytar) nil) ((:source keytar) nil) (keytar-config nil) (0 nil)) nil)"#
    ]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_backend_parse_forwards_entry_and_unmodified_backend_to_parameter_parser() {
    let elisp_form = r##"(let (calls)
          (cl-letf
              (((symbol-function
                 'auth-source-backend-parse-parameters)
                (lambda (entry backend)
                  (push
                   (list
                    entry
                    (auth-source-keytar-test-backend-data
                     backend))
                   calls)
                   (list
                    :parsed
                    entry
                    (slot-value backend 'source)))))
            (list
             (auth-source-keytar-backend-parse
              'keytar)
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((:parsed keytar "Keytar") ((keytar ("Keytar" keytar auth-source-keytar-search))))"#
    ]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_backend_parse_propagates_parameter_parser_failure_after_backend_construction()
{
    let elisp_form = r##"(let (observed)
          (cl-letf
              (((symbol-function
                 'auth-source-backend-parse-parameters)
                (lambda (entry backend)
                  (setq observed
                        (list
                         entry
                         (auth-source-keytar-test-backend-data
                          backend)))
                  (error
                   "fixture backend parser failed"))))
            (list
             (auth-source-keytar-test-error-data
              (lambda ()
                (auth-source-keytar-backend-parse
                 'keytar)))
             observed)))"##;
    let expect = expect![[
        r#"OK ((:error error ("fixture backend parser failed")) (keytar ("Keytar" keytar auth-source-keytar-search)))"#
    ]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_load_registers_parser_once_in_modern_auth_source_hook() {
    let elisp_form = r##"(list
          (boundp
           'auth-source-backend-parser-functions)
          (memq
           #'auth-source-keytar-backend-parse
           auth-source-backend-parser-functions)
          (length
           (seq-filter
            (lambda (function)
              (eq
               function
               #'auth-source-keytar-backend-parse))
            auth-source-backend-parser-functions))
          (advice-member-p
           #'auth-source-keytar-backend-parse
           'auth-source-backend-parse))"##;
    let expect = expect![
        "OK (t (auth-source-keytar-backend-parse auth-source-backends-parser-secrets auth-source-backends-parser-macos-keychain auth-source-backends-parser-file) 1 nil)"
    ];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_registered_hook_parses_keytar_and_declines_other_sources() {
    let elisp_form = r##"(list
          (auth-source-keytar-test-backend-data
           (run-hook-with-args-until-success
            'auth-source-backend-parser-functions
            'keytar))
          (run-hook-with-args-until-success
           'auth-source-backend-parser-functions
           "fixture.authinfo")
          (run-hook-with-args-until-success
           'auth-source-backend-parser-functions
           'unknown-backend))"##;
    let expect = expect![[
        r#"OK (("Keytar" keytar auth-source-keytar-search) #s(auth-source-backend ignore "" t t t nil ignore ignore) #s(auth-source-backend ignore "" t t t nil ignore ignore))"#
    ]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_real_auth_source_backend_parser_accepts_keytar_source() {
    let elisp_form = r##"(let ((backend
                                (auth-source-backend-parse
                                 'keytar)))
          (list
           (auth-source-keytar-test-backend-data
            backend)
           (auth-source-backend-p backend)))"##;
    let expect = expect![[r#"OK (("Keytar" keytar auth-source-keytar-search) t)"#]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_source_reloads_do_not_duplicate_modern_parser_hook() {
    let elisp_form = r##"(let ((source
                                (getenv
                                 "NEOMACS_PACKAGE_SOURCE")))
          (load source nil t t)
          (load source nil t t)
          (load source nil t t)
          (list
           (length
            (seq-filter
             (lambda (function)
               (eq
                function
                #'auth-source-keytar-backend-parse))
             auth-source-backend-parser-functions))
           (auth-source-keytar-test-backend-data
            (run-hook-with-args-until-success
             'auth-source-backend-parser-functions
             'keytar))))"##;
    let expect = expect![[r#"OK (1 ("Keytar" keytar auth-source-keytar-search))"#]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_legacy_reload_uses_before_until_advice_when_parser_hook_is_unbound() {
    let elisp_form = r##"(let ((source
                                (getenv
                                 "NEOMACS_PACKAGE_SOURCE")))
          (remove-hook
           'auth-source-backend-parser-functions
           #'auth-source-keytar-backend-parse)
          (makunbound
           'auth-source-backend-parser-functions)
          (load source nil t t)
          (let ((backend
                 (auth-source-backend-parse
                  'keytar)))
            (list
             (boundp
              'auth-source-backend-parser-functions)
             (and
              (advice-member-p
               #'auth-source-keytar-backend-parse
               'auth-source-backend-parse)
              t)
             (auth-source-keytar-test-backend-data
              backend))))"##;
    let expect = expect![[r#"OK (nil t ("Keytar" keytar auth-source-keytar-search))"#]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}
