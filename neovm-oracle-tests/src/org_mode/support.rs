use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_pcomplete_case_command_at_point_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-pcomplete)
  (with-temp-buffer
    (org-mode)
    (insert "#+STARTUP: fold\n")
    (insert "#+PROPERTY: Effort_ALL 0:15 0:30\n")
    (insert "* TODO Heading\n")
    (insert ":PROPERTIES:\n:Effort: 0:15\n:END:\n")
    (goto-char (point-min))
    (search-forward "STARTUP")
    (list (org-pcomplete-case-double '("todo" "done" "Wait"))
          (org-thing-at-point)
          (org-command-at-point))))"##,
    );
}

#[test]
fn org_ctags_lookup_replace_tag_table_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-ctags)
  (let* ((root (make-temp-file "org-ctags" t))
         (topic (expand-file-name "topic.org" root))
         (tags (expand-file-name "TAGS" root))
         (tags-file-name tags))
    (unwind-protect
        (progn
          (with-temp-file topic
            (insert "* Alpha\nBody\n* Beta\nBody\n"))
          (with-temp-file tags
            (insert "\f\n" topic ",20\n"
                    "Alpha\177Alpha\0011,1\n"
                    "Beta\177Beta\0013,14\n"))
          (let ((found (org-ctags-get-filename-for-tag "Alpha")))
            (list (org-ctags-string-search-and-replace
                   "a" "X" "abracadabra")
                  (list (file-name-nondirectory (nth 0 found))
                        (nth 1 found)
                        (nth 2 found))
                  (sort (org-ctags-all-tags-in-current-tags-table)
                        #'string<))))
      (delete-directory root t))))"#,
    );
}

#[test]
fn org_crypt_detect_encrypted_entry_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-crypt)
  (with-temp-buffer
    (org-mode)
    (insert "* Secret :crypt:\n")
    (insert ":PROPERTIES:\n:CRYPTKEY: nil\n:END:\n")
    (insert "-----BEGIN PGP MESSAGE-----\nabc\n-----END PGP MESSAGE-----\n")
    (insert "* Plain\n")
    (goto-char (point-min))
    (search-forward "Secret")
    (beginning-of-line)
    (let ((encrypted (org-at-encrypted-entry-p))
          (key (let ((org-crypt-key nil))
                 (org-crypt-key-for-heading))))
      (list (and encrypted
                 (list (- (car encrypted) (point-min))
                       (- (cdr encrypted) (point-min))))
            key
            (and encrypted
                 (org-crypt--encrypted-text
                  (car encrypted)
                  (cdr encrypted)))))))"#,
    );
}
