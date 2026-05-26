use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_html_export_drawer_special_footnote_filter_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Export Mix\n")
    (insert "* Kept\n")
    (insert ":LOGBOOK:\n")
    (insert "Drawer text with \\alpha.\n")
    (insert ":END:\n")
    (insert "#+ATTR_HTML: :class callout :data-x yes\n")
    (insert "#+begin_aside\n")
    (insert "Aside with [fn:a][fn:b] and [[https://example.org][link]].\n")
    (insert "#+end_aside\n")
    (insert "[fn:a] Alpha footnote.\n")
    (insert "[fn:b] Beta footnote.\n")
    (insert "** Hidden :noexport:\n")
    (insert "Should not export.\n")
    (insert "* COMMENT Commented\n")
    (insert "Should not export either.\n")
    (let* ((org-export-with-toc nil)
           (org-export-exclude-tags '("noexport"))
           (org-html-format-drawer-function
            (lambda (name contents)
              (format "<section class=\"drawer\" data-name=\"%s\">%s</section>"
                      name contents)))
           (html (org-export-as 'html nil nil t nil))
           (normalized
            (replace-regexp-in-string
             "org[[:alnum:]]+"
             "org-id"
             html)))
      (list
       (not (null (string-match-p "data-name=\"LOGBOOK\"" html)))
       (not (null (string-match-p "&alpha;" html)))
       (not (null (string-match-p "<aside" html)))
       (not (null (string-match-p "class=\"callout\"" html)))
       (not (null (string-match-p "footnotes" html)))
       (null (string-match-p "Should not export" html))
       normalized))))"##,
    );
}

#[test]
fn org_latex_export_entities_footnotes_special_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-latex)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Latex Mix\n")
    (insert "* H\n")
    (insert "Text \\alpha and \\rightarrow with [fn:x][fn:y].\n")
    (insert "#+ATTR_LATEX: :options frametitle={Box}\n")
    (insert "#+begin_tcolorbox\n")
    (insert "Inside *bold* and $x^2$.\n")
    (insert "#+end_tcolorbox\n")
    (insert "[fn:x] First footnote.\n")
    (insert "[fn:y] Second footnote with /italic/.\n")
    (let* ((org-export-with-toc nil)
           (latex (org-export-as 'latex nil nil t nil))
           (normalized
            (replace-regexp-in-string
             "sec:org[[:alnum:]]+"
             "sec:org-id"
             latex)))
      (list
       (not (null (string-match-p "\\\\alpha" latex)))
       (not (null (string-match-p "\\\\rightarrow" latex)))
       (not (null (string-match-p "\\\\footnote" latex)))
       (not (null (string-match-p "\\\\textsuperscript{,}" latex)))
       (not (null (string-match-p "\\\\begin{tcolorbox}" latex)))
       (not (null (string-match-p "frametitle={Box}" latex)))
       (not (null (string-match-p "\\\\textbf{bold}" latex)))
       normalized))))"##,
    );
}

#[test]
fn org_export_data_entities_footnote_numbers_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Data\n")
    (insert "* H\n")
    (insert "A \\beta ref [fn:1] and inline [fn::Inline note].\n")
    (insert "[fn:1] Defined note with [[https://example.org][url]].\n")
    (let* ((info (org-export-get-environment 'html nil nil))
           (tree (org-element-parse-buffer))
           (refs (org-element-map tree 'footnote-reference #'identity))
           (entities (org-element-map tree 'entity
                       (lambda (entity)
                         (list (org-element-property :name entity)
                               (org-element-property :html entity)
                               (org-element-property :latex entity)))))
           (numbers (mapcar
                     (lambda (ref)
                       (list (org-element-property :label ref)
                             (org-export-get-footnote-number ref info)
                             (org-export-footnote-first-reference-p ref info)))
                     refs))
           (rendered (mapcar
                      (lambda (ref)
                        (org-export-data ref info))
                      refs)))
      (list entities numbers rendered))))"##,
    );
}
