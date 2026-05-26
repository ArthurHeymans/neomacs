use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_cite_processor_declaration_and_plist_parse_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'oc)
  (list (org-cite--parse-as-plist ":style author-year :notes t :foo bar")
        (org-cite-read-processor-declaration "basic author-year")
        (org-cite-read-processor-declaration
         "biblatex bibstyle=authoryear citestyle=authoryear")))"#,
    );
}

#[test]
fn org_cite_bibliography_and_reference_boundaries_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (with-temp-buffer
    (org-mode)
    (insert "#+bibliography: refs.bib more.json\n")
    (insert "Text [cite/t:@doe2020 p. 4; see @roe2021] and [cite:@solo].\n")
    (let* ((tree (org-element-parse-buffer))
           (citations
            (org-element-map tree 'citation
              (lambda (citation)
                (list (org-element-property :style citation)
                      (org-cite-get-references citation t)
                      (let ((bounds (org-cite-boundaries citation)))
                        (cons (- (car bounds) (point-min))
                              (- (cdr bounds) (point-min))))
                      (org-cite-main-affixes citation))))))
      (list (org-cite-list-bibliography-files)
            citations))))"##,
    );
}
