//! Strong uncovered-features-49 oracle tests — org-export string, org-link, org-protocol.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as html
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_export_html() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-export-string-as "* H\nBody *bold*" 'html t)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as latex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_export_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-export-string-as "* H\nBody *bold*" 'latex t)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as ascii
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_export_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-export-string-as "* H\nBody *bold*" 'ascii t)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as with options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_export_opts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-export-string-as "#+TITLE: T\n* H\nBody" 'html t)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-link-types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_link_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(sort (copy-sequence org-link-types) 'string<)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-link-protocols
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_link_protocols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(mapcar 'car org-link-protocols)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-link-escape-browser
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_link_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-link-escape-browser "http://example.com?a=1&b=2")
        (org-link-escape-browser "hello world")
        (org-link-escape-browser "test%20"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-link-unescape
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_link_unescape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-link-unescape "http://example.com?a=1%26b=2")
        (org-link-unescape "hello%20world")
        (org-link-unescape "test%2520"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-link-plain-re
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_link_plain_re() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(string-match-p org-link-plain-re "http://example.com")"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-bracket-link-regexp
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_link_bracket_re() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(string-match-p org-bracket-link-regexp "[[http://example.com][Example]]")"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-protocol-parse-parameters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_protocol_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(org-protocol-parse-parameters "org-protocol://store-link?url=http://example.com&title=Test")"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-protocol-sanitize-uri
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_protocol_sanitize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-protocol-sanitize-uri "http://example.com")
        (org-protocol-sanitize-uri "https://test.org/path?a=1&b=2")
        (org-protocol-sanitize-uri "file:///tmp/test.txt"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-protocol-check-protocol-for
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_protocol_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-protocol-check-protocol-for "store-link")"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-store-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_store_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-min))
  (org-store-link nil)
  (list (car org-stored-links)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-insert-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_insert_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (org-insert-link nil "http://example.com" "Example")
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-open-at-point on link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_open_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n[[http://example.com][Link]]")
  (search-forward "Link")
  (list (org-element-property :type (org-element-context))
        (org-element-property :path (org-element-context))
        (org-element-property :raw-link (org-element-context))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map links
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_map_links() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n[[http://a.com][A]] [[file:b.el][B]] [[id:xxx][C]] [[mailto:d@e.com]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l)
                      (org-element-property :path l)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map timestamps
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_map_timestamps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nSCHEDULED: <2026-01-15 Wed>\n* U\n<2026-01-20>--<2026-01-25>\n* V\n[2026-01-30]")
  (org-element-map (org-element-parse-buffer) 'timestamp
    (lambda (ts) (list (org-element-property :type ts)
                      (org-element-property :year-start ts)
                      (org-element-property :day-start ts)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map planning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_map_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\nCLOSED: [2026-01-10]")
  (org-element-map (org-element-parse-buffer) 'planning
    (lambda (p) (list (org-element-property :scheduled p)
                      (org-element-property :deadline p)
                      (org-element-property :closed p)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map clock
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_map_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:30] =>  1:30")
  (org-element-map (org-element-parse-buffer) 'clock
    (lambda (c) (list (org-element-property :status c)
                      (org-element-property :duration c)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map footnote
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf49_map_footnote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] Def1\n[fn:2] Def2")
  (list (org-element-map (org-element-parse-buffer) 'footnote-reference
          (lambda (f) (org-element-property :label f)))
        (org-element-map (org-element-parse-buffer) 'footnote-definition
          (lambda (f) (org-element-property :label f)))))"##,
    );
}
