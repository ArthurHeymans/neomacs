use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_builtin_link_export_backends_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'ol-doi)
  (require 'ol-man)
  (list
   (org-link-doi-export "10.1000/xyz123" "Paper" 'html nil)
   (org-link-doi-export "10.1000/xyz123" "Paper" 'latex nil)
   (org-link-doi-export "10.1000/xyz123" nil 'ascii nil)
   (org-man-export "printf(3)" "Printf" 'html)
   (org-man-export "printf(3)" nil 'latex)
   (org-man-export "printf(3)" "Printf" 'ascii)))"#,
    );
}

#[test]
fn org_man_export_sections_markup_links_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-man)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Demo Manual\n")
    (insert "#+AUTHOR: Ada\n")
    (insert "* NAME\n")
    (insert "demo - short description\n")
    (insert "* SYNOPSIS\n")
    (insert "=demo --help=\n")
    (insert "* DESCRIPTION\n")
    (insert "Text with *bold*, /italic/, and [[https://example.org][link]].\n")
    (let* ((org-export-with-toc nil)
           (man (org-export-as 'man nil nil t nil)))
      (list (not (null (string-match-p "\\.SH \"NAME\"" man)))
            (not (null (string-match-p "\\.SH \"SYNOPSIS\"" man)))
            (not (null (string-match-p "\\\\fBbold\\\\fP" man)))
            (not (null (string-match-p "\\\\fIitalic\\\\fP" man)))
            (not (null (string-match-p "\\\\fIdemo \\\\-\\\\-help\\\\fP" man)))
            (not (null (string-match-p "https://example.org" man)))
            man))))"##,
    );
}
