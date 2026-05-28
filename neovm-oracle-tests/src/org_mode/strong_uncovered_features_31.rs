//! Strong uncovered-features-31 oracle tests — org-bibtex, org-cite, org-ref.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-bibtex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_bibtex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:TYPE: article\n:TITLE: Test Title\n:AUTHOR: John Doe\n:YEAR: 2026\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-bibtex)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-bibtex-create
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_bibtex_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (condition-case nil
      (org-bibtex-create)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-bibtex-check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_bibtex_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:TYPE: article\n:TITLE: Test Title\n:AUTHOR: John Doe\n:YEAR: 2026\n:END:")
  (condition-case nil
      (org-bibtex-check)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-bibtex-headline
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_bibtex_headline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:TYPE: article\n:TITLE: Test Title\n:AUTHOR: John Doe\n:YEAR: 2026\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-bibtex-headline)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-bibtex-export-to-kill-ring
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_bibtex_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:TYPE: article\n:TITLE: Test Title\n:AUTHOR: John Doe\n:YEAR: 2026\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-bibtex-export-to-kill-ring)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cite-basic--complete-style
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_cite_style() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-cite-basic--complete-style)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cite-basic--complete-key
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_cite_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-cite-basic--complete-key)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cite-basic--print-reference
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_cite_print() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-cite-basic--print-reference "test-key")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cite-basic--get-entry
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_cite_entry() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-cite-basic--get-entry "test-key")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cite-basic--all-keys
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_cite_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-cite-basic--all-keys)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ref)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-cite-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_cite() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text cite:key1,key2 end")
  (condition-case nil
      (org-ref-cite-link)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-ref-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text ref:label end")
  (condition-case nil
      (org-ref-ref-link)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-bibliography-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_bib() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "bibliography:refs.bib")
  (condition-case nil
      (org-ref-bibliography-link)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-bibliography*
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_bib2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ref-bibliography* "refs.bib")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-format-cite
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ref-format-cite '("key1" "key2"))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-get-bibtex-key-under-cursor
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "cite:key1")
  (goto-char (point-min))
  (condition-case nil
      (org-ref-get-bibtex-key-under-cursor)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-find-bibliography
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "bibliography:refs.bib\n\nbibliography:more.bib")
  (condition-case nil
      (org-ref-find-bibliography)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-valid-keys
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_valid() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-ref-valid-keys)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-cite-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_cite_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "cite:key1 and ref:label")
  (goto-char (point-min))
  (list (condition-case nil (org-ref-cite-p) (error nil))
        (progn (search-forward "ref:") (condition-case nil (org-ref-cite-p) (error nil)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-ref-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_ref_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "cite:key1 and ref:label")
  (goto-char (point-min))
  (list (condition-case nil (org-ref-ref-p) (error nil))
        (progn (search-forward "ref:") (condition-case nil (org-ref-ref-p) (error nil)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ref-bibliography-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf31_ref_bib_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "bibliography:refs.bib")
  (goto-char (point-min))
  (condition-case nil
      (org-ref-bibliography-p)
    (error nil)))"##,
    );
}
