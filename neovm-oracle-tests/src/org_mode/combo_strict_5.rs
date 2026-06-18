//! Org export/structure parity: HTML/LaTeX/ASCII body export (no headlines,
//! so anchors stay deterministic), element table-cell interpret, fill, and
//! org-table-to-lisp.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_ascii_list_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'ox-ascii)
  (with-temp-buffer (org-mode)
    (insert "1. first\n2. second\n\n| x | y |\n|---+---|\n| 9 | 8 |\n")
    (let ((org-ascii-text-width 72)) (org-export-as 'ascii nil nil t))))"##,
    );
}

#[test]
fn org_element_table_cells() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "| a | b | c |\n| 1 | 2 | 3 |\n")
    (org-element-map (org-element-parse-buffer) 'table-cell
      (lambda (c) (org-element-interpret-data (org-element-contents c))))))"##,
    );
}

#[test]
fn org_fill_paragraph() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (setq fill-column 20)
    (insert "aaaa bbbb cccc dddd eeee ffff gggg hhhh")
    (fill-paragraph) (buffer-string)))"##,
    );
}

#[test]
fn org_html_inline_markup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'ox-html)
  (with-temp-buffer (org-mode)
    (insert "Some *bold*, /italic/, =code=, ~verbatim~, +strike+ and a [[https://x.org][link]].\n")
    (org-export-as 'html nil nil t)))"##,
    );
}

#[test]
fn org_html_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'ox-html)
  (with-temp-buffer (org-mode)
    (insert "- one\n- two\n  - nested\n- three\n")
    (org-export-as 'html nil nil t)))"##,
    );
}

#[test]
fn org_html_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'ox-html)
  (with-temp-buffer (org-mode)
    (insert "| a | b |\n|---+---|\n| 1 | 2 |\n")
    (org-export-as 'html nil nil t)))"##,
    );
}

#[test]
fn org_latex_markup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'ox-latex)
  (with-temp-buffer (org-mode)
    (insert "Text with *bold* and /italic/ and $x^2$ math.\n")
    (org-export-as 'latex nil nil t)))"##,
    );
}

#[test]
fn org_table_to_lisp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "| 1 | 2 |\n|---+---|\n| 3 | 4 |\n")
    (goto-char (point-min)) (org-table-to-lisp)))"##,
    );
}
