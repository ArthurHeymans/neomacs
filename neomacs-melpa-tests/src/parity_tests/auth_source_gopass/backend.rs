use expect_test::expect;

use super::assert_auth_source_gopass_parity;

#[test]
fn auth_source_gopass_backend_has_exact_source_type_and_search_function() {
    let elisp_form = r##"(list
         (slot-value
          auth-source-gopass-backend
          'source)
         (slot-value
          auth-source-gopass-backend
          'type)
         (slot-value
          auth-source-gopass-backend
          'search-function)
         (functionp
          (slot-value
           auth-source-gopass-backend
           'search-function)))"##;
    let expect = expect![[r#"OK ("." gopass auth-source-gopass-search t)"#]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_backend_parse_forwards_exact_gopass_entry() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'auth-source-backend-parse-parameters)
               (lambda (&rest arguments)
                 (push arguments calls)
                 (list :parsed arguments))))
           (list
            (auth-source-gopass-backend-parse 'gopass)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((:parsed #1=(gopass #s(auth-source-backend gopass "." t t t nil ignore auth-source-gopass-search))) (#1#))"#
    ]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_backend_parse_rejects_other_entry_shapes_without_delegating() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'auth-source-backend-parse-parameters)
               (lambda (&rest arguments)
                 (push arguments calls)
                 :unexpected)))
           (list
            (mapcar
             #'auth-source-gopass-backend-parse
             '(nil "gopass" (:source gopass) pass default gopass-other))
            calls)))"##;
    let expect = expect!["OK ((nil nil nil nil nil nil) nil)"];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_registered_parser_builds_the_package_backend() {
    let elisp_form = r##"(let ((backend
                (auth-source-backend-parse
                 'gopass)))
         (list
          (object-of-class-p
           backend
           'auth-source-backend)
          (slot-value backend 'source)
          (slot-value backend 'type)
          (slot-value backend 'search-function)))"##;
    let expect = expect![[r#"OK (t "." gopass auth-source-gopass-search)"#]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}

#[test]
fn auth_source_gopass_backend_search_function_is_directly_usable() {
    let elisp_form = r##"(let ((search
                (slot-value
                 auth-source-gopass-backend
                 'search-function))
               calls)
         (cl-letf
             (((symbol-function 'executable-find)
               (lambda (_program)
                 "/fixture/bin/gopass"))
              ((symbol-function 'shell-command-to-string)
               (lambda (command)
                 (push command calls)
                 "backend-secret\n")))
           (list
            (funcall
             search
             :backend auth-source-gopass-backend
             :type 'gopass
             :host "imap.example"
             :user "alice"
             :port 993)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (((:user "alice" :secret "backend-secret")) ("gopass show -o accounts/imap.example/alice"))"#
    ]];
    assert_auth_source_gopass_parity(elisp_form, expect);
}
