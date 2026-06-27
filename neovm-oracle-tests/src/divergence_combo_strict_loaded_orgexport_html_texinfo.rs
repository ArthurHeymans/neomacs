//! Strict combo oracle probes, batch 77: org export to HTML (ox-html —
//! different logic from ASCII) and texinfo-format-buffer (convert texinfo
//! source to formatted text).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_p1_org_export_html() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (org-mode)
  (insert "* Heading\nText with [[https://example.com][a link]].\n")
  (org-export-as 'html))
"##,
        &["org/org.el", "org/ox.el", "org/ox-html.el"],
    );
}

#[test]
fn div_p1_org_export_html_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (org-mode)
  (insert "* Data\n| A | B |\n| 1 | 2 |\n")
  (org-export-as 'html))
"##,
        &["org/org.el", "org/ox.el", "org/ox-html.el"],
    );
}

#[test]
fn div_p1_texinfo_format_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (insert "@node Top\n@chapter Test Chapter\nThis is some text.\n@itemize\n@item First\n@item Second\n@end itemize\n")
  (texinfo-format-buffer)
  (buffer-string))
"##,
        &["textmodes/texinfo.el"],
    );
}
