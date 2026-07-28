use expect_test::expect;

use super::assert_auth_source_xoauth2_parity;

#[test]
fn auth_source_xoauth2_pass_find_match_prefers_three_argument_api() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'auth-source-pass--find-match)
               (lambda (&rest arguments)
                 (push arguments calls)
                 '((secret . "entry")))))
           (list
            (auth-source-xoauth2-pass--find-match
             "host" "user" 993)
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (((secret . "entry")) (("host" "user" 993)))"#]];
    assert_auth_source_xoauth2_parity(elisp_form, expect);
}

#[test]
fn auth_source_xoauth2_pass_find_match_falls_back_to_two_argument_api() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'auth-source-pass--find-match)
               (lambda (host user)
                 (push (list host user) calls)
                 '((secret . "legacy-entry")))))
           (list
            (auth-source-xoauth2-pass--find-match
             "host" "user" 993)
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (((secret . "legacy-entry")) (("host" "user")))"#]];
    assert_auth_source_xoauth2_parity(elisp_form, expect);
}

#[test]
fn auth_source_xoauth2_pass_get_supports_entry_name_and_parsed_alist() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'auth-source-pass-get)
               (lambda (key entry)
                 (push (list key entry) calls)
                 (concat entry ":" key))))
           (list
            (auth-source-xoauth2--pass-get
             "xoauth2_client_id"
             "accounts/mail/alice")
            (auth-source-xoauth2--pass-get
             "xoauth2_client_secret"
             '(("xoauth2_client_secret" . "parsed-secret")
               ("other" . "value")))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("accounts/mail/alice:xoauth2_client_id" "parsed-secret" (("xoauth2_client_id" "accounts/mail/alice")))"#
    ]];
    assert_auth_source_xoauth2_parity(elisp_form, expect);
}

#[test]
fn auth_source_xoauth2_pass_get_reports_missing_and_unsupported_entries() {
    let elisp_form = r##"(let (messages)
         (cl-letf
             (((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (let ((text
                        (apply
                         #'format
                         format-string
                         arguments)))
                   (push text messages)
                   text))))
           (list
            (auth-source-xoauth2--pass-get
             "missing"
             '(("other" . "value")))
            (auth-source-xoauth2--pass-get
             "missing"
             42)
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (nil nil ("Missing XOAuth2 entry value for 'missing'" "Missing XOAuth2 entry value for 'missing'"))"#
    ]];
    assert_auth_source_xoauth2_parity(elisp_form, expect);
}

#[test]
fn auth_source_xoauth2_pass_creds_builds_complete_provider_plist() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'auth-source-xoauth2-pass--find-match)
               (lambda (&rest arguments)
                 (push (cons :find arguments) calls)
                 '(("xoauth2_token_url" . "https://token.example")
                   ("xoauth2_client_id" . "client")
                   ("xoauth2_client_secret" . "secret")
                   ("xoauth2_refresh_token" . "refresh"))))
              ((symbol-function 'auth-source-xoauth2--pass-get)
               (lambda (key entry)
                 (push (list :get key entry) calls)
                 (cdr
                  (assoc key entry)))))
           (list
            (auth-source-xoauth2-pass-creds
             "host" "user" 443)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((:token-url "https://token.example" :client-id "client" :client-secret "secret" :refresh-token "refresh") ((:find "host" "user" 443) (:get "xoauth2_token_url" #1=(("xoauth2_token_url" . "https://token.example") ("xoauth2_client_id" . "client") ("xoauth2_client_secret" . "secret") ("xoauth2_refresh_token" . "refresh"))) (:get "xoauth2_client_id" #1#) (:get "xoauth2_client_secret" #1#) (:get "xoauth2_refresh_token" #1#)))"#
    ]];
    assert_auth_source_xoauth2_parity(elisp_form, expect);
}

#[test]
fn auth_source_xoauth2_pass_creds_stops_at_first_missing_value() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'auth-source-xoauth2-pass--find-match)
               (lambda (&rest _arguments)
                 :entry))
              ((symbol-function 'auth-source-xoauth2--pass-get)
               (lambda (key _entry)
                 (push key calls)
                 (cdr
                  (assoc
                   key
                   '(("xoauth2_token_url" . "url")
                     ("xoauth2_client_id" . "id")
                     ("xoauth2_client_secret")
                     ("xoauth2_refresh_token" . "refresh")))))))
           (list
            (auth-source-xoauth2-pass-creds
             "host" "user" 443)
            (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (nil ("xoauth2_token_url" "xoauth2_client_id" "xoauth2_client_secret"))"#]];
    assert_auth_source_xoauth2_parity(elisp_form, expect);
}

#[test]
fn auth_source_xoauth2_pass_creds_returns_nil_without_matching_entry() {
    let elisp_form = r##"(let (gets)
         (cl-letf
             (((symbol-function 'auth-source-xoauth2-pass--find-match)
               (lambda (&rest _arguments)
                 nil))
              ((symbol-function 'auth-source-xoauth2--pass-get)
               (lambda (&rest arguments)
                 (push arguments gets)
                 :unexpected)))
           (list
            (auth-source-xoauth2-pass-creds
             "host" "user" 443)
            gets)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_auth_source_xoauth2_parity(elisp_form, expect);
}
