//! Strong uncovered-features-9 oracle tests — test features not yet tested.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-at-heading-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody\n** H2\nSub")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :h1 (org-at-heading-p)) r)
    (forward-line 1)
    (push (list :body (org-at-heading-p)) r)
    (forward-line 1)
    (push (list :h2 (org-at-heading-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-table-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n\nBody")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :row1 (org-at-table-p)) r)
    (forward-line 1)
    (push (list :row2 (org-at-table-p)) r)
    (forward-line 2)
    (push (list :body (org-at-table-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-item-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- item 1\n- item 2\nBody")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :item1 (org-at-item-p)) r)
    (forward-line 1)
    (push (list :item2 (org-at-item-p)) r)
    (forward-line 1)
    (push (list :body (org-at-item-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-planning-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nSCHEDULED: <2026-01-15>\nBody")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :h (org-at-planning-p)) r)
    (forward-line 1)
    (push (list :sched (org-at-planning-p)) r)
    (forward-line 1)
    (push (list :body (org-at-planning-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-property-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\nBody")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :h (org-at-property-p)) r)
    (forward-line 2)
    (push (list :prop (org-at-property-p)) r)
    (forward-line 2)
    (push (list :body (org-at-property-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-comment-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# comment\nNormal\n# another")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :c1 (org-at-comment-p)) r)
    (forward-line 1)
    (push (list :normal (org-at-comment-p)) r)
    (forward-line 1)
    (push (list :c2 (org-at-comment-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-timestamp-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_timestamp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text <2026-01-15> and [2026-01-20] here")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :before (org-at-timestamp-p 'lax)) r)
    (search-forward "<")
    (push (list :active (org-at-timestamp-p 'lax)) r)
    (search-forward "[")
    (push (list :inactive (org-at-timestamp-p 'lax)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-block-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC\n(+ 1)\n#+END_SRC\nBody")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :begin (org-at-block-p)) r)
    (forward-line 1)
    (push (list :inside (org-at-block-p)) r)
    (forward-line 2)
    (push (list :body (org-at-block-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-drawer-p at various positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\nBody")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :h (org-at-drawer-p)) r)
    (forward-line 1)
    (push (list :drawer (org-at-drawer-p)) r)
    (forward-line 3)
    (push (list :body (org-at-drawer-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-footnote-definition-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_footnote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] Definition\nBody")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :text (footnote-at-reference-p)) r)
    (search-forward "[fn:1]")
    (push (list :ref (footnote-at-reference-p)) r)
    (forward-line 1)
    (push (list :def (footnote-at-definition-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-at-clock-log-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_at_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\n:END:\nBody")
  (let ((r '()))
    (goto-char (point-min))
    (search-forward "CLOCK:")
    (push (list :clock (org-at-clock-log-p)) r)
    (forward-line 2)
    (push (list :body (org-at-clock-log-p)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-struct
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_list_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- item 1\n  - sub 1\n  - sub 2\n- item 2")
  (let ((struct (org-list-struct)))
    (mapcar (lambda (s) (list (car s) (cdr s))) struct)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-prevs-alist
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_list_prevs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- a\n  - a1\n  - a2\n- b\n  - b1")
  (let ((prevs (org-list-prevs-alist (org-list-struct))))
    (mapcar (lambda (p) (list (car p) (cdr p))) prevs)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-list-parents-alist
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_list_parents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- a\n  - a1\n  - a2\n- b\n  - b1")
  (let ((parents (org-list-parents-alist (org-list-struct))))
    (mapcar (lambda (p) (list (car p) (cdr p))) parents)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-parent-chain deep
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_parent_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody text")
  (goto-char (point-min))
  (search-forward "Body")
  (let* ((para (org-element-at-point))
         (h3 (org-element-property :parent para))
         (h2 (org-element-property :parent h3))
         (h1 (org-element-property :parent h2))
         (tree (org-element-property :parent h1)))
    (list (org-element-type para)
          (org-element-type h3)
          (org-element-type h2)
          (org-element-type h1)
          (org-element-type tree))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-lineage
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_lineage() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody text")
  (goto-char (point-min))
  (search-forward "Body")
  (let* ((para (org-element-at-point))
         (lineage (org-element-lineage para)))
    (mapcar 'org-element-type lineage)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-contents
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody\n- item\n| tbl |")
  (let* ((tree (org-element-parse-buffer))
         (h (car (org-element-map tree 'headline (lambda (h) h))))
         (children (mapcar 'org-element-type (org-element-contents h))))
    children)"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-type-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_type_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody\n- item\n| tbl |")
  (let* ((tree (org-element-parse-buffer))
         (h (car (org-element-map tree 'headline (lambda (h) h))))
         (p (car (org-element-map tree 'paragraph (lambda (p) p))))
         (pl (car (org-element-map tree 'plain-list (lambda (l) l)))))
    (list (org-element-type-p h 'headline)
          (org-element-type-p h 'paragraph)
          (org-element-type-p p 'paragraph)
          (org-element-type-p p 'headline)
          (org-element-type-p pl 'plain-list)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map no-recursion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_no_recurse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2a\n*** H3\n** H2b")
  (let* ((tree (org-element-parse-buffer))
         (h1 (car (org-element-map tree 'headline (lambda (h) h))))
         (direct (org-element-map (org-element-contents h1) 'headline
                   (lambda (h) (org-element-property :raw-value h))
                   nil nil nil t))
         (recursive (org-element-map (org-element-contents h1) 'headline
                      (lambda (h) (org-element-property :raw-value h)))))
    (list direct recursive)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map first-match
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (let* ((tree (org-element-parse-buffer))
         (all (org-element-map tree 'headline
                (lambda (h) (org-element-property :raw-value h))))
         (first (org-element-map tree 'headline
                  (lambda (h) (org-element-property :raw-value h))
                  nil 'first-match)))
    (list all first)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map with info
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_map_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (let* ((tree (org-element-parse-buffer))
         (result (org-element-map tree 'headline
                   (lambda (h info)
                     (list (org-element-property :raw-value h)
                           (plist-get info :first-match)))
                   nil 'first-match)))
    result)"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-id-get-create
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2")
  (goto-char (point-min))
  (let ((id1 (org-id-get nil 'create)))
    (forward-line)
    (let ((id2 (org-id-get nil 'create)))
      (list (stringp id1) (stringp id2) (string= id1 id2)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-clock-sum
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\n:END:\n* B\n:LOGBOOK:\nCLOCK: [2026-01-11 14:00]--[2026-01-11 15:30] =>  1:30\n:END:")
  (org-clock-sum))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-store-link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_store() {
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
// org-sort-entries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Zebra\n* Apple\n* Mango\n* Banana")
  (org-sort-entries nil ?a)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (org-element-property :raw-value h))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-clone-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_clone() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task\n** Sub1\n** Sub2")
  (goto-char (point-min))
  (org-clone-subtree 2)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :level h)
                      (org-element-property :raw-value h)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-copy/paste subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_copy_paste() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** Sub1\n* H2\n** Sub2")
  (goto-char (point-min))
  (org-copy-subtree)
  (goto-char (point-max))
  (org-paste-subtree 1)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :level h)
                      (org-element-property :raw-value h)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-mark-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_mark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub\n* H1b")
  (goto-char (point-min))
  (org-mark-subtree)
  (let ((m (mark))
        (p (point)))
    (list (< p m) p m)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-narrow-to-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody 1\n** H2\nSub\n* H2b\nBody 2")
  (goto-char (point-min))
  (org-narrow-to-subtree)
  (let ((narrowed (buffer-string)))
    (widen)
    (list narrowed)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-end-of-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2a\n*** H3\nBody\n** H2b\n* H1b")
  (goto-char (point-min))
  (let ((p1 (progn (org-end-of-subtree) (point))))
    (goto-char (point-min))
    (search-forward "H2a")
    (beginning-of-line)
    (let ((p2 (progn (org-end-of-subtree) (point))))
      (list p1 p2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cycle-hide-drawers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_hide() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n:PROPERTIES:\n:A: 1\n:END:\nBody\n* H2")
  (goto-char (point-min))
  (org-cycle-hide-drawers 'all)
  (let ((hidden1 (get-char-property (search-forward "A") 'invisible)))
    (goto-char (point-max))
    (org-cycle-hide-drawers nil)
    (let ((hidden2 (get-char-property (search-forward "A") 'invisible)))
      (list hidden1 hidden2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-toggle-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Plain text\n* Existing heading")
  (goto-char (point-min))
  (org-toggle-heading)
  (let ((s1 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line)
    (org-toggle-heading)
    (let ((s2 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
      (list s1 s2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-move-item
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_move_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (let ((d1 (org-element-map (org-element-parse-buffer) 'item
              (lambda (i) (org-trim (buffer-substring-no-properties
                                      (org-element-property :contents-begin i)
                                      (org-element-property :contents-end i)))))))
    (org-move-item-down)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'item
                (lambda (i) (org-trim (buffer-substring-no-properties
                                        (org-element-property :contents-begin i)
                                        (org-element-property :contents-end i)))))))
      (list d1 d2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-insert-todo-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_insert_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Existing")
  (goto-char (point-max))
  (org-insert-todo-heading nil)
  (insert "New task")
  (org-insert-todo-heading 'right)
  (insert "Right task")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :raw-value h)
                      (org-element-property :todo-keyword h)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-move-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_move() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n* B\n* C\n* D")
  (goto-char (point-min))
  (let ((o1 (org-element-map (org-element-parse-buffer) 'headline
              (lambda (h) (org-element-property :raw-value h)))))
    (forward-line 1)
    (org-move-subtree-down)
    (let ((o2 (org-element-map (org-element-parse-buffer) 'headline
                (lambda (h) (org-element-property :raw-value h)))))
      (list o1 o2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-dblock-update
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_dblock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN: clocktable :maxlevel 2\n#+END:")
  (goto-char (point-min))
  (org-dblock-update)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro-replace-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: g Hello $1!\n{{{g(World)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-try-structure-completion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_structure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<s")
  (org-try-structure-completion)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-update-statistics-cookies
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [X] a\n- [ ] b\n- [X] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-match-sparse-tree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf9_sparse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C")
  (goto-char (point-min))
  (org-match-sparse-tree nil "TODO")
  (let ((v '()) (h '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((hd (org-get-heading t t t t)))
        (when hd
          (if (get-char-property (point) 'invisible) (push hd h) (push hd v))))
      (forward-line))
    (list (nreverse v) (nreverse h))))"##,
    );
}
