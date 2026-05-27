//! Strong uncovered-features-21 oracle tests — complex multi-step workflows.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-cycle + org-element-parse after cycling
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_cycle_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b")
  (goto-char (point-min))
  (org-overview)
  (let ((r '()))
    (push (list :headlines (length (org-element-map (org-element-parse-buffer) 'headline 'identity))) r)
    (push (list :visible (buffer-substring-no-properties (point-min) (point-max))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// insert headline + set todo + set tags + set property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_full_build() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task1 :work:\n:PROPERTIES:\n:EFFORT: 2h\n:END:\n** DONE Sub1 :home:\n* WAITING Task2\nSCHEDULED: <2026-02-01>")
  (let ((r '()))
    (push (list :headlines (org-element-map (org-element-parse-buffer) 'headline
                              (lambda (h) (list (org-element-property :level h)
                                                (org-element-property :raw-value h)
                                                (org-element-property :todo-keyword h)
                                                (org-element-property :tags h))))) r)
    (push (list :planning (org-element-map (org-element-parse-buffer) 'planning
                            (lambda (p) (list (when (org-element-property :scheduled p) "sched")
                                              (when (org-element-property :deadline p) "dead"))))) r)
    (push (list :properties (org-element-map (org-element-parse-buffer) 'node-property
                              (lambda (p) (list (org-element-property :key p)
                                                (org-element-property :value p))))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc then reparse after modifications
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_modify_reparse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n* B\n* C")
  (let ((r '()))
    (push (list :init (org-element-map (org-element-parse-buffer) 'headline
                        (lambda (h) (org-element-property :raw-value h)))) r)
    (goto-char (point-min))
    (org-metadown)
    (push (list :after-move (org-element-map (org-element-parse-buffer) 'headline
                              (lambda (h) (org-element-property :raw-value h)))) r)
    (goto-char (point-max))
    (insert "\n* D")
    (push (list :after-insert (org-element-map (org-element-parse-buffer) 'headline
                                (lambda (h) (org-element-property :raw-value h)))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build list then indent/dedent multiple items
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_list_indent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C\n- D")
  (let ((r '()))
    (push (list :init (org-element-map (org-element-parse-buffer) 'item
                        (lambda (i) (list (org-element-property :level i)
                                          (org-trim (buffer-substring-no-properties
                                                      (org-element-property :contents-begin i)
                                                      (org-element-property :contents-end i))))))) r)
    (goto-char (point-min))
    (forward-line 1)
    (org-metaright)
    (forward-line 1)
    (org-metaright)
    (push (list :indented (org-element-map (org-element-parse-buffer) 'item
                            (lambda (i) (list (org-element-property :level i)
                                              (org-trim (buffer-substring-no-properties
                                                          (org-element-property :contents-begin i)
                                                          (org-element-property :contents-end i))))))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build table then add row/column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_table_add() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|---+---|\n| 1 | 2 |")
  (goto-char (point-max))
  (org-table-insert-row)
  (insert "3 | 4")
  (org-table-align)
  (let ((r '()))
    (push (list :rows (length (org-element-map (org-element-parse-buffer) 'table-row 'identity))) r)
    (push (list :cells (length (org-element-map (org-element-parse-buffer) 'table-cell 'identity))) r)
    (push (list :content (buffer-string)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build src block then execute
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_src_exec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-execute-src-block)
  (let ((r '()))
    (push (list :results (org-element-map (org-element-parse-buffer) 'fixed-width
                           (lambda (fw) (org-element-property :value fw)))) r)
    (push (list :content (buffer-string)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with footnotes then collect
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_footnotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2] end\n\n[fn:1] First def\n[fn:2] Second def")
  (let ((r '()))
    (push (list :refs (org-element-map (org-element-parse-buffer) 'footnote-reference
                        (lambda (f) (org-element-property :label f)))) r)
    (push (list :defs (org-element-map (org-element-parse-buffer) 'footnote-definition
                        (lambda (f) (list (org-element-property :label f)
                                          (org-trim (buffer-substring-no-properties
                                                      (org-element-property :contents-begin f)
                                                      (org-element-property :contents-end f))))))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with links then collect types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_links() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n[[http://a.com][A]] [[file:b.el][B]] [[id:xxx][C]] [[mailto:d@e.com]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l)
                      (org-element-property :path l)
                      (org-element-property :raw-link l)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with inline markup then collect all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nPara *bold* /italic/ _under_ +strike+ =code= ~verb~")
  (org-element-map (org-element-parse-buffer) '(bold italic underline strike-through code verbatim)
    (lambda (o) (list (org-element-type o)
                      (org-trim (buffer-substring-no-properties
                                  (org-element-property :contents-begin o)
                                  (org-element-property :contents-end o)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with entities then collect
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_entities() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text \\alpha \\beta \\gamma \\delta")
  (org-element-map (org-element-parse-buffer) 'entity
    (lambda (e) (list (org-element-property :name e)
                      (org-element-property :utf-8 e)
                      (org-element-property :latex e)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with latex fragments then collect
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text $x^2$ $$y=mx+b$$ and \\(z\\) \\[w\\]")
  (org-element-map (org-element-parse-buffer) 'latex-fragment
    (lambda (l) (org-element-property :value l))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with timestamps then collect properties
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_timestamps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nSCHEDULED: <2026-01-15 Wed>\n* U\nDEADLINE: <2026-01-20 Mon +1w>\n* V\n<2026-01-25>--<2026-01-30>")
  (org-element-map (org-element-parse-buffer) 'timestamp
    (lambda (ts) (list (org-element-property :type ts)
                      (org-element-property :year-start ts)
                      (org-element-property :month-start ts)
                      (org-element-property :day-start ts)
                      (org-element-property :repeater-type ts)
                      (org-element-property :repeater-value ts)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with clock entries then collect
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_clocks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:30] =>  1:30\nCLOCK: [2026-01-11 14:00]--[2026-01-11 15:00] =>  1:00")
  (org-element-map (org-element-parse-buffer) 'clock
    (lambda (c) (list (org-element-property :status c)
                      (org-element-property :duration c)
                      (org-element-property :value c)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build complex doc then get full element type distribution
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_distribution() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Complex\n#+FILETAGS: :t1:t2:\n* TODO [#A] H1 :work:\nSCHEDULED: <2026-01-15>\nBody *bold* /italic/\n** H2\n- [X] a\n- [ ] b\n| x | y |\n|---+---|\n| 1 | 2 |\n#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\n* DONE [#B] H2 :home:\n:PROPERTIES:\n:A: 1\n:END:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00")
  (let ((types (org-element-map (org-element-parse-buffer) 'element 'org-element-type)))
    (list (length types)
          (sort (delete-dups (copy-sequence types)) 'string<))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with all block types then collect
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_blocks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n#+END_SRC\n#+BEGIN_QUOTE\nQ\n#+END_QUOTE\n#+BEGIN_CENTER\nC\n#+END_CENTER\n#+BEGIN_EXPORT html\n<b>Bold</b>\n#+END_EXPORT\n#+BEGIN_VERSE\nV\n#+END_VERSE")
  (org-element-map (org-element-parse-buffer) '(src-block quote-block center-block export-block verse-block)
    (lambda (b) (org-element-type b))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with all planning types then collect
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\nSCHEDULED: <2026-01-15>\n* B\nDEADLINE: <2026-01-20>\n* C\nCLOSED: [2026-01-10]\n* D\nSCHEDULED: <2026-01-15> DEADLINE: <2026-01-20>")
  (org-element-map (org-element-parse-buffer) 'planning
    (lambda (p) (list (when (org-element-property :scheduled p) "S")
                      (when (org-element-property :deadline p) "D")
                      (when (org-element-property :closed p) "C")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc with all keyword types then collect
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+AUTHOR: A\n#+DATE: D\n#+OPTIONS: o\n#+FILETAGS: :t:\n#+STARTUP: overview\n#+CATEGORY: c")
  (org-element-map (org-element-parse-buffer) 'keyword
    (lambda (k) (list (org-element-property :key k)
                      (org-element-property :value k)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc then org-element-map with-multiple-type filter
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_multi_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nPara *bold* /italic/ [[http://a][Link]] $x^2$")
  (sort (delete-dups (org-element-map (org-element-parse-buffer) '(bold italic link latex-fragment)
                        'org-element-type))
        'string<))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc then get parent chain for deeply nested object
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_parent_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nPara *bold* text")
  (search-forward "bold")
  (let* ((obj (org-element-context))
         (chain '()))
    (let ((p obj))
      (while p
        (push (org-element-type p) chain)
        (setq p (org-element-property :parent p))))
    (nreverse chain)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// build doc then get lineage with types filter
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf21_lineage() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nPara *bold* text")
  (search-forward "bold")
  (let* ((obj (org-element-context))
         (lineage (org-element-lineage obj '(headline paragraph bold) t)))
    (mapcar 'org-element-type lineage)))"##,
    );
}
