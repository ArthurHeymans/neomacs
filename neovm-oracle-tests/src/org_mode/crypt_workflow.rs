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

#[test]
fn org_crypt_decrypt_nested_headings_autosave_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-crypt)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-crypt-key "fallback@example.org")
          (org-crypt-tag-matcher "crypt")
          (org-crypt-disable-auto-save 'encrypt)
          (cipher-table nil)
          (calls nil))
      (cl-labels
          ((cipher-for
            (name plain)
            (let ((cipher
                   (format "-----BEGIN PGP MESSAGE-----\n%s\n-----END PGP MESSAGE-----\n"
                           name)))
              (push (cons (org-crypt--encrypted-text
                           1 (with-temp-buffer
                               (insert cipher)
                               (point-max)))
                          plain)
                    cipher-table)
              cipher))
           (snapshot
            (label)
            (list label
                  (buffer-substring-no-properties
                   (point-min) (point-max))
                  (mapcar
                   (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward needle nil t)
                       (and (match-beginning 0)
                            (list needle
                                  (line-number-at-pos)
                                  (org-current-level)
                                  (org-at-heading-p)
                                  (not (null
                                        (org-invisible-p
                                         (line-beginning-position))))))))
                   '("Vault" "Child one" "Grand child" "Peer one"
                     "Symmetric" "Sym child" "Plain" "Not encrypted"))
                  (mapcar
                   (lambda (fn)
                     (cond ((eq fn 'org-encrypt-entries)
                            'org-encrypt-entries)
                           ((functionp fn) 'function)
                           (t fn)))
                   auto-save-hook))))
        (let* ((plain-a "* Child one\nchild body\n** Grand child\ngrand body\n* Peer one\npeer body\n")
               (plain-b "** Sym child\nsym body\n"))
          (cl-letf (((symbol-function 'epg-make-context)
                     (lambda (&rest args)
                       (push (cons 'context args) calls)
                       'mock-context))
                    ((symbol-function 'epg-list-keys)
                     (lambda (_context name &optional mode)
                       (push (list 'keys name mode) calls)
                       (and name
                            (not (string= name ""))
                            (list (concat "KEY:" name)))))
                    ((symbol-function 'epg-encrypt-string)
                     (lambda (_context plain recipients &optional sign trust)
                       (let ((cipher
                              (format "-----BEGIN PGP MESSAGE-----\nre:%s\n-----END PGP MESSAGE-----\n"
                                      (sha1 plain))))
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
            (auto-save-mode 1)
            (insert "* Vault :crypt:\n")
            (insert ":PROPERTIES:\n:CRYPTKEY: bob@example.org\n:END:\n")
            (insert (cipher-for "cipher-a" plain-a))
            (insert "* Symmetric :crypt:\n")
            (insert ":PROPERTIES:\n:CRYPTKEY: nil\n:END:\n")
            (insert (cipher-for "cipher-b" plain-b))
            (insert "* Plain\nNot encrypted\n")
            (goto-char (point-min))
            (org-fold-hide-subtree)
            (let ((initial (snapshot 'initial)))
              (org-decrypt-entries)
              (let ((after-decrypt (snapshot 'after-decrypt))
                    (encrypted-after-decrypt
                     (mapcar
                      (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (search-forward needle)
                          (beginning-of-line)
                          (org-at-encrypted-entry-p)))
                      '("Vault" "Symmetric" "Plain"))))
                (run-hooks 'auto-save-hook)
                (let ((after-autosave (snapshot 'after-autosave))
                      (encrypted-after-autosave
                       (mapcar
                        (lambda (needle)
                          (save-excursion
                            (goto-char (point-min))
                            (search-forward needle)
                            (beginning-of-line)
                            (not (null (org-at-encrypted-entry-p)))))
                        '("Vault" "Symmetric" "Plain"))))
                  (list initial
                        after-decrypt
                        encrypted-after-decrypt
                        after-autosave
                        encrypted-after-autosave
                        (mapcar
                         (lambda (call)
                           (pcase (car-safe call)
                             ('encrypt
                              (list 'encrypt
                                    (nth 1 call)
                                    (string-match-p
                                     "Child one\\|Sym child"
                                     (nth 2 call))
                                    (string-match-p
                                     "^\\*\\* Child one"
                                     (nth 2 call))))
                             ('decrypt
                              (list 'decrypt
                                    (string-match-p
                                     "BEGIN PGP MESSAGE"
                                     (nth 1 call))))
                             (_ call)))
                         (reverse calls)))))))))))"##,
    );
}
