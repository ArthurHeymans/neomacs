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

#[test]
fn org_cite_basic_bibtex_json_parse_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (require 'oc-basic)
  (require 'ox-org)
  (let* ((root (make-temp-file "org-cite-basic" t))
         (bib (expand-file-name "refs.bib" root))
         (json (expand-file-name "refs.json" root))
         (org-cite-global-bibliography nil)
         (org-cite-export-processors '((t basic))))
    (unwind-protect
        (progn
          (with-temp-file bib
            (insert "@string{j = {Journal One}}\n")
            (insert "@article{smith2020,\n")
            (insert "  author = {Smith, Ada and Roe, Bob},\n")
            (insert "  title = {Alpha Study},\n")
            (insert "  journal = j,\n")
            (insert "  year = {2020}}\n")
            (insert "@book{doe2019,\n")
            (insert "  editor = {Doe, Dana},\n")
            (insert "  title = {Beta Book},\n")
            (insert "  publisher = {Press},\n")
            (insert "  year = {2019}}\n"))
          (with-temp-file json
            (insert "[{\"id\":\"json2021\",")
            (insert "\"author\":[{\"family\":\"Young\",\"given\":\"Yara\"}],")
            (insert "\"title\":\"Gamma JSON\",")
            (insert "\"issued\":{\"date-parts\":[[2021]]},")
            (insert "\"publisher\":\"JSON Press\"}]"))
          (with-temp-buffer
            (org-mode)
            (insert "#+cite_export: basic author-year numeric\n")
            (insert "#+bibliography: " bib " " json "\n")
            (insert "Lead [cite:@smith2020; @json2021 p. 7] and ")
            (insert "[cite/author:@doe2019].\n")
            (insert "#+print_bibliography:\n")
            (let* ((info (org-export-get-environment 'org nil))
                   (keys (org-cite-list-keys info))
                   (numbers (mapcar (lambda (key)
                                      (list key (org-cite-basic--key-number
                                                 key info)))
                                    keys))
                   (parsed (mapcar
                            (lambda (key)
                              (list key
                                    (org-cite-basic--get-author key info 'raw)
                                    (org-cite-basic--get-year key info 'no-suffix)
                                    (org-cite-basic--get-field
                                     'title key info 'raw)))
                            (sort (copy-sequence keys) #'string<)))
                   (out (org-export-as 'org nil nil t nil)))
              (list (org-cite-list-bibliography-files)
                    keys
                    numbers
                    parsed
                    out)))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_cite_basic_note_numeric_bibliography_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (require 'oc-basic)
  (require 'ox-ascii)
  (let* ((root (make-temp-file "org-cite-export" t))
         (bib (expand-file-name "refs.bib" root))
         (org-cite-global-bibliography nil)
         (org-cite-export-processors '((ascii basic))))
    (unwind-protect
        (progn
          (with-temp-file bib
            (insert "@article{alpha,\n")
            (insert "  author = {Alpha, Ann},\n")
            (insert "  title = {First Paper},\n")
            (insert "  journal = {J},\n")
            (insert "  year = {2020}}\n")
            (insert "@article{beta,\n")
            (insert "  author = {Beta, Ben},\n")
            (insert "  title = {Second Paper},\n")
            (insert "  journal = {J},\n")
            (insert "  year = {2021}}\n")
            (insert "@article{gamma,\n")
            (insert "  author = {Gamma, Gail},\n")
            (insert "  title = {Third Paper},\n")
            (insert "  journal = {J},\n")
            (insert "  year = {2022}}\n"))
          (with-temp-buffer
            (org-mode)
            (insert "#+cite_export: basic numeric text/bare\n")
            (insert "#+bibliography: " bib "\n")
            (insert "* Cites\n")
            (insert "Text [cite:@beta; @alpha] sentence.\n")
            (insert "Note ending.[cite/note:@gamma] next.\n")
            (insert "[cite/n:@alpha] keeps key only for bibliography.\n")
            (insert "#+print_bibliography: :style numeric\n")
            (let* ((org-export-with-toc nil)
                   (org-ascii-text-width 80)
                   (org-ascii-charset 'utf-8)
                   (info (org-export-get-environment 'ascii nil))
                   (citations
                    (mapcar
                     (lambda (cite)
                       (list (org-element-property :style cite)
                             (org-cite-get-references cite t)
                             (org-cite-inside-footnote-p cite)
                             (org-cite-citation-style cite info)))
                     (org-cite-list-citations info)))
                   (keys (org-cite-list-keys info))
                   (output (org-export-as 'ascii nil nil t nil)))
              (list keys citations output)))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_cite_delete_reference_and_citation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (with-temp-buffer
    (org-mode)
    (insert "Before [cite:see @one p. 1; compare @two; @three] after.\n")
    (insert "Solo [cite:@solo] done.\n")
    (goto-char (point-min))
    (search-forward "@two")
    (let* ((ref (org-element-context))
           (cite (org-element-lineage ref '(citation))))
      (org-cite-delete-reference ref)
      (goto-char (point-min))
      (search-forward "@solo")
      (org-cite-delete-citation (org-element-lineage
                                 (org-element-context)
                                 '(citation)))
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (remaining
              (org-element-map tree 'citation
                (lambda (citation)
                  (list (org-element-property :style citation)
                        (org-cite-get-references citation t)
                        (org-cite-main-affixes citation)))))
             (objects
              (org-element-map tree '(citation citation-reference)
                (lambda (obj)
                  (list (org-element-type obj)
                        (org-element-property :key obj)
                        (org-element-property :prefix obj)
                        (org-element-property :suffix obj))))))
        (list remaining
              objects
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}
