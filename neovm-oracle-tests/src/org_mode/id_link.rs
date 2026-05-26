use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_id_create_save_reload_find_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-id)
  (let* ((file (make-temp-file
                "org-id-round" nil ".org"
                "* A\n:PROPERTIES:\n:ID: a-id\n:END:\nBody\n* B\n"))
         (org-id-locations-file (make-temp-file "org-id-loc"))
         (org-id-track-globally t)
         (org-id-method 'org))
    (unwind-protect
        (progn
          (org-id-update-id-locations (list file) t)
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (goto-char (point-min))
            (search-forward "* B")
            (beginning-of-line)
            (let ((new (org-id-get-create)))
              (org-id-locations-save)
              (clrhash org-id-locations)
              (org-id-locations-load)
              (list (not (null (string-match-p "\\`[[:alnum:]]+\\'" new)))
                    (file-name-extension (org-id-find-id-file "a-id"))
                    (hash-table-count org-id-locations)
                    (markerp (org-id-find new t))
                    (with-current-buffer (marker-buffer (org-id-find "a-id" t))
                      (org-get-heading t t t t))
                    (replace-regexp-in-string
                     (regexp-quote new)
                     "<generated-id>"
                     (buffer-substring-no-properties
                      (point-min) (point-max)))))))
      (when (get-file-buffer file)
        (kill-buffer (get-file-buffer file)))
      (delete-file file)
      (when (file-exists-p org-id-locations-file)
        (delete-file org-id-locations-file)))))"#,
    );
}

#[test]
fn org_fuzzy_link_search_and_open_heading_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'ol)
  (with-temp-buffer
    (org-mode)
    (insert "* Target\nBody line\n* Other\n[[*Target][go]]\n")
    (goto-char (point-min))
    (search-forward "Other")
    (search-forward "[[")
    (let ((link (org-element-context)))
      (list (org-element-property :type link)
            (org-element-property :path link)
            (save-excursion
              (org-link-search "*Target")
              (org-get-heading t t t t))
            (save-excursion
              (org-open-at-point)
              (org-get-heading t t t t))))))"#,
    );
}
