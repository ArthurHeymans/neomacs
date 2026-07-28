use expect_test::expect;

use super::assert_auth_source_kwallet_parity;

#[test]
fn auth_source_kwallet_parser_builds_exact_kwallet_backend_contract() {
    let elisp_form = r##"(let ((backend
                                (auth-source-kwallet--kwallet-backend-parse
                                 'kwallet)))
                           (auth-source-kwallet-test-backend
                            backend))"##;
    let expect = expect![[
        r#"OK (auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore)"#
    ]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}

#[test]
fn auth_source_kwallet_parser_rejects_every_similar_but_nonidentical_entry_shape() {
    let elisp_form = r##"(mapcar
                          (lambda (entry)
                            (list
                             entry
                             (auth-source-kwallet--kwallet-backend-parse
                              entry)))
                          '(nil
                            "kwallet"
                            :kwallet
                            (kwallet)
                            (:source kwallet)
                            (:type kwallet)
                            kwallet-query
                            KWallet))"##;
    let expect = expect![[
        r#"OK ((nil nil) ("kwallet" nil) (:kwallet nil) ((kwallet) nil) ((:source kwallet) nil) ((:type kwallet) nil) (kwallet-query nil) (KWallet nil))"#
    ]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}

#[test]
fn auth_source_kwallet_enable_adds_source_advice_and_forgets_existing_auth_cache() {
    let elisp_form = r##"(let* ((spec
                                 '(:host
                                   "cached.example"
                                   :user
                                   "old-user"))
                                (_
                                 (auth-source-remember
                                  spec
                                  '((:user
                                     "old-user"
                                     :secret
                                     "old-secret"))))
                                (cached-before
                                 (list
                                  (auth-source-remembered-p spec)
                                  (auth-source-recall spec)))
                                (result
                                 (progn
                                   (setq auth-sources
                                         '("primary.authinfo"))
                                   (auth-source-kwallet-enable))))
                           (list
                            result
                            cached-before
                            (auth-source-remembered-p spec)
                            (auth-source-recall spec)
                            auth-sources
                            (and
                             (advice-member-p
                              #'auth-source-kwallet--kwallet-backend-parse
                              'auth-source-backend-parse)
                             t)
                            (auth-source-kwallet-test-backend
                             (auth-source-backend-parse
                              'kwallet))))"##;
    let expect = expect![[
        r#"OK (nil (t ((:user "old-user" :secret "old-secret"))) nil nil (kwallet "primary.authinfo") t (auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore))"#
    ]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}

#[test]
fn auth_source_kwallet_repeated_enable_is_idempotent_for_source_and_advice() {
    let elisp_form = r##"(progn
                          (auth-source-kwallet-test-enable-clean)
                          (auth-source-kwallet-enable)
                          (auth-source-kwallet-enable)
                          (let ((matching-advice 0)
                                (all-advice nil))
                            (advice-mapc
                             (lambda (advice properties)
                               (push
                                (list advice properties)
                                all-advice)
                               (when
                                   (eq
                                    advice
                                    #'auth-source-kwallet--kwallet-backend-parse)
                                 (setq matching-advice
                                       (1+ matching-advice))))
                             'auth-source-backend-parse)
                            (list
                             auth-sources
                             matching-advice
                             (length all-advice)
                             (auth-source-kwallet-test-backend
                              (auth-source-backend-parse
                               'kwallet)))))"##;
    let expect = expect![[
        r#"OK ((kwallet) 1 1 (auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore))"#
    ]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}

#[test]
fn auth_source_kwallet_enable_preserves_existing_kwallet_position_in_mixed_sources() {
    let elisp_form = r##"(let ((auth-sources
                                '("first.authinfo"
                                  kwallet
                                  "last.authinfo")))
                           (auth-source-kwallet-enable)
                           (list
                            auth-sources
                            (mapcar
                             (lambda (backend)
                               (list
                                (slot-value backend 'source)
                                (slot-value backend 'type)))
                             (auth-source-backends))))"##;
    let expect =
        expect![[r#"OK (("first.authinfo" kwallet "last.authinfo") (("KWallet" kwallet)))"#]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}

#[test]
fn auth_source_kwallet_advised_core_parser_handles_kwallet_and_leaves_other_entries_to_core() {
    let elisp_form = r##"(progn
                          (auth-source-kwallet-test-enable-clean)
                          (let ((auth-source-ignore-non-existing-file
                                 t))
                            (mapcar
                             (lambda (entry)
                               (list
                                entry
                                (auth-source-kwallet-test-backend
                                 (auth-source-backend-parse
                                  entry))))
                             '(kwallet
                               "missing.authinfo"
                               :kwallet
                               (:source
                                "also-missing.authinfo")))))"##;
    let expect = expect![[
        r#"OK ((kwallet (auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore)) ("missing.authinfo" (auth-source-backend "" ignore t t t ignore ignore)) (:kwallet (auth-source-backend "" ignore t t t ignore ignore)) ((:source "also-missing.authinfo") (auth-source-backend "" ignore t t t ignore ignore)))"#
    ]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}

#[test]
fn auth_source_kwallet_advice_removal_disables_parsing_without_mutating_sources() {
    let elisp_form = r##"(progn
                          (auth-source-kwallet-test-enable-clean)
                          (let ((before
                                 (auth-source-kwallet-test-backend
                                  (auth-source-backend-parse
                                   'kwallet))))
                            (advice-remove
                             'auth-source-backend-parse
                             #'auth-source-kwallet--kwallet-backend-parse)
                            (list
                             before
                             auth-sources
                             (auth-source-kwallet-test-backend
                              (auth-source-backend-parse
                               'kwallet))
                             (and
                              (advice-member-p
                               #'auth-source-kwallet--kwallet-backend-parse
                               'auth-source-backend-parse)
                              t))))"##;
    let expect = expect![[
        r#"OK ((auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore) (kwallet) (auth-source-backend "" ignore t t t ignore ignore) nil)"#
    ]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}

#[test]
fn auth_source_kwallet_backend_equality_and_seq_deduplication_match_core_auth_source() {
    let elisp_form = r##"(progn
                          (auth-source-kwallet-test-enable-clean)
                          (setq auth-sources
                                '(kwallet
                                  kwallet
                                  kwallet))
                          (let ((parsed
                                 (mapcar
                                  #'auth-source-backend-parse
                                  auth-sources))
                                (backends
                                 (auth-source-backends)))
                            (list
                             (length parsed)
                             (mapcar
                              #'auth-source-kwallet-test-backend
                              parsed)
                             (length backends)
                             (mapcar
                              #'auth-source-kwallet-test-backend
                              backends))))"##;
    let expect = expect![[
        r#"OK (3 ((auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore) (auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore) (auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore)) 1 ((auth-source-backend "KWallet" kwallet t t t auth-source-kwallet--kwallet-search ignore)))"#
    ]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}

#[test]
fn auth_source_kwallet_backend_parameter_parser_can_apply_host_user_and_port_constraints() {
    let elisp_form = r##"(let* ((backend
                                 (auth-source-kwallet--kwallet-backend-parse
                                  'kwallet))
                                (configured
                                 (auth-source-backend-parse-parameters
                                  '(:host
                                    "api.example"
                                    :user
                                    "deploy"
                                    :port
                                    "https")
                                  backend)))
                           (auth-source-kwallet-test-backend
                            configured))"##;
    let expect = expect![[
        r#"OK (auth-source-backend "KWallet" kwallet "api.example" "deploy" "https" auth-source-kwallet--kwallet-search ignore)"#
    ]];

    assert_auth_source_kwallet_parity(elisp_form, expect);
}
