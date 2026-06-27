//! Strict combo oracle probes, batch 76: org-export (ox-ascii — export an org
//! document to plain text) and mml-parse (MIME multipart compose). These are
//! the heaviest commonly-used libraries remaining untested.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_p0_org_export_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (org-mode)
  (insert "* Heading 1\n** Sub heading\nParagraph text here.\n- item one\n- item two\n")
  (org-export-as 'ascii))
"##,
        &["org/org.el", "org/ox.el", "org/ox-ascii.el"],
    );
}

#[test]
fn div_p0_org_export_ascii_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (org-mode)
  (insert "* Table test\n| Name | Value |\n|---+---|\n| a | 1 |\n| b | 2 |\n")
  (org-export-as 'ascii))
"##,
        &["org/org.el", "org/ox.el", "org/ox-ascii.el"],
    );
}

#[test]
fn div_p0_mml_parse_multipart() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (insert "<#part type=\"text/plain\" disposition=\"inline\">\nHello body\n<#/part>\n")
  (mml-parse))
"##,
        &["gnus/mml.el"],
    );
}
