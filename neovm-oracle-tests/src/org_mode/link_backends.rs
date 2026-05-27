use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_link_doi_info_man_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ol-doi)
  (require 'ol-info)
  (require 'ol-man)
  (let ((org-link-doi-server-url "https://doi.example/"))
    (list
     (mapcar (lambda (backend)
               (org-link-doi-export "10.1000/foo bar" "A DOI" backend
                                    '(:ascii-links-to-notes nil)))
             '(html latex ascii texinfo md))
     (mapcar (lambda (backend)
               (org-info-export "elisp#Non-ASCII in Strings" "Strings" backend))
             '(html texinfo ascii))
     (mapcar (lambda (backend)
               (org-man-export "printf(3)::format" "printf" backend))
             '(html latex texinfo ascii md org)))))"##,
    );
}

#[test]
fn org_info_link_file_node_description_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ol-info)
  (list
   (mapcar #'org-info--link-file-node
           '(nil "" "emacs" "elisp#Non-ASCII in Strings"
                 "org:Tables" "info#:Special Node"))
   (mapcar #'org-info--expand-node-name
           '("Top" "Non-ASCII in Strings" "1.2 Weird/Node" "  spaced  "))
   (mapcar (lambda (pair)
             (org-info-description-as-command (car pair) (cdr pair)))
           '(("info:dir" . nil)
             ("info:elisp" . "")
             ("info:elisp#Non-ASCII in Strings" . nil)
             ("https://example.org" . "Desc")))))"##,
    );
}

#[test]
fn org_info_man_store_link_context_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ol)
  (require 'ol-info)
  (require 'ol-man)
  (let ((org-store-link-plist nil))
    (with-temp-buffer
      (setq major-mode 'Info-mode
            Info-current-file "/usr/share/info/elisp.info"
            Info-current-node "Symbols")
      (let ((info-link (org-info-store-link)))
        (let ((info-plist org-store-link-plist))
          (setq org-store-link-plist nil)
          (with-temp-buffer
            (rename-buffer "*Man printf*")
            (setq major-mode 'Man-mode)
            (let ((man-link (org-man-store-link)))
              (list info-link
                    info-plist
                    man-link
                    org-store-link-plist
                    (org-man-get-page-name))))))))"##,
    );
}
