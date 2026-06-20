//! combo_strict_29.rs + strong 93/94 — exhaustive surface probes
use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};
#[test]
fn strict_org_babel_execute_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ob-core)
 (list :es-fbound (fboundp 'org-babel-execute-subtree) :eb-fbound (fboundp 'org-babel-execute-buffer)))"##,
    );
}
#[test]
fn strict_org_export_backend_name_from_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ox) (list
 :ascii-name (org-export-backend-name (assq 'ascii org-export-registered-backends))
 :html-name (org-export-backend-name (assq 'html org-export-registered-backends))
 :latex-name (org-export-backend-name (assq 'latex org-export-registered-backends))))"##,
    );
}
#[test]
fn strict_org_element_parent_of_root() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org) (require 'org-element)
 (with-temp-buffer (org-mode) (insert "* H\n") (let* ((t (org-element-parse-buffer))
  (h (car (org-element-map t 'headline #'identity))))
  (list :h-level (org-element-property :level h) :h-parent (org-element-type (org-element-property :parent h))
   :root-parent (org-element-property :parent t)))))"##,
    );
}
#[test]
fn strict_org_entity_unicode_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-entities) (with-temp-buffer (org-mode)
 (insert "\\alpha \\beta \\gamma\n") (goto-char (point-min))
 (let ((t (org-element-parse-buffer))) (list
  :ent-count (length (org-element-map t 'entity #'identity))
  :ent-names (mapcar (lambda (e) (org-element-property :name e)) (org-element-map t 'entity #'identity))))))"##,
    );
}
#[test]
fn strict_org_babel_hide_result() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ob-core) (list
 :hide-result-fbound (fboundp 'org-babel-hide-result-toggle) :remove-result-fbound (fboundp 'org-babel-remove-result)))"##,
    );
}
#[test]
fn strict_org_cycle_local_visibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "* A\n** B\nBody.\n* C\n") (let ((r '())) (goto-char (point-min))
  (search-forward "* C") (beginning-of-line)
  (condition-case nil (org-cycle) (error nil))
  (push (list :after-cycle-C-invis (get-char-property (point) 'invisible)) r) (nreverse r)))"##,
    );
}
#[test]
fn strict_org_prepare_search_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-agenda)
 (list :psb-fbound (fboundp 'org-prepare-agenda-buffers) :psb-day-fbound (fboundp 'org-prepare-agenda-buffers)
  :buf-list (when (fboundp 'org-agenda-files) (fboundp 'org-agenda-files))))"##,
    );
}
#[test]
fn strict_org_macro_case_sensitivity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "#+MACRO: test MyTest\n{{{test}}} and {{{Test}}} and {{{TEST}}}\n")
 (let ((r '())) (let* ((t (org-element-parse-buffer)) (i (substring-no-properties (org-element-interpret-data t))))
  (push (list :has-MyTest (string-match-p "MyTest" i)) r)
  (push (list :still-has-Test (string-match-p "{{{Test}}}" i)) r)
  (push (list :still-has-TEST (string-match-p "{{{TEST}}}" i)) r)) (nreverse r)))"##,
    );
}
#[test]
fn strict_org_table_relative_row_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "| a |  b |\n| 1 |  2 |\n| 3 |  4 |\n|   |    |\n")
 (insert "#+TBLFM: @>$1=vsum(@2..@-1)::$2=$1+0\n")
 (let ((r '())) (goto-char (point-min)) (org-table-recalculate t) (org-table-align)
  (push (list :to-lisp (org-table-to-lisp)) r) (nreverse r)))"##,
    );
}
#[test]
fn strict_org_cite_style_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'oc) (list
 :styles (when (boundp 'org-cite-supported-styles) org-cite-supported-styles)
 :default-style (when (boundp 'org-cite-default-style) org-cite-default-style)
 :export-bibliography (when (boundp 'org-cite-export-bibliography) org-cite-export-bibliography)))"##,
    );
}
