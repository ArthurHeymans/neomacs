//! Strict combo oracle probes, batch 78: org export to LaTeX (ox-latex) and
//! Markdown (ox-md) — each backend has distinct conversion logic.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_p2_org_export_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(replace-regexp-in-string "\\borg[0-9a-f]\\{6,\\}\\b" "orgID"
  (with-temp-buffer
    (org-mode)
    (insert "* Heading\nText with $E=mc^2$ and [[https://example.com][link]].\n")
    (org-export-as 'latex)))
"##,
        &["org/org.el", "org/ox.el", "org/ox-latex.el"],
    );
}

#[test]
fn div_p2_org_export_markdown() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (org-mode)
  (insert "* Heading\nText with [[https://example.com][link]].\n- item one\n- item two\n")
  (org-export-as 'md))
"##,
        &["org/org.el", "org/ox.el", "org/ox-md.el"],
    );
}

#[test]
fn div_p2_org_export_latex_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(replace-regexp-in-string "\\borg[0-9a-f]\\{6,\\}\\b" "orgID"
  (with-temp-buffer
    (org-mode)
    (insert "* Table\n| A | B |\n| 1 | 2 |\n| 3 | 4 |\n")
    (org-export-as 'latex)))
"##,
        &["org/org.el", "org/ox.el", "org/ox-latex.el"],
    );
}
