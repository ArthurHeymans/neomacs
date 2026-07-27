use expect_test::expect;

use super::assert_agenix_parity;

#[test]
fn agenix_save_encrypts_real_buffer_with_all_recipients_then_reverts_and_records_restore_state() {
    let elisp_form = r##"(let (process-calls revert-calls)
         (cl-letf (((symbol-function 'call-process-region)
                    (lambda (start end program delete
                             destination display
                             &rest arguments)
                      (push
                       (list
                        start end program delete
                        (buffer-name destination)
                        display arguments)
                       process-calls)
                      (with-current-buffer destination
                        (insert "age encryption output"))
                      0))
                   ((symbol-function 'revert-buffer)
                    (lambda (&rest arguments)
                      (push arguments revert-calls)
                      (erase-buffer)
                      (insert "CIPHERTEXT")
                      (goto-char (point-min))
                      'reverted)))
           (with-temp-buffer
             (insert "plain secret λ\nsecond line")
             (goto-char 8)
             (setq buffer-undo-list
                   '((8 . 9))
                   agenix--keys
                   '("age1alice"
                     "ssh-ed25519 AAA"
                     "age1alice")
                   agenix--encrypted-fp
                   "/repo/secret.age"
                   agenix-age-program
                   "/opt/age")
             (set-buffer-modified-p t)
             (list
              (agenix-save-decrypted)
              (buffer-string)
              (point)
              agenix--point
              agenix--undo-list
              (buffer-modified-p)
              (nreverse process-calls)
              (nreverse revert-calls)))))"##;
    let expect = expect![[
        r#"OK (t "CIPHERTEXT" 1 8 ((8 . 9)) nil (("plain secret λ\nsecond line" nil "/opt/age" nil "*age-buf*" t ("--encrypt" "--recipient" "age1alice" "--recipient" "ssh-ed25519 AAA" "--recipient" "age1alice" "-o" "/repo/secret.age"))) ((:ignore-auto :noconfirm :preserve-modes)))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_save_failure_surfaces_age_output_without_revert_or_state_replacement() {
    let elisp_form = r##"(let (process-calls revert-calls)
         (cl-letf (((symbol-function 'call-process-region)
                    (lambda (start end program delete
                             destination display
                             &rest arguments)
                      (push
                       (list
                        start end program delete
                        (buffer-name destination)
                        display arguments)
                       process-calls)
                      (with-current-buffer destination
                        (insert "recipient rejected\n"))
                      2))
                   ((symbol-function 'revert-buffer)
                    (lambda (&rest arguments)
                      (push arguments revert-calls)
                      'unexpected)))
           (with-temp-buffer
             (insert "plain secret")
             (goto-char 4)
             (setq agenix--keys '("bad-key")
                   agenix--encrypted-fp
                   "/repo/secret.age"
                   agenix--point 'old-point
                   agenix--undo-list 'old-undo
                   buffer-undo-list
                   '((1 . 2)))
             (set-buffer-modified-p t)
             (list
              (condition-case error-data
                  (agenix-save-decrypted)
                (error
                 (list
                  (car error-data)
                  (cadr error-data))))
              (buffer-string)
              (point)
              agenix--point
              agenix--undo-list
              (buffer-modified-p)
              (nreverse process-calls)
              (nreverse revert-calls)))))"##;
    let expect = expect![[
        r#"OK ((error "recipient rejected\n") "plain secret" 4 old-point old-undo t (("plain secret" nil "age" nil "*age-buf*" t ("--encrypt" "--recipient" "bad-key" "-o" "/repo/secret.age"))) nil)"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_save_optional_buffer_targets_named_buffer_without_switching_caller() {
    let elisp_form = r##"(let ((caller
                (generate-new-buffer
                 " *agenix-save-caller*"))
               (target
                (generate-new-buffer
                 " *agenix-save-target*"))
               events)
         (unwind-protect
             (progn
               (with-current-buffer caller
                 (insert "caller text"))
               (with-current-buffer target
                 (insert "target plaintext")
                 (setq agenix--keys nil
                       agenix--encrypted-fp
                       "/repo/empty-recipients.age"))
               (cl-letf (((symbol-function
                           'call-process-region)
                          (lambda (start end program
                                   delete destination
                                   display
                                   &rest arguments)
                            (push
                             (list
                              start end program delete
                              (buffer-name destination)
                              display arguments
                              (buffer-name
                               (current-buffer)))
                             events)
                            0))
                         ((symbol-function 'revert-buffer)
                          (lambda (&rest arguments)
                            (push
                             (list
                              'revert
                              (buffer-name)
                              arguments)
                             events)
                            'reverted)))
                 (with-current-buffer caller
                   (let ((before (current-buffer)))
                     (list
                      (agenix-save-decrypted target)
                      (eq before (current-buffer))
                      (buffer-string)
                      (with-current-buffer target
                        (list
                         (buffer-string)
                         agenix--point
                         agenix--undo-list
                         (buffer-modified-p)))
                      (nreverse events))))))
           (dolist (buffer (list caller target))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (t t "caller text" ("target plaintext" 17 t nil) (("target plaintext" nil "age" nil "*age-buf*" t ("--encrypt" "-o" "/repo/empty-recipients.age") " *agenix-save-target*") (revert " *agenix-save-target*" (:ignore-auto :noconfirm :preserve-modes))))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}
