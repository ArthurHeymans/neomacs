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
