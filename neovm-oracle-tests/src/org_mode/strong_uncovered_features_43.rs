//! Strong uncovered-features-43 oracle tests — org-macro, org-entities, org-footnote.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-macro-replace-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_macro_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greeting Hello $1!\n{{{greeting(World)}}} and {{{greeting(Elisp)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro-accumulate-arguments
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_macro_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-macro-accumulate-arguments "{{{macro(a,b,c)}}}" 0)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro-expand-macro
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_macro_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greeting Hello $1!\n{{{greeting(World)}}}")
  (let ((org-macro-templates (org-macro--collect-macros)))
    (org-macro-expand-macro "{{{greeting(World)}}}" org-macro-templates)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro--collect-macros
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_macro_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: a 1\n#+MACRO: b 2\n{{{a}}} {{{b}}}")
  (org-macro--collect-macros))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entity-get
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_entity_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-entity-get "alpha")
        (org-entity-get "beta")
        (org-entity-get "gamma")
        (org-entity-get "nonexistent"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entity-get-utf-8
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_entity_utf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-entity-get-utf-8 "alpha")
        (org-entity-get-utf-8 "beta")
        (org-entity-get-utf-8 "gamma"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entity-get-latex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_entity_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-entity-get-latex "alpha")
        (org-entity-get-latex "beta")
        (org-entity-get-latex "gamma"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entity-get-html
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_entity_html() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-entity-get-html "alpha")
        (org-entity-get-html "beta")
        (org-entity-get-html "gamma"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entity-get-ascii
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_entity_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-entity-get-ascii "alpha")
        (org-entity-get-ascii "beta")
        (org-entity-get-ascii "gamma"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-new
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_new() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text")
  (goto-char (point-max))
  (condition-case nil
      (org-footnote-new)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-action
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_action() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (goto-char (point-min))
  (search-forward "[fn:1]")
  (goto-char (match-beginning 0))
  (condition-case nil
      (org-footnote-action)
    (error nil))
  (list (point)
        (buffer-substring-no-properties
         (line-beginning-position)
         (line-end-position))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-goto-definition
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_goto() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (goto-char (point-min))
  (condition-case nil
      (org-footnote-goto-definition "1")
    (error nil))
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-goto-previous-reference
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_prev() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:1]\n\n[fn:1] Def")
  (goto-char (point-max))
  (condition-case nil
      (org-footnote-goto-previous-reference "1")
    (error nil))
  (point))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-delete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (goto-char (point-min))
  (condition-case nil
      (org-footnote-delete "1")
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-renumber-fn:A
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_renumber() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:a] more[fn:b]\n\n[fn:a] DefA\n[fn:b] DefB")
  (condition-case nil
      (org-footnote-renumber-fn:A)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-normalize
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_normalize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (condition-case nil
      (org-footnote-normalize)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-all-notes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] Def1\n[fn:2] Def2")
  (org-footnote-all-notes))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-at-reference-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_at_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (let ((r '()))
    (goto-char (point-min))
    (search-forward "[fn:1]")
    (push (list :ref (org-footnote-at-reference-p)) r)
    (search-forward "[fn:1]")
    (push (list :def (org-footnote-at-reference-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-at-definition-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_at_def() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (let ((r '()))
    (goto-char (point-min))
    (search-forward "[fn:1]")
    (push (list :ref (org-footnote-at-definition-p)) r)
    (search-forward "[fn:1]")
    (push (list :def (org-footnote-at-definition-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-unique-label
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_unique() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (org-footnote-unique-label))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-insert-definition
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf43_footnote_insert_def() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text")
  (goto-char (point-max))
  (condition-case nil
      (org-footnote-insert-definition "test" "Test definition")
    (error nil))
  (buffer-string))"##,
    );
}
