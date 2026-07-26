use expect_test::expect;

use super::assert_magit_parity;

#[test]
fn magit_prompt_matching_selects_patterns_suffixes_and_match_groups() {
    let elisp_form = r##"(let* ((prompts
                     '("^bar: ?$"
                       "^foo '\\(?99:.*\\)': ?$"
                       "^foo: ?$"))
                    (matched
                     (magit-process-match-prompt
                      prompts "foo 'payload':"))
                    (payload
                     (match-string-no-properties
                      99 "foo 'payload':")))
               (list
                (magit-process-match-prompt '("^foo: ?$") "bar: ")
                (magit-process-match-prompt '("^foo: ?$") "foo:")
                (magit-process-match-prompt '("^foo: ?$") "foo: ")
                matched
                payload))"##;
    let expect = expect![[r#"OK (nil "foo: " "foo: " "foo 'payload': " "payload")"#]];

    assert_magit_parity(elisp_form, expect);
}

#[test]
fn magit_password_prompt_patterns_extract_hosts_without_protocol_noise() {
    let elisp_form = r##"(mapcar
              (lambda (prompt)
                (and
                 (magit-process-match-prompt
                  magit-process-password-prompt-regexps prompt)
                 (or
                  (match-string-no-properties 99 prompt)
                  t)))
              '("Passphrase: "
                "Enter passphrase for key '/home/me/.ssh/id_rsa': "
                "Password for 'https://example.com': "
                "Password for 'https://me@magit.vc':"
                "Password for ahihi@foo:"
                "(user@host) Password for user@host: "
                "volumio@192.168.0.211's password: "
                "Token: "
                "not a credential prompt"))"##;
    let expect = expect![[
        r#"OK (t t "example.com" "me@magit.vc" "ahihi@foo" "user@host" "volumio@192.168.0.211" t nil)"#
    ]];

    assert_magit_parity(elisp_form, expect);
}
