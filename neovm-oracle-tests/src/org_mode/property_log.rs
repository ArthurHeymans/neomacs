use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_tags_multivalue_property_delete_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task :old:\n")
    (insert ":PROPERTIES:\n:A: 1\n:B: two\n:END:\n")
    (goto-char (point-min))
    (org-toggle-tag "new" 'on)
    (org-toggle-tag "old" 'off)
    (org-entry-put nil "A" "updated")
    (org-entry-put-multivalued-property nil "Multi" "x" "y" "z")
    (org-entry-delete nil "B")
    (list (org-get-tags)
          (org-entry-properties nil 'standard)
          (buffer-substring-no-properties (point-min) (point-max)))))"#,
    );
}

#[test]
fn org_archive_tag_toggle_parse_roundtrip_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-archive)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Active\n** DONE Child\nBody\n** TODO Keep\n")
    (goto-char (point-min))
    (search-forward "Child")
    (beginning-of-line)
    (org-toggle-archive-tag)
    (let ((after-archive
           (buffer-substring-no-properties (point-min) (point-max))))
      (org-toggle-archive-tag)
      (list after-archive
            (buffer-substring-no-properties (point-min) (point-max))
            (org-element-map (org-element-parse-buffer) 'headline
              (lambda (headline)
                (list (org-element-property :raw-value headline)
                      (org-element-property :tags headline))))))))"#,
    );
}

#[test]
fn org_done_log_drawer_timestamp_normalized_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task\n")
    (goto-char (point-min))
    (let ((org-log-into-drawer t)
          (org-log-note-clock-out nil)
          (org-log-done 'time))
      (org-todo "DONE")
      (list (org-log-beginning t)
            (replace-regexp-in-string
             "CLOSED: \\[.*\\]"
             "CLOSED: [stamp]"
             (buffer-substring-no-properties (point-min) (point-max)))))))"#,
    );
}

#[test]
fn org_property_inheritance_allowed_cycle_delete_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-use-property-inheritance '("Owner" "Milestone"))
          (changes nil))
      (add-hook 'org-property-changed-functions
                (lambda (key value) (push (list key value) changes))
                nil t)
      (org-mode)
      (insert "#+PROPERTY: Status_ALL Todo Doing Done :ETC\n")
      (insert "* Project\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:Milestone: M1\n:END:\n")
      (insert "** Task\n")
      (insert ":PROPERTIES:\n:Status: Todo\n:Owner: Bea\n:Other: keep\n:END:\n")
      (goto-char (point-min))
      (search-forward "Task")
      (beginning-of-line)
      (let ((inherited (list (org-entry-get nil "Owner" 'inherit)
                             (org-entry-get nil "Milestone" 'inherit)
                             (org-entry-get-with-inheritance "Milestone")))
            (allowed (org-property-get-allowed-values nil "Status" 'table)))
        (search-forward ":Status:")
        (org-property-next-allowed-value)
        (org-property-next-allowed-value)
        (org-property-previous-allowed-value)
        (goto-char (point-min))
        (search-forward "Task")
        (beginning-of-line)
        (org-entry-add-to-multivalued-property nil "Multi" "x")
        (org-entry-add-to-multivalued-property nil "Multi" "y")
        (org-entry-remove-from-multivalued-property nil "Multi" "x")
        (org-entry-delete nil "Other")
        (list inherited
              allowed
              (org-entry-get nil "Status")
              (org-entry-get-multivalued-property nil "Multi")
              (org-entry-member-in-multivalued-property nil "Multi" "y")
              (nreverse changes)
              (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_property_values_global_delete_roundtrip_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+PROPERTY: Owner_ALL Ada Bea Cy\n")
    (insert "* A\n:PROPERTIES:\n:Owner: Ada\n:Effort: 0:30\n:END:\n")
    (insert "** A1\n:PROPERTIES:\n:Owner: Bea\n:Effort: 0:15\n:END:\n")
    (insert "* B\n:PROPERTIES:\n:Owner: Ada\n:Effort: 1:00\n:END:\n")
    (goto-char (point-min))
    (let ((owners-before (sort (copy-sequence (org-property-values "Owner"))
                               #'string<))
          (efforts-before (sort (copy-sequence (org-property-values "Effort"))
                                #'string<)))
      (org-delete-property-globally "Effort")
      (goto-char (point-min))
      (search-forward "A1")
      (beginning-of-line)
      (org-entry-put nil "Owner" "Cy")
      (list owners-before
            efforts-before
            (sort (copy-sequence (org-property-values "Owner")) #'string<)
            (org-property-values "Effort")
            (org-entry-properties nil 'standard)
            (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}
