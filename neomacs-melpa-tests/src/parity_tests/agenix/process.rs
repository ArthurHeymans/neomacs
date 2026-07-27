use expect_test::expect;

use super::assert_agenix_parity;

#[test]
fn agenix_buffer_helpers_read_other_buffer_without_properties_and_dispose_temp_buffer() {
    let elisp_form = r##"(let ((source
                (generate-new-buffer
                 " *agenix-source-parity*"))
               events)
         (unwind-protect
             (progn
               (with-current-buffer source
                 (insert
                  (propertize
                   "secret λ\n"
                   'face 'bold
                   'agenix-property t)))
               (let ((before (current-buffer))
                     (other
                      (agenix--buffer-string* source))
                     (temporary
                      (agenix--with-temp-buffer
                       (lambda (buffer)
                         (push
                          (list
                           (buffer-name buffer)
                           (eq buffer
                               (current-buffer)))
                          events)
                         (with-current-buffer buffer
                           (insert
                            (propertize
                             "temporary"
                             'face 'italic))
                           (list
                            (agenix--buffer-string*
                             buffer)
                            (text-properties-at
                             (point-min))))))))
                 (list
                  other
                  (text-properties-at 0 other)
                  temporary
                  (eq before (current-buffer))
                  (get-buffer "*age-buf*")
                  (nreverse events))))
           (when (buffer-live-p source)
             (kill-buffer source))))"##;
    let expect =
        expect![[r#"OK ("secret λ\n" nil ("temporary" (face italic)) t nil (("*age-buf*" nil)))"#]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_identity_protection_probe_invokes_ssh_keygen_and_classifies_exit_codes() {
    let elisp_form = r##"(progn
         (defvar agenix-parity-exit-code nil)
         (let (calls)
           (cl-letf (((symbol-function 'call-process)
                      (lambda (&rest arguments)
                        (push arguments calls)
                        agenix-parity-exit-code)))
             (list
              (let ((agenix-parity-exit-code 0))
                (agenix--identity-protected-p
                 "/keys/clear"))
              (let ((agenix-parity-exit-code 1))
                (agenix--identity-protected-p
                 "/keys/protected"))
              (let ((agenix-parity-exit-code 255))
                (agenix--identity-protected-p
                 "/keys/broken"))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (nil t t (("ssh-keygen" nil nil nil "-y" "-P" "" "-f" "/keys/clear") ("ssh-keygen" nil nil nil "-y" "-P" "" "-f" "/keys/protected") ("ssh-keygen" nil nil nil "-y" "-P" "" "-f" "/keys/broken")))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_password_prompt_uses_identity_path_and_returns_read_passwd_result() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'read-passwd)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      "correct horse")))
           (list
            (agenix--prompt-password
             "/home/user/.ssh/id_ed25519")
            (agenix--prompt-password
             "/path with spaces/key")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("correct horse" "correct horse" (("Password for /home/user/.ssh/id_ed25519: ") ("Password for /path with spaces/key: ")))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_temp_identity_runs_copy_then_rekey_and_reports_both_failure_stages() {
    let elisp_form = r##"(progn
         (defvar agenix-parity-copy-code nil)
         (defvar agenix-parity-rekey-code nil)
         (let* ((temp-path
                 (expand-file-name
                  "agenix-temp-identity-fixed"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                calls)
           (cl-letf (((symbol-function 'make-temp-file)
                      (lambda (prefix &rest arguments)
                        (push
                         (list
                          'make-temp-file
                          prefix
                          arguments)
                         calls)
                        temp-path))
                     ((symbol-function 'call-process)
                      (lambda (program
                               infile destination display
                               &rest arguments)
                        (push
                         (list
                          program infile destination
                          display arguments)
                         calls)
                        (if (equal program "cp")
                            agenix-parity-copy-code
                          agenix-parity-rekey-code))))
             (mapcar
              (lambda (case)
                (pcase-let ((`(,copy-code ,rekey-code) case))
                  (let ((agenix-parity-copy-code copy-code)
                        (agenix-parity-rekey-code rekey-code))
                    (list
                     case
                     (condition-case error-data
                         (agenix--create-temp-identity
                          "/keys/protected"
                          "password")
                       (error
                        (list
                         (car error-data)
                         (cadr error-data))))
                     (prog1
                         (nreverse calls)
                       (setq calls nil))))))
              '((0 0)
                (1 0)
                (0 1))))))"##;
    let expect = expect![[
        r#"OK (((0 0) "[ORACLE-SANDBOX]/agenix-temp-identity-fixed" ((make-temp-file "agenix-temp-identity" nil) ("cp" nil nil nil ("/keys/protected" "[ORACLE-SANDBOX]/agenix-temp-identity-fixed")) ("ssh-keygen" nil nil nil ("-p" "-P" "password" "-N" "" "-f" "[ORACLE-SANDBOX]/agenix-temp-identity-fixed")))) ((1 0) (error "Failed to create temporary copy of identity file. Please close the buffer and try again") ((make-temp-file "agenix-temp-identity" nil) ("cp" nil nil nil ("/keys/protected" "[ORACLE-SANDBOX]/agenix-temp-identity-fixed")))) ((0 1) (error "Failed to open private key /keys/protected. Wrong password? Please close the buffer and try again") ((make-temp-file "agenix-temp-identity" nil) ("cp" nil nil nil ("/keys/protected" "[ORACLE-SANDBOX]/agenix-temp-identity-fixed")) ("ssh-keygen" nil nil nil ("-p" "-P" "password" "-N" "" "-f" "[ORACLE-SANDBOX]/agenix-temp-identity-fixed")))))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_process_wrapper_captures_exit_output_arguments_and_always_kills_temp_buffer() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'call-process)
                    (lambda (program
                             infile destination display
                             &rest arguments)
                      (push
                       (list
                        program
                        infile
                        (buffer-name destination)
                        display
                        arguments)
                       calls)
                      (with-current-buffer destination
                        (insert
                         (propertize
                          "stdout λ\nstderr\n"
                          'face 'error)))
                      7)))
           (list
            (agenix--process-exit-code-and-output
             "/opt/program"
             "--flag"
             "value with spaces"
             "λ")
            (nreverse calls)
            (get-buffer "*age-buf*"))))"##;
    let expect = expect![[
        r#"OK ((7 "stdout λ\nstderr\n") (("/opt/program" nil "*age-buf*" nil ("--flag" "value with spaces" "λ"))) nil)"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}
