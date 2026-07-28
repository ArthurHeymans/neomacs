use expect_test::expect;

use super::assert_auth_source_gopass_parity;

#[test]
fn auth_source_gopass_enable_prepends_source_and_flushes_cache_once() {
    let elisp_form = r##"(let ((auth-sources
                '("~/.authinfo" default))
               calls)
         (cl-letf
             (((symbol-function
                'auth-source-forget-all-cached)
               (lambda ()
                 (push :forget calls)
                 :forgotten)))
           (list
            (auth-source-gopass-enable)
            auth-sources
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (:forgotten (gopass "~/.authinfo" default) (:forget))"#]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_enable_deduplicates_existing_source_in_any_position() {
    let elisp_form = r##"(mapcar
         (lambda (initial)
           (let ((auth-sources
                  (copy-sequence initial))
                 calls)
             (cl-letf
                 (((symbol-function
                    'auth-source-forget-all-cached)
                   (lambda ()
                     (push :forget calls))))
               (auth-source-gopass-enable)
               (auth-source-gopass-enable)
               (list
                auth-sources
                (nreverse calls)))))
         '(nil
           (gopass)
           ("~/.authinfo" gopass default)
           (gopass "~/.authinfo" gopass)))"##;
    let expect = expect![[
        r#"OK (((gopass) (:forget :forget)) ((gopass) (:forget :forget)) (("~/.authinfo" gopass default) (:forget :forget)) ((gopass "~/.authinfo" gopass) (:forget :forget)))"#
    ]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_real_auth_source_search_resolves_credential() {
    let elisp_form = r##"(let ((auth-sources
                '(gopass))
               commands)
         (auth-source-forget-all-cached)
         (cl-letf
             (((symbol-function 'executable-find)
               (lambda (_program)
                 "/fixture/bin/gopass"))
              ((symbol-function 'shell-command-to-string)
               (lambda (command)
                 (push command commands)
                 "integration-secret\n")))
           (let ((matches
                  (auth-source-search
                   :host "smtp.example"
                   :user "alice@example"
                   :port 587
                   :require '(:user :secret)
                   :max 1)))
             (list
              matches
              (mapcar
               (lambda (entry)
                 (list
                  (plist-get entry :user)
                  (plist-get entry :secret)))
               matches)
              (nreverse commands)))))"##;
    let expect = expect![[
        r#"OK (((:user "alice@example" :secret "integration-secret")) (("alice@example" "integration-secret")) ("gopass show -o accounts/smtp.example/alice\\@example"))"#
    ]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_real_password_lookup_returns_secret() {
    let elisp_form = r##"(let ((auth-sources
                '(gopass))
               commands)
         (auth-source-forget-all-cached)
         (cl-letf
             (((symbol-function 'executable-find)
               (lambda (_program)
                 "/fixture/bin/gopass"))
              ((symbol-function 'shell-command-to-string)
               (lambda (command)
                 (push command commands)
                 "smtp-password\n")))
           (list
            (auth-source-pick-first-password
             :host "smtp.example"
             :user "alice@example"
             :port "submission")
            (nreverse commands))))"##;
    let expect = expect![[
        r#"OK ("smtp-password" ("gopass show -o accounts/smtp.example/alice\\@example"))"#
    ]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_enable_then_search_models_application_startup() {
    let elisp_form = r##"(let ((auth-sources
                '("~/.authinfo"))
               events)
         (cl-letf
             (((symbol-function 'executable-find)
               (lambda (program)
                 (push (list :find program) events)
                 "/fixture/bin/gopass"))
              ((symbol-function 'shell-command-to-string)
               (lambda (command)
                 (push (list :shell command) events)
                 "startup-secret\n")))
           (auth-source-gopass-enable)
           (let ((result
                  (auth-source-search
                   :host "imap.example"
                   :user "alice"
                   :port 993
                   :max 1)))
             (list
              auth-sources
              result
              (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ((gopass "~/.authinfo") ((:user "alice" :secret "startup-secret")) ((:find "gopass") (:shell "gopass show -o accounts/imap.example/alice")))"#
    ]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_application_can_customize_full_vault_layout() {
    let elisp_form = r##"(let ((auth-sources
                '(gopass))
               (auth-source-gopass-executable
                "gopass-company")
               (auth-source-gopass-construct-query-path
                (lambda (backend type host user port)
                  (format
                   "teams/%s/%s/%s@%s:%s"
                   (slot-value backend 'source)
                   type
                   user
                   host
                   port)))
               events)
         (auth-source-forget-all-cached)
         (cl-letf
             (((symbol-function 'executable-find)
               (lambda (program)
                 (push (list :find program) events)
                 "/fixture/bin/gopass-company"))
              ((symbol-function 'shell-command-to-string)
               (lambda (command)
                 (push (list :shell command) events)
                 "company-secret\n")))
           (list
            (auth-source-pick-first-password
             :host "smtp.internal"
             :user "alice"
             :port 465)
            (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("company-secret" ((:find "gopass-company") (:shell "gopass-company show -o teams/./gopass/alice\\@smtp.internal\\:465")))"#
    ]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}
