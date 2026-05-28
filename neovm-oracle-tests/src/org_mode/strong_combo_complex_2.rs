//! Strong combo-complex-2 oracle tests — deep multi-step workflows.
//!
//! Every test chains multiple operations capturing deep mutable state to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Build doc with nested headings → move/indent → parent chain verify
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_nested_move() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n** B\n*** C\n**** D\n* E")
  (let ((r '()))
    ;; initial parent chain for D
    (goto-char (point-min))
    (search-forward "D")
    (beginning-of-line)
    (let* ((obj (org-element-at-point))
           (chain '()))
      (let ((p obj))
        (while p
          (push (org-element-property :raw-value p) chain)
          (setq p (org-element-property :parent p))))
      (push (list :init-chain (nreverse chain)) r))
    ;; move D up one level (indent left)
    (org-metaleft)
    (let* ((obj (org-element-at-point))
           (chain '()))
      (let ((p obj))
        (while p
          (push (org-element-property :raw-value p) chain)
          (setq p (org-element-property :parent p))))
      (push (list :after-left (nreverse chain)) r))
    ;; move D back right
    (org-metaright)
    (let* ((obj (org-element-at-point))
           (chain '()))
      (let ((p obj))
        (while p
          (push (org-element-property :raw-value p) chain)
          (setq p (org-element-property :parent p))))
      (push (list :after-right (nreverse chain)) r))
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → cycle global → cycle local → verify visibility states
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_visibility_states() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody\n* H1b\n** H2b\nSub")
  (let ((r '()))
    ;; overview
    (org-overview)
    (push (list :overview (buffer-substring-no-properties (point-min) (point-max))) r)
    ;; global cycle 1
    (org-global-cycle nil)
    (push (list :global1 (buffer-substring-no-properties (point-min) (point-max))) r)
    ;; global cycle 2
    (org-global-cycle nil)
    (push (list :global2 (buffer-substring-no-properties (point-min) (point-max))) r)
    ;; global cycle 3
    (org-global-cycle nil)
    (push (list :global3 (buffer-substring-no-properties (point-min) (point-max))) r)
    ;; local cycle on H1
    (goto-char (point-min))
    (org-cycle 'children)
    (push (list :local-children (buffer-substring-no-properties (point-min) (point-max))) r)
    ;; local cycle subtree
    (org-cycle 'subtree)
    (push (list :local-subtree (buffer-substring-no-properties (point-min) (point-max))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc with all object types → parse → verify all present
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_all_objects() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nPara *bold* /italic/ _under_ +strike+ =code= ~verb~ [[http://a][Link]] $x^2$ \\alpha H_2O E=mc^2")
  (let ((r '()))
    ;; collect all object types
    (push (list :types (sort (delete-dups (org-element-map (org-element-parse-buffer) 'object 'org-element-type)) 'string<)) r)
    ;; collect all objects with content
    (push (list :objects (org-element-map (org-element-parse-buffer) '(bold italic underline strike-through code verbatim link latex-fragment entity subscript superscript)
                           (lambda (o) (list (org-element-type o)
                                             (org-trim (buffer-substring-no-properties
                                                         (org-element-property :contents-begin o)
                                                         (org-element-property :contents-end o))))))) r)
    ;; parent chain for bold
    (goto-char (point-min))
    (search-forward "bold")
    (let* ((obj (org-element-context))
           (chain '()))
      (let ((p obj))
        (while p
          (push (org-element-type p) chain)
          (setq p (org-element-property :parent p))))
      (push (list :bold-chain (nreverse chain)) r))
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → export html/latex/ascii → compare structure
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_export_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((src "* H\nBody *bold* /italic/"))
  (let ((html (org-export-string-as src 'html t))
        (latex (org-export-string-as src 'latex t))
        (ascii (org-export-string-as src 'ascii t)))
    (list (list :html-has-h2 (string-match-p "<h2>" html))
          (list :html-has-bold (string-match-p "<b>bold</b>" html))
          (list :html-has-italic (string-match-p "<i>italic</i>" html))
          (list :latex-has-section (string-match-p "\\\\section" latex))
          (list :latex-has-textbf (string-match-p "\\\\textbf" latex))
          (list :latex-has-textit (string-match-p "\\\\textit" latex))
          (list :ascii-has-h (string-match-p "H" ascii))
          (list :ascii-has-bold (string-match-p "bold" ascii)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → complex property operations → verify state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_props_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (let ((r '()))
    (goto-char (point-min))
    ;; put multiple properties
    (org-entry-put nil "A" "1")
    (org-entry-put nil "B" "2")
    (org-entry-put nil "C" "3")
    (push (list :after-put (list (org-entry-get nil "A") (org-entry-get nil "B") (org-entry-get nil "C"))) r)
    ;; update B
    (org-entry-put nil "B" "22")
    (push (list :after-update (list (org-entry-get nil "A") (org-entry-get nil "B") (org-entry-get nil "C"))) r)
    ;; delete A
    (org-entry-delete nil "A")
    (push (list :after-delete (list (org-entry-get nil "A") (org-entry-get nil "B") (org-entry-get nil "C"))) r)
    ;; multivalued property
    (org-entry-put-multivalued-property nil "D" "v1" "v2" "v3")
    (push (list :multi (org-entry-get-multivalued-property nil "D")) r)
    ;; verify buffer
    (push (list :content (buffer-string)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → complex tag operations → verify state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_tags_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T :existing:")
  (let ((r '()))
    (goto-char (point-min))
    ;; initial tags
    (push (list :init (org-get-tags)) r)
    ;; add tag
    (org-toggle-tag "new")
    (push (list :after-add (org-get-tags)) r)
    ;; remove existing
    (org-toggle-tag "existing")
    (push (list :after-remove (org-get-tags)) r)
    ;; set tags directly
    (org-set-tags '("a" "b" "c"))
    (push (list :after-set (org-get-tags)) r)
    ;; toggle a
    (org-toggle-tag "a")
    (push (list :after-toggle (org-get-tags)) r)
    ;; verify buffer
    (push (list :content (buffer-string)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → complex todo operations → verify state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_todo_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (let ((r '()))
    (goto-char (point-min))
    ;; initial
    (push (list :init (org-get-todo-state)) r)
    ;; cycle 6 times
    (dotimes (_ 6)
      (org-todo)
      (push (list :cycle (org-get-todo-state)) r))
    ;; verify buffer
    (push (list :content (buffer-string)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → complex priority operations → verify state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_priority_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* [#B] T")
  (let ((r '()))
    (goto-char (point-min))
    ;; initial
    (push (list :init (org-element-property :priority (org-element-at-point))) r)
    ;; priority up 3 times
    (dotimes (_ 3)
      (org-priority-up)
      (push (list :up (org-element-property :priority (org-element-at-point))) r))
    ;; priority down 3 times
    (dotimes (_ 3)
      (org-priority-down)
      (push (list :down (org-element-property :priority (org-element-at-point))) r))
    ;; verify buffer
    (push (list :content (buffer-string)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → complex list operations → verify state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_list_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C\n- D")
  (let ((r '()))
    ;; initial
    (push (list :init (org-element-map (org-element-parse-buffer) 'item
                        (lambda (i) (org-trim (buffer-substring-no-properties
                                                (org-element-property :contents-begin i)
                                                (org-element-property :contents-end i)))))) r)
    ;; indent B under A
    (goto-char (point-min))
    (forward-line 1)
    (org-metaright)
    (push (list :after-indent (org-element-map (org-element-parse-buffer) 'item
                                (lambda (i) (list (org-element-property :level i)
                                                  (org-trim (buffer-substring-no-properties
                                                              (org-element-property :contents-begin i)
                                                              (org-element-property :contents-end i))))))) r)
    ;; move C up
    (goto-char (point-min))
    (forward-line 2)
    (org-metaup)
    (push (list :after-move (org-element-map (org-element-parse-buffer) 'item
                              (lambda (i) (list (org-element-property :level i)
                                                (org-trim (buffer-substring-no-properties
                                                            (org-element-property :contents-begin i)
                                                            (org-element-property :contents-end i))))))) r)
    ;; dedent B
    (goto-char (point-min))
    (search-forward "B")
    (beginning-of-line)
    (org-metaleft)
    (push (list :after-dedent (org-element-map (org-element-parse-buffer) 'item
                                (lambda (i) (list (org-element-property :level i)
                                                  (org-trim (buffer-substring-no-properties
                                                              (org-element-property :contents-begin i)
                                                              (org-element-property :contents-end i))))))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → complex table operations → verify state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_table_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |")
  (let ((r '()))
    ;; initial
    (push (list :init (buffer-string)) r)
    (push (list :rows (length (org-element-map (org-element-parse-buffer) 'table-row 'identity))) r)
    ;; add row
    (goto-char (point-max))
    (org-table-insert-row)
    (insert "5 | 6")
    (org-table-align)
    (push (list :after-row (buffer-string)) r)
    ;; add column
    (org-table-insert-column)
    (push (list :after-col (buffer-string)) r)
    ;; sort
    (org-table-sort-lines nil ?a)
    (push (list :after-sort (buffer-string)) r)
    ;; transpose
    (goto-char (point-min))
    (org-table-transpose-table-at-point)
    (push (list :after-transpose (buffer-string)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → complex src operations → verify state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_src_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1)\n(+ 2)\n(+ 3)\n#+END_SRC")
  (let ((r '()))
    ;; initial
    (push (list :init (buffer-string)) r)
    ;; demarcate block
    (goto-char (point-min))
    (search-forward "(+ 2)")
    (beginning-of-line)
    (org-babel-demarcate-block)
    (push (list :after-demarcate (buffer-string)) r)
    ;; verify src blocks
    (push (list :blocks (org-element-map (org-element-parse-buffer) 'src-block
                          (lambda (s) (list (org-element-property :language s)
                                            (org-element-property :value s))))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Build doc → complex heading navigation → verify positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo2_navigation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n** B\n*** C\n** D\n* E\n** F")
  (let ((r '()))
    ;; forward same level
    (goto-char (point-min))
    (org-forward-heading-same-level 1)
    (push (list :fwd1 (org-element-property :raw-value (org-element-at-point))) r)
    (org-forward-heading-same-level 1)
    (push (list :fwd2 (org-element-property :raw-value (org-element-at-point))) r)
    ;; backward same level
    (org-backward-heading-same-level 1)
    (push (list :back1 (org-element-property :raw-value (org-element-at-point))) r)
    ;; up heading
    (org-up-heading)
    (push (list :up1 (org-element-property :raw-value (org-element-at-point))) r)
    ;; next visible
    (goto-char (point-min))
    (org-next-visible-heading 1)
    (push (list :next1 (org-element-property :raw-value (org-element-at-point))) r)
    (org-next-visible-heading 1)
    (push (list :next2 (org-element-property :raw-value (org-element-at-point))) r)
    ;; previous visible
    (org-previous-visible-heading 1)
    (push (list :prev1 (org-element-property :raw-value (org-element-at-point))) r)
    (nreverse r)))"##,
    );
}
