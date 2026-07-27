use expect_test::expect;

use super::assert_age_parity;

#[test]
fn age_scrypt_detection_reads_plain_armored_and_edge_case_headers() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               cases)
         (dolist
             (case
              `(("plain.age"
                 . "age-encryption.org/v1\n-> scrypt salt label\nbody\n")
                ("armor.age"
                 .
                 ,(concat
                   "-----BEGIN AGE ENCRYPTED FILE-----\n"
                   (base64-encode-string
                    "-> scrypt salt label"
                    t)
                   "\nbody\n"))
                ("recipient.age"
                 . "age-encryption.org/v1\n-> X25519 key\n")
                ("one-line.age"
                 . "age-encryption.org/v1")
                ("empty.age" . "")))
           (let ((path (expand-file-name (car case) root)))
             (write-region (cdr case) nil path nil 'quiet)
             (push
              (list (car case)
                    (age-scrypt-p path))
              cases)))
         (list
          (nreverse cases)
          (age-scrypt-p
           (expand-file-name "missing.age" root))))"##;
    let expect = expect![[
        r#"OK ((("plain.age" t) ("armor.age" t) ("recipient.age" nil) ("one-line.age" nil) ("empty.age" nil)) nil)"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_passphrase_callbacks_format_prompts_and_cache_defensive_copies_per_file() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (age-file-passphrase-alist nil)
               prompts
               counter)
         (cl-letf (((symbol-function 'read-passwd)
                    (lambda (prompt confirm)
                      (push (list prompt confirm) prompts)
                      (setq counter (1+ (or counter 0)))
                      (format "secret-%d" counter))))
           (let ((encrypt-context
                  (cl-letf
                      (((symbol-function 'age-find-configuration)
                        (lambda (_protocol)
                          '((program . "age")))))
                    (age-make-context)))
                 (decrypt-context
                  (cl-letf
                      (((symbol-function 'age-find-configuration)
                        (lambda (_protocol)
                          '((program . "age")))))
                    (age-make-context)))
                 (file
                  (expand-file-name "vault.age" root)))
             (setf (age-context-operation encrypt-context)
                   'encrypt
                   (age-context-operation decrypt-context)
                   'decrypt)
             (write-region "" nil file nil 'quiet)
             (let ((direct-encrypt
                    (age-passphrase-callback-function
                     encrypt-context file))
                   (direct-decrypt
                    (age-passphrase-callback-function
                     decrypt-context nil))
                   (age-file-cache-passphrase-for-symmetric-encryption
                    t))
               (let ((first
                      (age-file-passphrase-callback-function
                       decrypt-context nil file)))
                 (aset first 0 ?X)
                 (let ((second
                        (age-file-passphrase-callback-function
                         decrypt-context nil file)))
                   (list
                    direct-encrypt
                    direct-decrypt
                    first
                    second
                    age-file-passphrase-alist
                    (nreverse prompts))))))))"##;
    let expect = expect![[
        r#"OK ("secret-1" "secret-2" "Xecret-3" "secret-3" (("[ORACLE-SANDBOX]/vault.age" . "secret-3")) (("Passphrase for [ORACLE-SANDBOX]/vault.age: " t) ("Passphrase: " nil) ("Passphrase for [ORACLE-SANDBOX]/vault.age: " nil)))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_select_keys_merges_explicit_existing_default_and_prompted_recipients() {
    let elisp_form = r##"(let (events)
         (list
          (with-temp-buffer
            (let ((age-file-encrypt-to
                   '("age1existing" "age1second")))
              (age-select-keys nil "ignored"
                               '("age1prefix"))))
          (with-temp-buffer
            (let ((age-file-encrypt-to nil)
                  (age-default-recipient
                   "age1default")
                  (age-always-use-default-keys t))
              (age-select-keys nil "ignored"
                               '("age1prefix"))))
          (with-temp-buffer
            (let ((age-file-encrypt-to nil)
                  (age-default-recipient
                   "age1default")
                  (age-always-use-default-keys nil))
              (cl-letf
                  (((symbol-function 'y-or-n-p)
                    (lambda (prompt)
                      (push (list 'confirm prompt) events)
                      nil))
                   ((symbol-function 'read-file-name)
                    (lambda (prompt directory)
                      (push (list 'file prompt directory)
                            events)
                      "~/keys/recipient.pub")))
                (list
                 (age-select-keys nil "ignored" nil)
                 (local-variable-p
                  'age-file-encrypt-to
                  (current-buffer))))))
          (nreverse events)))"##;
    let expect = expect![[
        r#"OK (("age1prefix" "age1existing" "age1second") ("age1prefix" "age1default") (("[ORACLE-HOME]/keys/recipient.pub") t) ((confirm "Use default recipient(s)? ") (file "Path to recipient(s): " "[ORACLE-HOME]/")))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_file_handler_routes_registered_operations_and_respects_inhibition() {
    let elisp_form = r##"(let (events)
         (put 'age-test-operation
              'age-file
              'age-test-operation-handler)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'age-test-operation-handler)
                   (lambda (&rest arguments)
                     (push (cons 'handled arguments) events)
                     'handled-result))
                  ((symbol-function 'age-file-run-real-handler)
                   (lambda (operation arguments)
                     (push (list 'real operation arguments)
                           events)
                     'real-result)))
               (list
                (let ((age-inhibit nil))
                  (age-file-handler
                   'age-test-operation "a" 2))
                (let ((age-inhibit t))
                  (age-file-handler
                   'age-test-operation "b" 3))
                (age-file-handler
                 'unregistered-operation "c")
                (nreverse events)))
           (put 'age-test-operation 'age-file nil)
           (when (fboundp 'age-test-operation-handler)
             (fmakunbound 'age-test-operation-handler))))"##;
    let expect = expect![[
        r#"OK (handled-result real-result real-result ((handled "a" 2) (real age-test-operation ("b" 3)) (real unregistered-operation ("c"))))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_real_handler_scopes_recursive_handler_inhibition_for_nested_operation() {
    let elisp_form = r##"(let ((file-name-handler-alist
                (list age-file-handler
                      '("other" . other-handler)))
               observations)
         (cl-letf (((symbol-function 'age-test-real-operation)
                    (lambda (&rest arguments)
                      (setq observations
                            (list
                             arguments
                             inhibit-file-name-handlers
                             inhibit-file-name-operation))
                      'real-value)))
           (let ((inhibit-file-name-handlers
                  '(existing-handler))
                 (inhibit-file-name-operation
                  'age-test-real-operation))
             (list
              (age-file-run-real-handler
               'age-test-real-operation
               '(one two))
              observations
              inhibit-file-name-handlers
              inhibit-file-name-operation))))"##;
    let expect = expect![
        "OK (real-value ((one two) (age-file-handler . #1=(existing-handler)) age-test-real-operation) #1# age-test-real-operation)"
    ];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_decode_insert_and_replace_preserve_point_and_minimize_buffer_edits() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "prefix|suffix")
           (goto-char 8)
           (let ((length
                  (age-file-decode-and-insert
                   "line one\nline two"
                   "notes.txt.age"
                   nil nil nil nil)))
             (list length
                   (point)
                   (buffer-string))))
         (with-temp-buffer
           (insert "alpha beta gamma")
           (goto-char 7)
           (let ((length
                  (age-file--replace-text
                   "alpha BETA gamma"
                   "notes.txt.age"
                   nil nil nil)))
             (list length
                   (point)
                   (buffer-string))))
         (with-temp-buffer
           (insert "same text")
           (goto-char 4)
           (let ((length
                  (age-file--replace-text
                   "same text"
                   "notes.txt.age"
                   nil nil nil)))
             (list length
                   (point)
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ((17 25 "prefix|line one\nline twosuffix") (16 7 "alpha BETA gamma") (9 4 "same text"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_insert_file_contents_runs_real_buffer_workflow_with_ranges_and_recipient_state() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                (file (expand-file-name "document.txt.age" root))
                calls)
         (write-region
          "age-encryption.org/v1\n-> X25519 key\ncipher"
          nil file nil 'quiet)
         (cl-letf (((symbol-function 'age-make-context)
                    (lambda (&rest _arguments)
                      (cl-letf
                          (((symbol-function
                             'age-find-configuration)
                            (lambda (_protocol)
                              '((program . "age")))))
                        (age-context--make 'Age))))
                   ((symbol-function 'age-decrypt-file)
                    (lambda (context input output)
                      (push (list input output
                                  (age-context-passphrase context)
                                  (cdr
                                   (age-context-passphrase-callback
                                    context)))
                            calls)
                      (age-context-set-result-for
                       context
                       'encrypted-to
                       '(("age1alice" . key)
                         ("age1bob" . key)))
                      "0123456789")))
           (with-temp-buffer
             (insert "before")
             (goto-char (point-max))
             (let ((result
                    (age-file-insert-file-contents
                     file nil 2 8 nil)))
               (list
                result
                (buffer-string)
                age-file-encrypt-to
                (local-variable-p
                 'age-file-encrypt-to
                 (current-buffer))
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-SANDBOX]/document.txt.age" 6) "before234567" ("age1alice" "age1bob") t (("[ORACLE-SANDBOX]/document.txt.age" nil nil "[ORACLE-SANDBOX]/document.txt.age")))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_insert_file_contents_validates_visit_ranges_and_read_only_buffers() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
         (mapcar
          (lambda (case)
            (with-temp-buffer
              (setq buffer-read-only (eq case 'read-only))
              (condition-case error-data
                  (age-file-insert-file-contents
                   (expand-file-name "missing.age" root)
                   (eq case 'partial-visit)
                   (and (eq case 'partial-visit) 0)
                   nil nil)
                (error
                 (list
                  case
                  (car error-data)
                  (cadr error-data))))))
          '(partial-visit read-only)))"##;
    let expect = expect![[
        r#"OK ((partial-visit error "Attempt to visit less than an entire file") (read-only buffer-read-only (:buffer nil)))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_write_region_encrypts_real_string_and_buffer_regions_with_coding_and_visiting() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               calls messages)
         (cl-letf (((symbol-function 'age-make-context)
                    (lambda (&rest _arguments)
                      (cl-letf
                          (((symbol-function
                             'age-find-configuration)
                            (lambda (_protocol)
                              '((program . "age")))))
                        (age-context--make 'Age))))
                   ((symbol-function 'age-scrypt-p)
                    (lambda (file)
                      (string-match-p "symmetric" file)))
                   ((symbol-function 'age-encrypt-string)
                    (lambda (context plain recipients)
                      (push
                       (list
                        (string-to-list plain)
                        recipients
                        (age-context-passphrase context)
                        (age-context-armor context)
                        (cdr
                         (age-context-passphrase-callback
                          context)))
                       calls)
                      (concat "cipher:" plain)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push
                       (apply #'format
                              format-string
                              arguments)
                       messages))))
           (with-temp-buffer
             (insert "prefix α suffix")
             (let ((age-file-encrypt-to
                    '("age1alice" "age1bob"))
                   (age-file-select-keys 'silent)
                   (age-armor nil)
                   (coding-system-for-write 'utf-8))
               (age-file-write-region
                8 9
                (expand-file-name "region.age" root)
                nil nil nil nil)))
           (with-temp-buffer
             (let ((age-file-encrypt-to
                    "age1single")
                   (age-file-select-keys 'silent)
                   (age-armor t)
                   (coding-system-for-write 'utf-8))
               (age-file-write-region
                "direct λ"
                nil
                (expand-file-name "symmetric.age" root)
                nil nil nil nil)))
           (list
            (nreverse calls)
            (with-temp-buffer
              (insert-file-contents-literally
               (expand-file-name "region.age" root))
              (buffer-string))
            (with-temp-buffer
              (insert-file-contents-literally
               (expand-file-name "symmetric.age" root))
              (buffer-string))
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ((((206 177) nil nil nil "[ORACLE-SANDBOX]/region.age") ((100 105 114 101 99 116 32 206 187) nil 207 t "[ORACLE-SANDBOX]/symmetric.age")) "cipher:\316\261" "cipher:direct \316\273" ("Wrote nil" "Wrote nil"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_write_region_rejects_append_before_crypto_or_filesystem_side_effects() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               encrypted)
         (cl-letf (((symbol-function 'age-encrypt-string)
                    (lambda (&rest _arguments)
                      (setq encrypted t)
                      "cipher")))
           (let ((path
                  (expand-file-name "append.age" root)))
             (list
              (condition-case error-data
                  (age-file-write-region
                   "text" nil path t nil nil nil)
                (error
                 (list (car error-data)
                       (cadr error-data))))
              encrypted
              (file-exists-p path)))))"##;
    let expect = expect![[r#"OK ((error "Can’t append to the file") nil nil)"#]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_file_enable_disable_updates_handlers_hooks_modes_and_tramp_lifecycle_idempotently() {
    let elisp_form = r##"(let ((file-name-handler-alist nil)
               (find-file-hook nil)
               (auto-mode-alist nil)
               advice-events
               messages)
         (cl-letf (((symbol-function 'age-advise-tramp)
                    (lambda (&optional remove)
                      (push remove advice-events)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages))))
           (age-file-enable)
           (let ((enabled
                  (list file-name-handler-alist
                        find-file-hook
                        auto-mode-alist)))
             (age-file-enable)
             (age-file-disable)
             (let ((disabled
                    (list file-name-handler-alist
                          find-file-hook
                          auto-mode-alist)))
               (age-file-disable)
               (list
                enabled
                disabled
                (nreverse advice-events)
                (nreverse messages))))))"##;
    let expect = expect![[
        r#"OK (((("\\.age\\'" . age-file-handler)) (age-file-find-file-hook) (("\\.age\\'" nil age-file))) (nil nil nil) (nil nil t t) ("`age-file' enabled" "`age-file' already enabled" "`age-file' disabled" "`age-file' already disabled"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_tramp_advice_wraps_each_target_with_dynamic_inhibition_and_removes_cleanly() {
    let elisp_form = r##"(let ((age-tramp-inhibit-funcs
                '(age-test-tramp-one age-test-tramp-two))
               events)
         (fset 'age-test-tramp-one
               (lambda (value)
                 (push (list 'one value age-inhibit)
                       events)
                 (list 'one value age-inhibit)))
         (fset 'age-test-tramp-two
               (lambda (value)
                 (push (list 'two value age-inhibit)
                       events)
                 (list 'two value age-inhibit)))
         (unwind-protect
             (progn
               (age-advise-tramp)
               (let ((members-before
                      (mapcar
                       (lambda (function)
                         (and
                          (advice-member-p
                           #'age-inhibit-advice function)
                          t))
                       age-tramp-inhibit-funcs))
                     (calls
                      (list
                       (age-test-tramp-one "a")
                       (age-test-tramp-two "b"))))
                 (age-advise-tramp)
                 (age-advise-tramp t)
                 (list
                  members-before
                  calls
                  (nreverse events)
                  (mapcar
                   (lambda (function)
                     (and
                      (advice-member-p
                       #'age-inhibit-advice function)
                      t))
                   age-tramp-inhibit-funcs))))
           (advice-remove
            'age-test-tramp-one
            #'age-inhibit-advice)
           (advice-remove
            'age-test-tramp-two
            #'age-inhibit-advice)
           (fmakunbound 'age-test-tramp-one)
           (fmakunbound 'age-test-tramp-two)))"##;
    let expect =
        expect![[r#"OK ((t t) ((one "a" t) (two "b" t)) ((one "a" t) (two "b" t)) (nil nil))"#]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_find_file_hook_disables_auto_save_only_for_matching_visited_files() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'auto-save-mode)
                    (lambda (argument)
                      (push argument calls)
                      argument)))
           (dolist
               (case
                '(("/vault/secret.org.age" t)
                  ("/vault/plain.org" t)
                  ("/vault/secret.age" nil)
                  (nil t)))
             (let ((buffer-file-name (car case))
                   (age-file-inhibit-auto-save
                    (cadr case)))
               (age-file-find-file-hook)))
           (nreverse calls)))"##;
    let expect = expect!["OK (0)"];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_display_error_renders_operation_specific_buffer_and_honors_suppression() {
    let elisp_form = r##"(let ((context
                (cl-letf (((symbol-function
                            'age-find-configuration)
                           (lambda (_protocol)
                             '((program . "/usr/bin/age")))))
                  (age-make-context)))
               displayed)
         (setf (age-context-error-output context)
               "age: error: no identity\n")
         (cl-letf (((symbol-function 'display-buffer)
                    (lambda (buffer &rest _arguments)
                      (push
                       (with-current-buffer buffer
                         (list (buffer-name)
                               (point)
                               (buffer-string)))
                       displayed)
                      buffer)))
           (dolist (operation '(decrypt encrypt inspect))
             (setf (age-context-operation context) operation)
             (age-display-error context))
           (let ((age-suppress-error-buffer t))
             (age-display-error context))
           (prog1
               (nreverse displayed)
             (when (buffer-live-p
                    (get-buffer "*Error*"))
               (kill-buffer (get-buffer "*Error*")))
             (when (and age-error-buffer
                        (buffer-live-p age-error-buffer))
               (kill-buffer age-error-buffer)))))"##;
    let expect = expect![[
        r#"OK (("*Error*" 1 "Error while decrypting with \"/usr/bin/age\":\n\nage: error: no identity\n") ("*Error*" 1 "Error while encrypting with \"/usr/bin/age\":\n\nage: error: no identity\n") ("*Error*" 1 "Error while executing \"/usr/bin/age\":\n\n\n\nage: error: no identity\n"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_missing_file_callback_distinguishes_wrong_passphrase_from_missing_file() {
    let elisp_form = r##"(mapcar
         (lambda (error-value)
           (let ((buffer
                  (generate-new-buffer
                   " *age-missing-callback*")))
             (condition-case error-data
                 (with-current-buffer buffer
                   (setq-local age-file-error error-value)
                   (age-file--find-file-not-found-function))
               (error
                (list
                 (car error-data)
                 (cdr error-data)
                 (buffer-live-p buffer))))))
         '((file-error "Opening input file" "vault.age"
                       "incorrect passphrase")
           (file-missing "Opening input file" "vault.age")))"##;
    let expect = expect![[
        r#"OK ((user-error ("Wrong passphrase: incorrect passphrase") nil) (file-missing ("Opening input file" "Opening input file" "vault.age") nil))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_encryption_mode_and_select_keys_command_delegate_real_global_and_buffer_state() {
    let elisp_form = r##"(let (events)
         (cl-letf (((symbol-function 'age-file-enable)
                    (lambda ()
                      (push 'enable events)))
                   ((symbol-function 'age-file-disable)
                    (lambda ()
                      (push 'disable events)))
                   ((symbol-function 'age-make-context)
                    (lambda (&rest arguments)
                      (push (cons 'context arguments) events)
                      'context))
                   ((symbol-function 'age-select-keys)
                    (lambda (context message &optional recipients)
                      (push (list 'select context message recipients)
                            events)
                      '("age1selected"))))
           (unwind-protect
               (progn
                 (age-encryption-mode 1)
                 (age-encryption-mode 0)
                 (with-temp-buffer
                   (age-file-select-keys)
                   (list
                    age-file-encrypt-to
                    (local-variable-p
                     'age-file-encrypt-to
                     (current-buffer))
                    (nreverse events))))
             (setq age-encryption-mode nil))))"##;
    let expect = expect![[
        r#"OK (("age1selected") t (enable disable (context) (select context "Select recipients for encryption." nil)))"#
    ]];
    assert_age_parity(elisp_form, expect);
}
