use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_footnote_renumber_delete_sort_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Notes\n")
    (insert "B ref[fn:7], A ref[fn:3], B again[fn:7].\n\n")
    (insert "[fn:7] Bee definition\n")
    (insert "[fn:3] Aye definition\n")
    (insert "[fn:9] Unused definition\n")
    (org-footnote-renumber-fn:N)
    (let ((after-renumber
           (buffer-substring-no-properties (point-min) (point-max))))
      (org-footnote-sort)
      (let ((after-sort
             (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "[fn:1]")
        (let ((deleted (org-footnote-delete)))
          (list after-renumber
                after-sort
                deleted
                (org-footnote-all-labels)
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_footnote_inline_normalize_section_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Alpha\n")
    (insert "Text [fn::Inline *bold* note] and named [fn:name].\n")
    (insert "* Footnotes\n")
    (insert "[fn:name] Named note\n")
    (let ((org-footnote-section "Footnotes")
          (org-footnote-fill-after-inline-note-extraction nil))
      (org-footnote-normalize)
      (list (org-footnote-all-labels)
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_footnote_reference_definition_navigation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* One\n")
    (insert "First [fn:a] and second [fn:b].\n")
    (insert "* Footnotes\n")
    (insert "[fn:b] Bee\n")
    (insert "[fn:a] Aye\n")
    (goto-char (point-min))
    (search-forward "[fn:a]")
    (let ((ref-a (org-footnote-at-reference-p)))
      (org-footnote-goto-definition "a")
      (let ((def-a (list (line-number-at-pos)
                         (org-footnote-at-definition-p))))
        (org-footnote-goto-previous-reference "a")
        (let ((back-a (list (line-number-at-pos)
                            (org-footnote-at-reference-p))))
          (goto-char (point-max))
          (let ((pos (org-footnote-create-definition "c")))
            (list ref-a
                  def-a
                  back-a
                  pos
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}
