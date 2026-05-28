//! Strong uncovered-features-26 oracle tests — org-export and publishing.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-html
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_html() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-html nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-latex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-latex nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-ascii
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-ascii nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-utf8
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-utf8 nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-html-to-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_html_buf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-html-to-buffer nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-latex-to-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_latex_buf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-latex-to-buffer nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-region-as-html
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (goto-char (point-min))
  (search-forward "Body")
  (beginning-of-line)
  (org-export-region-as-html (point) (point-max) nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-pdf
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_pdf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-export-as-pdf nil)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-pdf-and-open
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_pdf_open() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-export-as-pdf-and-open nil)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-dispatch
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_dispatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-export-dispatch nil)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-html-export-as-html
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_html_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-html-export-as-html)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-latex-export-as-latex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_latex_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-latex-export-as-latex)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ascii-export-as-ascii
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_ascii_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-ascii-export-as-ascii)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-publish "test" nil)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-publish-all nil)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish-current-file
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-publish-current-file nil)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish-current-project
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish_project() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-publish-current-project nil)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish-sitemap
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish_sitemap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-publish-sitemap "test")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-define-backend
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-export-define-backend 'test '((template . (lambda (contents info) contents))))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-get-environment
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_env() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+AUTHOR: Me\n* H1\nBody")
  (let ((env (org-export-get-environment nil)))
    (list (plist-get env :title)
          (plist-get env :author))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-get-contents
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody\n** H2\nSub")
  (let ((info (org-export-get-environment nil)))
    (org-export-get-contents (current-buffer) info)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-export-string-as "* H\nBody *bold*" 'html t)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as latex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_string_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-export-string-as "* H\nBody *bold*" 'latex t)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as ascii
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_string_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-export-string-as "* H\nBody *bold*" 'ascii t)"##);
}
