//! Strong uncovered-features-52 oracle tests — org-macro, org-entities, org-footnote deep.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-macro-replace-all with args
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_macro_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greeting Hello $1 $2!\n{{{greeting(World, Elisp)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
        expect_test::expect![[r#""ERR (error \"Undefined Org macro: greeting; aborting\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro-accumulate-arguments nested
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_macro_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(org-macro-accumulate-arguments "{{{macro(a, b(c), d)}}}" 0)"##,
        expect_test::expect![[r#""ERR (void-function org-macro-accumulate-arguments)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entity-get multiple
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_entity_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-entity-get "alpha")
        (org-entity-get "beta")
        (org-entity-get "gamma")
        (org-entity-get "Agrave")
        (org-entity-get "copy")
        (org-entity-get "nonexistent"))"##,
        expect_test::expect![[
            r#""OK ((\"alpha\" \"\\\\alpha\" t \"&alpha;\" \"alpha\" \"alpha\" \"α\") (\"beta\" \"\\\\beta\" t \"&beta;\" \"beta\" \"beta\" \"β\") (\"gamma\" \"\\\\gamma\" t \"&gamma;\" \"gamma\" \"gamma\" \"γ\") (\"Agrave\" \"\\\\`{A}\" nil \"&Agrave;\" \"A\" \"À\" \"À\") (\"copy\" \"\\\\textcopyright{}\" nil \"&copy;\" \"(c)\" \"©\" \"©\") nil)""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entity-get-utf-8 multiple
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_entity_utf_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-entity-get-utf-8 "alpha")
        (org-entity-get-utf-8 "beta")
        (org-entity-get-utf-8 "gamma")
        (org-entity-get-utf-8 "Agrave")
        (org-entity-get-utf-8 "copy"))"##,
        expect_test::expect![[r#""ERR (void-function org-entity-get-utf-8)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-new
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_new() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text")
  (goto-char (point-max))
  (condition-case nil
      (org-footnote-new)
    (error nil))
  (buffer-string))"##,
        expect_test::expect![[r#""OK \"Text[fn:1]\n\n* Footnotes\n\n[fn:1] \n\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-action at ref
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_action_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (goto-char (point-min))
  (search-forward "[fn:1]")
  (condition-case nil
      (org-footnote-action)
    (error nil))
  (point))"##,
        expect_test::expect![[r#""OK 19""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-action at def
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_action_def() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (goto-char (point-max))
  (search-backward "[fn:1]")
  (condition-case nil
      (org-footnote-action)
    (error nil))
  (point))"##,
        expect_test::expect![[r#""OK 5""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-goto-definition
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_goto() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (goto-char (point-min))
  (condition-case nil
      (org-footnote-goto-definition "1")
    (error nil))
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
        expect_test::expect![[r#""OK \"[fn:1] Def\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-delete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (goto-char (point-min))
  (condition-case nil
      (org-footnote-delete "1")
    (error nil))
  (buffer-string))"##,
        expect_test::expect![[r#""OK \"Text\n\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-normalize
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_normalize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (condition-case nil
      (org-footnote-normalize)
    (error nil))
  (buffer-string))"##,
        expect_test::expect![[r#""OK \"Text[fn:1]\n\n* Footnotes\n\n[fn:1] Def\n\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-at-reference-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_at_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""OK ((:ref nil) (:def nil))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-at-definition-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_at_def() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""OK ((:ref nil) (:def (\"1\" 13 23 \"Def\")))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-all-notes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] Def1\n[fn:2] Def2")
  (org-footnote-all-notes))"##,
        expect_test::expect![[r#""ERR (void-function org-footnote-all-notes)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-footnote-unique-label
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_footnote_unique() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Def")
  (org-footnote-unique-label))"##,
        expect_test::expect![[r#""OK \"2\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map footnote-reference
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_map_footnote_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] Def1\n[fn:2] Def2")
  (org-element-map (org-element-parse-buffer) 'footnote-reference
    (lambda (f) (org-element-property :label f))))"##,
        expect_test::expect![[r#""OK (\"1\" \"2\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map footnote-definition
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_map_footnote_def() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] Def1\n[fn:2] Def2")
  (org-element-map (org-element-parse-buffer) 'footnote-definition
    (lambda (f) (list (org-element-property :label f)
                      (org-trim (buffer-substring-no-properties
                                  (org-element-property :contents-begin f)
                                  (org-element-property :contents-end f)))))))"##,
        expect_test::expect![[r#""OK ((\"1\" \"Def1\") (\"2\" \"Def2\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map macro
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_map_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: name Hello\nText {{{name}}} end")
  (org-element-map (org-element-parse-buffer) 'macro
    (lambda (m) (list (org-element-property :key m)
                      (org-element-property :value m)
                      (org-element-property :args m)))))"##,
        expect_test::expect![[r#""OK ((\"name\" \"{{{name}}}\" nil))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map entity
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_map_entity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text \\alpha \\beta \\gamma")
  (org-element-map (org-element-parse-buffer) 'entity
    (lambda (e) (list (org-element-property :name e)
                      (org-element-property :utf-8 e)))))"##,
        expect_test::expect![[r#""OK ((\"alpha\" \"α\") (\"beta\" \"β\") (\"gamma\" \"γ\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map latex-fragment
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_map_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text $x^2$ and $$y=mx+b$$ and \\(z\\)")
  (org-element-map (org-element-parse-buffer) 'latex-fragment
    (lambda (l) (org-element-property :value l))))"##,
        expect_test::expect![[r#""OK (\"$x^2$\" \"$$y=mx+b$$\" \"\\\\(z\\\\)\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map statistics-cookie
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf52_map_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [1/2]\n- [X] a\n- [ ] b")
  (org-element-map (org-element-parse-buffer) 'statistics-cookie
    (lambda (s) (list (org-element-property :value s)
                      (org-element-property :begin s)))))"##,
        expect_test::expect![[r#""OK ((\"[1/2]\" 5))""#]],
    );
}
