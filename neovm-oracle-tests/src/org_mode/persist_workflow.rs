use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_persist_grouped_elisp_version_load_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org-persist)
  (let* ((root (make-temp-file "org-persist" t))
         (org-persist-directory (expand-file-name "cache/" root))
         (org-persist--index nil)
         (org-persist--index-hash nil)
         (org-persist--index-age nil)
         (org-persist--write-cache (make-hash-table :test #'equal))
         (org-persist-before-write-hook nil)
         (org-persist-before-read-hook nil)
         (org-persist-after-read-hook nil)
         (org-persist-default-expiry 'never)
         (org-persist-test-var '(:old nil))
         (read-events nil))
    (unwind-protect
        (progn
          (add-hook 'org-persist-after-read-hook
                    (lambda (container associated)
                      (push (list container associated) read-events)))
          (setq org-persist-test-var '(:value (1 2 3) :label "alpha"))
          (org-persist-register
           '((elisp org-persist-test-var)
             (version "v1")
             "literal"
             :keyword)
           '(:key "suite")
           :write-immediately t)
          (let ((read-one (org-persist-read
                           'org-persist-test-var '(:key "suite")))
                (read-related (org-persist-read
                               '(version "v1") '(:key "suite")
                               nil nil :read-related t)))
            (setq org-persist-test-var '(:reset t))
            (let ((loaded (org-persist-load
                           'org-persist-test-var '(:key "suite"))))
              (org-persist-unregister
               '(version "v1") '(:key "suite") :remove-related t)
              (list read-one
                    read-related
                    loaded
                    org-persist-test-var
                    (mapcar #'car (reverse read-events))
                    (org-persist-read
                     'org-persist-test-var '(:key "suite"))))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_persist_buffer_local_hash_match_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-persist)
  (defvar org-persist-test-buffer-var nil)
  (let* ((root (make-temp-file "org-persist-buffer" t))
         (file (expand-file-name "note.org" root))
         (org-persist-directory (expand-file-name "cache/" root))
         (org-persist--index nil)
         (org-persist--index-hash nil)
         (org-persist--index-age nil)
         (org-persist--write-cache (make-hash-table :test #'equal))
         (org-persist-default-expiry 'never))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* A\nBody\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (setq-local org-persist-test-buffer-var
                        '(:buffer-value "original" :items (a b)))
            (org-persist-register
             'org-persist-test-buffer-var (current-buffer)
             :write-immediately t)
            (let ((same-hash (org-persist-read
                              'org-persist-test-buffer-var
                              (current-buffer) t)))
              (goto-char (point-max))
              (insert "Changed in memory.\n")
              (let ((mismatch-hash (org-persist-read
                                    'org-persist-test-buffer-var
                                    (current-buffer) t))
                    (ignore-hash (org-persist-read
                                  'org-persist-test-buffer-var
                                  (current-buffer) nil)))
                (setq-local org-persist-test-buffer-var '(:reset t))
                (org-persist-load
                 'org-persist-test-buffer-var
                 (list :file file))
                (list same-hash
                      mismatch-hash
                      ignore-hash
                      org-persist-test-buffer-var
                      (not (null kill-buffer-hook)))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_persist_file_container_gc_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org-persist)
  (let* ((root (make-temp-file "org-persist-file" t))
         (source (expand-file-name "source.txt" root))
         (org-persist-directory (expand-file-name "cache/" root))
         (org-persist--index nil)
         (org-persist--index-hash nil)
         (org-persist--index-age nil)
         (org-persist--write-cache (make-hash-table :test #'equal))
         (org-persist-default-expiry 'never))
    (unwind-protect
        (progn
          (with-temp-file source
            (insert "one\ntwo\n"))
          (org-persist-register
           '((file) (version "file-v1") "payload")
           source
           :write-immediately t)
          (let* ((stored (org-persist-read '(file) source))
                 (stored-exists (and stored (file-exists-p stored)))
                 (stored-text (and stored
                                   (with-temp-buffer
                                     (insert-file-contents stored)
                                     (buffer-string))))
                 (related (org-persist-read
                           '(version "file-v1") source
                           nil nil :read-related t)))
            (org-persist-unregister
             '(file) source :remove-related t)
            (list stored-exists
                  stored-text
                  (mapcar (lambda (x)
                            (if (and (stringp x)
                                     (file-name-absolute-p x))
                                "<persist-file>"
                              x))
                          related)
                  (and stored (file-exists-p stored))
                  (org-persist-read '(file) source))))
      (delete-directory root t))))"##,
    );
}
