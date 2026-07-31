use expect_test::expect;

use super::assert_auth_source_xoauth2_batch;

#[test]
fn credentials_public_surface_batch() {
    assert_auth_source_xoauth2_batch(&[
        (
            "auth_source_xoauth2_file_creds_reads_plist_from_gpg_named_file",
            r##"(let ((file-name-handler-alist nil)
               (file
                (auth-source-xoauth2-test-file
                 "single-account.gpg")))
         (with-temp-file file
           (insert
            "(:token-url \"https://token.example\" "
            ":client-id \"client\" "
            ":client-secret \"secret\" "
            ":refresh-token \"refresh\")"))
         (auth-source-xoauth2--file-creds
          file
          "ignored.example"
          "ignored-user"
          443))"##,
            true,
            expect![[r#"OK "Symbol’s function definition is void: :token-url""#]],
        ),
        (
            "auth_source_xoauth2_file_creds_uses_exact_hash_table_tuple",
            r##"(let ((file-name-handler-alist nil)
               (file
                (auth-source-xoauth2-test-file
                 "accounts.gpg")))
         (with-temp-file file
           (prin1
            (let ((table
                   (make-hash-table
                    :test #'equal)))
              (puthash
               '("imap.one" "alice" 993)
               '(:token-url "one"
                 :client-id "id-one"
                 :client-secret "secret-one"
                 :refresh-token "refresh-one")
               table)
              (puthash
               '("smtp.two" "bob" "submission")
               '(:token-url "two"
                 :client-id "id-two"
                 :client-secret "secret-two"
                 :refresh-token "refresh-two")
               table)
              table)
            (current-buffer)))
         (list
          (auth-source-xoauth2--file-creds
           file "imap.one" "alice" 993)
          (auth-source-xoauth2--file-creds
           file "smtp.two" "bob" "submission")
          (auth-source-xoauth2--file-creds
           file "smtp.two" "bob" 587)
          (auth-source-xoauth2--file-creds
           file "missing" "alice" 993)))"##,
            true,
            expect![[
        r#"OK ((:token-url "one" :client-id "id-one" :client-secret "secret-one" :refresh-token "refresh-one") (:token-url "two" :client-id "id-two" :client-secret "secret-two" :refresh-token "refresh-two") nil nil)"#
    ]],
        ),
        (
            "auth_source_xoauth2_file_creds_requires_gpg_extension",
            r##"(mapcar
         (lambda (name)
           (auth-source-xoauth2-test-error-data
            (lambda ()
              (auth-source-xoauth2--file-creds
               name "host" "user" "port"))))
         '("/fixture/creds.el"
           "/fixture/creds.gpg~"
           "/fixture/no-extension"
           "/fixture/GPG"))"##,
            true,
            expect![[
        r#"OK ((:error error ("The auth-source-xoauth2-creds file must be GPG encrypted")) (:ok "GPG error: \"no usable configuration\", OpenPGP") (:error error ("The auth-source-xoauth2-creds file must be GPG encrypted")) (:error error ("The auth-source-xoauth2-creds file must be GPG encrypted")))"#
    ]],
        ),
        (
            "auth_source_xoauth2_file_creds_reports_read_and_eval_failures",
            r##"(let ((file-name-handler-alist nil)
               (invalid
                (auth-source-xoauth2-test-file
                 "invalid.gpg"))
               (runtime-error
                (auth-source-xoauth2-test-file
                 "runtime-error.gpg"))
               (missing
                (auth-source-xoauth2-test-file
                 "missing.gpg")))
         (with-temp-file invalid
           (insert "(:token-url \"unterminated)"))
         (with-temp-file runtime-error
           (insert "(error \"credential exploded\")"))
         (mapcar
          (lambda (file)
            (auth-source-xoauth2-test-error-data
             (lambda ()
               (auth-source-xoauth2--file-creds
                file "host" "user" "port"))))
          (list invalid runtime-error missing)))"##,
            true,
            expect![[
        r#"OK ((:ok "End of file during parsing: #<killed buffer>") (:ok "credential exploded") (:ok "Opening input file: No such file or directory, [ORACLE-SANDBOX]/missing.gpg"))"#
    ]],
        ),
        (
            "auth_source_xoauth2_file_creds_evaluates_form_with_lexical_binding",
            r##"(let ((file-name-handler-alist nil)
               (file
                (auth-source-xoauth2-test-file
                 "computed.gpg")))
         (with-temp-file file
           (insert
            "(let ((prefix \"computed\")) "
            "  (list :token-url (concat prefix \"-url\") "
            "        :client-id (concat prefix \"-id\") "
            "        :client-secret \"secret\" "
            "        :refresh-token \"refresh\"))"))
         (auth-source-xoauth2--file-creds
          file "host" "user" "port"))"##,
            true,
            expect![[
        r#"OK (:token-url "computed-url" :client-id "computed-id" :client-secret "secret" :refresh-token "refresh")"#
    ]],
        ),
        (
            "auth_source_xoauth2_file_hash_lookup_emits_exact_debug_coordinates",
            r##"(let ((file-name-handler-alist nil)
               (file
                (auth-source-xoauth2-test-file
                 "debug.gpg"))
               calls)
         (with-temp-file file
           (prin1
            (let ((table
                   (make-hash-table
                    :test #'equal)))
              (puthash
               '("host" "user" 443)
               '(:token-url "url")
               table)
              table)
            (current-buffer)))
         (cl-letf
             (((symbol-function 'auth-source-do-debug)
               (lambda (format-string &rest arguments)
                 (push
                  (list format-string arguments)
                  calls))))
           (list
            (auth-source-xoauth2--file-creds
             file "host" "user" 443)
            (nreverse calls))))"##,
            true,
            expect![[
        r#"OK ((:token-url "url") (("Searching hash table for (%S %S %S)" ("host" "user" 443))))"#
    ]],
        ),
    ]);
}
