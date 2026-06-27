//! Strict combo oracle probes, batch 47: org-mode / org-element parsing via
//! assert_oracle_parity_with_load. org is the largest commonly-used library;
//! org-element-parse-buffer is intricate. These attempt to load org/org.el
//! and org/org-element.el and parse a small document.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_i4_org_mode_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (org-mode)
  (list major-mode
        (boundp 'org-element--cache)
        (derived-mode-p 'org-mode)))
"##,
        &["org/org.el"],
    );
}

#[test]
fn div_i4_org_element_parse_headings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (insert "* Heading 1\n** Sub heading\nSome text.\n")
  (org-mode)
  (let ((parsed (org-element-parse-buffer)))
    (list (org-element-map parsed 'headline
            (lambda (h) (org-element-property :raw-value h)))
          (length (org-element-map parsed 'headline 'identity))
          (org-element-map parsed 'paragraph
            (lambda (p) (org-element-property :begin p)) nil t))))
"##,
        &["org/org.el", "org/org-element.el"],
    );
}

#[test]
fn div_i4_org_element_parse_link_and_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (insert "Text with [[https://example.com][a link]].\n- item one\n- item two\n")
  (org-mode)
  (let ((parsed (org-element-parse-buffer)))
    (list (org-element-map parsed 'link
            (lambda (l) (org-element-property :path l)))
          (length (org-element-map parsed 'item 'identity))
          (org-element-map parsed 'plain-list 'identity nil t))))
"##,
        &["org/org.el", "org/org-element.el"],
    );
}

#[test]
fn div_i4_org_element_property_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (insert "* Heading\n:PROPERTIES:\n:CUSTOM_ID: my-id\n:END:\nBody.\n")
  (org-mode)
  (let ((parsed (org-element-parse-buffer)))
    (org-element-map parsed 'headline
      (lambda (h) (org-element-property :CUSTOM_ID h)) nil t)))
"##,
        &["org/org.el", "org/org-element.el"],
    );
}
