use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_id_create_save_reload_find_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
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
        (delete-file org-id-locations-file)))))"##,
    );
}

#[test]
fn org_fuzzy_link_search_and_open_heading_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
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
              (org-get-heading t t t t))))))"##,
    );
}

#[test]
fn org_id_relative_locations_reload_and_find_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-id)
  (let* ((root (make-temp-file "org-id-relative" t))
         (sub (expand-file-name "sub" root))
         (file-a (expand-file-name "a.org" root))
         (file-b (expand-file-name "b.org" sub))
         (org-id-locations-file (expand-file-name "ids.el" root))
         (org-id-locations-file-relative t)
         (org-id-track-globally t))
    (unwind-protect
        (progn
          (make-directory sub)
          (with-temp-file file-a
            (insert "* A\n:PROPERTIES:\n:ID: rel-a\n:END:\n"))
          (with-temp-file file-b
            (insert "* B\n:PROPERTIES:\n:ID: rel-b\n:END:\n"))
          (org-id-update-id-locations (list file-a file-b) t)
          (org-id-locations-save)
          (let ((raw (with-temp-buffer
                       (insert-file-contents org-id-locations-file)
                       (buffer-string))))
            (setq org-id-locations nil)
            (org-id-locations-load)
            (let ((marker-a (org-id-find "rel-a" t))
                  (marker-b (org-id-find "rel-b" t)))
              (list raw
                    (hash-table-count org-id-locations)
                    (file-name-nondirectory (org-id-find-id-file "rel-a"))
                    (file-relative-name (org-id-find-id-file "rel-b") root)
                    (and marker-a
                         (with-current-buffer (marker-buffer marker-a)
                           (org-get-heading t t t t)))
                    (and marker-b
                         (with-current-buffer (marker-buffer marker-b)
                           (org-get-heading t t t t)))))))
      (when (get-file-buffer file-a) (kill-buffer (get-file-buffer file-a)))
      (when (get-file-buffer file-b) (kill-buffer (get-file-buffer file-b)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_id_store_parent_context_and_open_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-id)
  (require 'ol)
  (let* ((file (make-temp-file "org-id-parent" nil ".org"
                               "* Parent\n:PROPERTIES:\n:ID: parent-id\n:END:\n** Child\nBody\n** Sibling\n"))
         (org-id-locations-file (make-temp-file "org-id-parent-loc"))
         (org-id-track-globally t)
         (org-id-link-to-org-use-id 'use-existing)
         (org-id-link-consider-parent-id t)
         (org-id-link-use-context t)
         (org-link-context-for-files t)
         (org-store-link-plist nil))
    (unwind-protect
        (with-current-buffer (find-file-noselect file)
          (org-mode)
          (org-id-update-id-locations (list file) t)
          (goto-char (point-min))
          (search-forward "** Child")
          (beginning-of-line)
          (let* ((stored (org-id-store-link))
                 (plist org-store-link-plist))
            (goto-char (point-min))
            (search-forward "** Sibling")
            (beginning-of-line)
            (org-id-open (substring stored 3) nil)
            (list stored
                  plist
                  (org-get-heading t t t t)
                  (org-entry-get nil "ID")
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-file file)
      (when (file-exists-p org-id-locations-file)
        (delete-file org-id-locations-file)))))"##,
    );
}

#[test]
fn org_store_link_custom_id_and_id_policy_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-id)
  (require 'ol)
  (let* ((file (make-temp-file "org-store-custom" nil ".org"
                               "#+TITLE: Store\n* Target\n:PROPERTIES:\n:CUSTOM_ID: custom-target\n:ID: explicit-id\n:END:\nBody\n"))
         (org-id-locations-file (make-temp-file "org-store-custom-loc"))
         (org-id-track-globally t)
         (org-id-link-to-org-use-id 'create-if-interactive-and-no-custom-id)
         (org-link-context-for-files t)
         (org-id-link-use-context t)
         (org-stored-links nil)
         (org-store-link-plist nil))
    (unwind-protect
        (with-current-buffer (find-file-noselect file)
          (org-mode)
          (org-id-update-id-locations (list file) t)
          (goto-char (point-min))
          (search-forward "Target")
          (beginning-of-line)
          (let ((noninteractive (org-store-link nil nil))
                (plist-after-noninteractive org-store-link-plist))
            (setq org-store-link-plist nil
                  org-stored-links nil)
            (let ((interactive (org-store-link nil t)))
              (list noninteractive
                    plist-after-noninteractive
                    interactive
                    org-stored-links
                    org-store-link-plist
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-file file)
      (when (file-exists-p org-id-locations-file)
        (delete-file org-id-locations-file)))))"##,
    );
}
