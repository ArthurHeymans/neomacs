use expect_test::expect;

use super::assert_agenix_parity;

#[test]
fn agenix_mode_runs_pre_hook_then_decrypt_and_installs_complete_buffer_lifecycle() {
    let elisp_form = r##"(let (events)
         (cl-letf (((symbol-function 'agenix-decrypt-buffer)
                    (lambda (&optional buffer)
                      (push
                       (list
                        'decrypt
                        buffer
                        buffer-read-only
                        agenix-age-program
                        (point))
                       events)
                      (read-only-mode -1)
                      (erase-buffer)
                      (insert "decrypted content")
                      'decrypted)))
           (with-temp-buffer
             (insert "ciphertext")
             (goto-char (point-max))
             (setq buffer-file-name
                   "/repo/secret.age"
                   buffer-auto-save-file-name
                   "/repo/#secret.age#"
                   require-final-newline t
                   agenix-pre-mode-hook
                   (list
                    (lambda ()
                      (push
                       (list
                        'pre
                        buffer-read-only
                        major-mode
                        (point))
                       events)
                      (setq agenix-age-program
                            "/nix/store/age"))))
             (agenix-mode)
             (list
              major-mode
              mode-name
              (derived-mode-p 'text-mode)
              (buffer-string)
              (point)
              buffer-read-only
              buffer-undo-list
              require-final-newline
              buffer-auto-save-file-name
              write-contents-functions
              (length after-revert-hook)
              (local-variable-p
               'after-revert-hook)
              (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (agenix-mode "agenix" text-mode "decrypted content" 1 nil nil nil nil (agenix-save-decrypted) 2 nil ((pre t agenix-mode 11) (decrypt nil t "/nix/store/age" 11)))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_after_revert_hook_only_decrypts_buffers_still_using_agenix_mode() {
    let elisp_form = r##"(let (events saved-hook)
         (cl-letf (((symbol-function 'agenix-decrypt-buffer)
                    (lambda (&optional buffer)
                      (push
                       (list
                        'decrypt
                        buffer
                        major-mode)
                       events)
                      'decrypted)))
           (with-temp-buffer
             (setq agenix-pre-mode-hook nil)
             (agenix-mode)
             (setq saved-hook
                   (copy-sequence after-revert-hook))
             (run-hooks 'after-revert-hook)
             (fundamental-mode)
             (let ((after-revert-hook saved-hook))
               (run-hooks 'after-revert-hook))
             (nreverse events))))"##;
    let expect = expect!["OK ((decrypt nil agenix-mode) (decrypt nil agenix-mode))"];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_mode_reentry_resets_undo_point_and_write_hook_after_existing_local_state() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'agenix-decrypt-buffer)
                    (lambda (&optional _buffer)
                      (push
                       (list
                        (point)
                        buffer-undo-list
                        write-contents-functions)
                       calls)
                      (read-only-mode -1)
                      'decrypted)))
           (with-temp-buffer
             (insert "ciphertext")
             (setq agenix-pre-mode-hook nil
                   buffer-undo-list
                   '((old . undo))
                   write-contents-functions
                   '(old-writer)
                   require-final-newline t
                   buffer-auto-save-file-name
                   "/auto-save")
             (goto-char (point-max))
             (agenix-mode)
             (let ((first
                    (list
                     (point)
                     buffer-undo-list
                     write-contents-functions
                     require-final-newline
                     buffer-auto-save-file-name)))
               (setq buffer-undo-list
                     '((new . undo))
                     write-contents-functions
                     '(other-writer))
               (goto-char (point-max))
               (agenix-mode)
               (list
                first
                (point)
                buffer-undo-list
                write-contents-functions
                (nreverse calls))))))"##;
    let expect = expect![
        "OK ((1 nil #1=(agenix-save-decrypted) nil nil) 1 nil #1# ((11 ((old . undo)) nil) (11 ((new . undo)) nil)))"
    ];
    assert_agenix_parity(elisp_form, expect);
}
