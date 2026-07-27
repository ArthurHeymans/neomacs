use expect_test::expect;

use super::assert_age_parity;

#[test]
fn age_exact_pin_package_metadata_and_custom_registration_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr (assq 'age package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-reqs descriptor)
          (package-desc-summary descriptor)
          (copy-tree
           (package-desc-extras descriptor))
          (mapcar
           (lambda (symbol)
             (list
             symbol
              (and (custom-variable-p symbol) t)
              (get symbol 'custom-type)))
           '(age-default-recipient
             age-default-identity
             age-always-use-default-keys
             age-program
             age-passphrase-coding-system
             age-pinentry-mode
             age-debug
             age-file-name-regexp
             age-file-inhibit-auto-save
             age-file-cache-passphrase-for-symmetric-encryption
             age-file-select-keys))
          (mapcar
           (lambda (group)
             (list
              group
              (get group 'group-documentation)
              (mapcar #'car
                      (get group 'custom-group))))
           '(age age-file))))"##;
    let expect = expect![[
        r#"OK (age "20250806.1723" ((emacs (28 1))) "The Age Encryption Library." ((:maintainers ("Bas Alberts" . "bas@anti.computer")) (:authors ("Daiki Ueno" . "ueno@unixuser.org") ("Bas Alberts" . "bas@anti.computer")) (:keywords "data") (:revdesc . "e99165ef5274") (:commit . "e99165ef5274bc4512b8d77ba2ac208c59b5d456") (:url . "https://github.com/anticomputer/age.el")) ((age-default-recipient t (choice (file :tag "File path to default recipient (public key path)") (repeat :tag "List of default recipients (public key paths or values)" (choice (file :tag "File path to default recipient (public key path)") (string :tag "Default recipient (public key value"))) (string :tag "Default recipient (public key value)"))) (age-default-identity t (choice (file :tag "File path to default identity (private key path)") (repeat :tag "List of default identities (private key paths)" (file :tag "File path to default identity (private key path)")))) (age-always-use-default-keys t boolean) (age-program t string) (age-passphrase-coding-system t symbol) (age-pinentry-mode t (choice (const nil) (const ask) (const cancel) (const error))) (age-debug t boolean) (age-file-name-regexp t regexp) (age-file-inhibit-auto-save t boolean) (age-file-cache-passphrase-for-symmetric-encryption t boolean) (age-file-select-keys t (choice (const :tag "Ask always" t) (const :tag "Ask when recipients are not set" nil) (const :tag "Don't ask" silent)))) ((age "Interface to Age." (age-default-recipient age-default-identity age-always-use-default-keys age-program age-passphrase-coding-system age-pinentry-mode age-debug)) (age-file nil (age-file-name-regexp age-file-inhibit-auto-save age-encryption-mode age-file-cache-passphrase-for-symmetric-encryption age-file-select-keys))))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_registry_exposes_exact_metadata_defaults_and_protocol_tables() {
    let elisp_form = r##"(list
         age-package-name
         age-version-number
         age-minimum-version
         age-file-name-regexp
         age-always-use-default-keys
         age-passphrase-coding-system
         age-pinentry-mode
         age-debug
         age-file-inhibit-auto-save
         age-file-cache-passphrase-for-symmetric-encryption
         age-file-select-keys
         age-armor
         age-invalid-recipients-reason-alist
         age-no-data-reason-alist
         age-unexpected-reason-alist
         age-config--program-alist
         age-config--configuration-constructor-alist
         (get 'age-file-handler 'safe-magic)
         (get 'age-file-handler 'operations)
         (get 'age-file-encrypt-to 'permanent-local)
         (featurep 'age))"##;
    let expect = expect![[
        r#"OK ("age" "0.1.9" "1.0.0" "\\.age\\'" t nil nil nil t nil nil t ((0 . "unknown recipient type")) ((1 . "did you mean to use -a/--armor")) nil ((Age age-program ("rage" . "0.9.0") ("age" . "1.0.0"))) ((Age . age-config--make-age-configuration)) t (write-region insert-file-contents) t t)"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_registry_exposes_complete_callable_surface_and_generated_accessors() {
    let elisp_form = r##"(let ((functions
                '(age-find-configuration
                  age-config--make-age-configuration
                  age-configuration
                  age-check-configuration
                  age-required-version-p
                  age-make-data-from-file
                  age-make-data-from-string
                  age-data-file
                  age-data-string
                  age-context--make
                  age-context-protocol
                  age-context-program
                  age-context-armor
                  age-context-passphrase
                  age-context-passphrase-callback
                  age-context-edit-callback
                  age-context-process
                  age-context-output-file
                  age-context-result
                  age-context-operation
                  age-context-pinentry-mode
                  age-context-error-output
                  age-context-error-buffer
                  age-with-dev-shm
                  age-make-context
                  age-context-set-passphrase-callback
                  age-context-result-for
                  age-context-set-result-for
                  age-error-to-string
                  age-errors-to-string
                  age--start
                  age--process-stdout-filter
                  age--process-stderr-filter
                  age-read-output
                  age-wait-for-completion
                  age-reset
                  age-delete-output-file
                  age--status-GET_PASSPHRASE
                  age--status-AGE_FAILED
                  age-cancel
                  age-start-decrypt
                  age--check-error-for-decrypt
                  age-decrypt-file
                  age-decrypt-string
                  age-start-encrypt
                  age-encrypt-file
                  age-encrypt-string
                  age--decode-percent-escape
                  age--decode-percent-escape-as-utf-8
                  age--decode-hexstring
                  age--decode-quotedstring
                  age-file-find-file-hook
                  age-encryption-mode
                  age-passphrase-callback-function
                  age-file-passphrase-callback-function
                  age-display-error
                  age-scrypt-p
                  age-inhibit-advice
                  age-advise-tramp
                  age-file-handler
                  age-file-run-real-handler
                  age-file-decode-and-insert
                  age-file--find-file-not-found-function
                  age--wrong-password-p
                  age-file-insert-file-contents
                  age-file--replace-text
                  age-select-keys
                  age-file-write-region
                  age-file-select-keys
                  age-file-enable
                  age-file-disable)))
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (commandp symbol)
                  (and (fboundp symbol)
                       (help-function-arglist symbol t))))
          functions))"##;
    let expect = expect![
        "OK ((age-find-configuration t nil (protocol &optional no-cache program-alist)) (age-config--make-age-configuration t nil (program)) (age-configuration t nil nil) (age-check-configuration t nil (config &optional req-versions)) (age-required-version-p t nil (protocol required-version)) (age-make-data-from-file t nil (file)) (age-make-data-from-string t nil (string)) (age-data-file t nil #1=(x)) (age-data-string t nil #1#) (age-context--make t nil (protocol &optional armor &rest --cl-rest--)) (age-context-protocol t nil #1#) (age-context-program t nil #1#) (age-context-armor t nil #1#) (age-context-passphrase t nil #1#) (age-context-passphrase-callback t nil #1#) (age-context-edit-callback t nil #1#) (age-context-process t nil #1#) (age-context-output-file t nil #1#) (age-context-result t nil #1#) (age-context-operation t nil #1#) (age-context-pinentry-mode t nil #1#) (age-context-error-output t nil #1#) (age-context-error-buffer t nil #1#) (age-with-dev-shm t nil (&rest body)) (age-make-context t nil (&optional protocol armor)) (age-context-set-passphrase-callback t nil (context passphrase-callback)) (age-context-result-for t nil (context name)) (age-context-set-result-for t nil (context name value)) (age-error-to-string t nil (error)) (age-errors-to-string t nil (errors)) (age--start t nil (context args)) (age--process-stdout-filter t nil (_process input)) (age--process-stderr-filter t nil (process input)) (age-read-output t nil (context)) (age-wait-for-completion t nil (context)) (age-reset t nil (context)) (age-delete-output-file t nil (context)) (age--status-GET_PASSPHRASE t nil (context string)) (age--status-AGE_FAILED t nil (context _string)) (age-cancel t nil (context)) (age-start-decrypt t nil (context cipher)) (age--check-error-for-decrypt t nil (context)) (age-decrypt-file t nil (context cipher plain)) (age-decrypt-string t nil (context cipher)) (age-start-encrypt t nil (context plain recipients)) (age-encrypt-file t nil (context plain recipients cipher)) (age-encrypt-string t nil (context plain recipients)) (age--decode-percent-escape t nil (string)) (age--decode-percent-escape-as-utf-8 t nil (string)) (age--decode-hexstring t nil (string)) (age--decode-quotedstring t nil (string)) (age-file-find-file-hook t nil nil) (age-encryption-mode t t (&optional arg)) (age-passphrase-callback-function t nil (context handback)) (age-file-passphrase-callback-function t nil (context _key-id file)) (age-display-error t nil (context)) (age-scrypt-p t nil (file)) (age-inhibit-advice t nil (orig-func &rest args)) (age-advise-tramp t nil (&optional remove)) (age-file-handler t nil (operation &rest args)) (age-file-run-real-handler t nil (operation args)) (age-file-decode-and-insert t nil (string file visit beg end replace)) (age-file--find-file-not-found-function t nil nil) (age--wrong-password-p t nil (context)) (age-file-insert-file-contents t nil (file &optional visit beg end replace)) (age-file--replace-text t nil (string file visit beg end)) (age-select-keys t nil (_context _msg &optional recipients)) (age-file-write-region t nil (start end file &optional append visit lockname mustbenew)) (age-file-select-keys t t nil) (age-file-enable t t nil) (age-file-disable t t nil))"
    ];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_safe_local_recipient_policy_accepts_only_strings_and_string_lists() {
    let elisp_form = r##"(let ((predicate
                (get 'age-file-encrypt-to 'safe-local-variable)))
         (mapcar
          (lambda (value)
            (list value (funcall predicate value)))
          '(nil
            ""
            "age1recipient"
            ("one")
            ("one" "two")
            ("one" 2)
            (symbol)
            17
            [vector])))"##;
    let expect = expect![[
        r#"OK ((nil t) ("" t) ("age1recipient" t) (("one") t) (("one" "two") t) (("one" 2) nil) ((symbol) nil) (17 nil) ([vector] nil))"#
    ]];
    assert_age_parity(elisp_form, expect);
}
