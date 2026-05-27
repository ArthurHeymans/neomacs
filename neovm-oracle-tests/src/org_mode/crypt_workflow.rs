use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_crypt_stubbed_encrypt_decrypt_reuse_hook_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-crypt)
  (with-temp-buffer
    (let ((org-crypt-key "default@example.org")
          (org-crypt-tag-matcher "crypt")
          (org-crypt-disable-auto-save nil)
          (cipher-table nil)
          (calls nil))
      (cl-letf (((symbol-function 'epg-make-context)
                 (lambda (&rest args)
                   (push (cons 'context args) calls)
                   'mock-context))
                ((symbol-function 'epg-list-keys)
                 (lambda (_context name &optional mode)
                   (push (list 'keys name mode) calls)
                   (and (not (string= name ""))
                        (list (concat "KEY:" name)))))
                ((symbol-function 'epg-encrypt-string)
                 (lambda (_context plain recipients &optional sign trust)
                   (let ((cipher
                          (format "-----BEGIN PGP MESSAGE-----\nkey=%S sign=%S trust=%S\nsha=%s\n-----END PGP MESSAGE-----\n"
                                  recipients sign trust (sha1 plain))))
                     (push (list 'encrypt recipients plain) calls)
                     (push (cons (org-crypt--encrypted-text
                                  1 (with-temp-buffer
                                      (insert cipher)
                                      (point-max)))
                                 plain)
                           cipher-table)
                     cipher)))
                ((symbol-function 'epg-decrypt-string)
                 (lambda (_context cipher)
                   (push (list 'decrypt cipher) calls)
                   (or (cdr (assoc cipher cipher-table))
                       (error "missing cipher")))))
        (org-mode)
        (insert "* Secrets :crypt:\n")
        (insert ":PROPERTIES:\n:CRYPTKEY: alice@example.org\n:END:\n")
        (insert "Plain alpha\n")
        (insert "** Nested raw\nBody nested\n")
        (insert "* Symmetric :crypt:\n")
        (insert ":PROPERTIES:\n:CRYPTKEY: nil\n:END:\n")
        (insert "Plain beta\n")
        (let ((initial (buffer-substring-no-properties
                        (point-min) (point-max))))
          (goto-char (point-min))
          (org-encrypt-entries)
          (let ((after-encrypt
                 (buffer-substring-no-properties (point-min) (point-max)))
                (encrypted-regions
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (search-forward needle)
                      (beginning-of-line)
                      (org-at-encrypted-entry-p)))
                  '("Secrets" "Symmetric"))))
            (goto-char (point-min))
            (org-decrypt-entry)
            (let ((after-first-decrypt
                   (buffer-substring-no-properties
                    (point-min) (point-max))))
              (org-encrypt-entry)
              (let ((after-reencrypt
                     (buffer-substring-no-properties
                      (point-min) (point-max))))
                (goto-char (point-min))
                (search-forward "Symmetric")
                (beginning-of-line)
                (org-decrypt-entry)
                (goto-char (point-min))
                (org-crypt-use-before-save-magic)
                (let ((hooks (mapcar (lambda (fn)
                                       (cond ((eq fn 'org-encrypt-entries)
                                              'org-encrypt-entries)
                                             ((functionp fn) 'function)
                                             (t fn)))
                                     org-mode-hook))
                      (encrypt-calls
                       (mapcar (lambda (call)
                                 (and (eq (car-safe call) 'encrypt)
                                      (list (nth 1 call)
                                            (string-match-p
                                             "Plain alpha\\|Plain beta"
                                             (nth 2 call)))))
                               (reverse calls))))
                  (list initial
                        after-encrypt
                        encrypted-regions
                        after-first-decrypt
                        after-reencrypt
                        (buffer-substring-no-properties
                         (point-min) (point-max))
                        encrypt-calls
                        hooks
                        (mapcar (lambda (call)
                                  (if (eq (car-safe call) 'decrypt)
                                      (list 'decrypt
                                            (string-match-p
                                             "BEGIN PGP MESSAGE"
                                             (nth 1 call)))
                                    call))
                                (reverse calls)))))))))))"##,
    );
}
