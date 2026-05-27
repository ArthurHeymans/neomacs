//! Strong mega combo oracle tests — extreme multi-operation sequences.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Mega: full document lifecycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_full_document_lifecycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Lifecycle Test\n#+AUTHOR: Test\n* TODO Phase 1\n** TODO Task 1\n** TODO Task 2\n* Phase 2\n** DONE Sub 1\n** TODO Sub 2")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (h1 (org-element-map tree 'headline
               (lambda (h) (list (org-element-property :raw-value h)
                                 (org-element-property :todo-keyword h))))))
    ;; Modify
    (goto-char (point-min))
    (search-forward "Task 1")
    (org-todo 'done)
    (org-set-tags '("done"))
    ;; Read back
    (goto-char (point-min))
    (let* ((tree2 (org-element-parse-buffer))
           (h2 (org-element-map tree2 'headline
                 (lambda (h) (list (org-element-property :raw-value h)
                                   (org-element-property :todo-keyword h)
                                   (org-element-property :tags h))))))
      (list (plist-get info :title) h1 h2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: property inheritance chain with modifications
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_inheritance_chain_mod() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PROPERTY: VAR 1\n* L1\n:PROPERTIES:\n:VAR: 2\n:END:\n** L2\n*** L3\n:PROPERTIES:\n:VAR: 3\n:END:")
  (goto-char (point-min))
  (search-forward "L3")
  (let ((v3i (org-entry-get nil "VAR" 'inherit))
        (v3 (org-entry-get nil "VAR" nil)))
    (org-entry-put nil "VAR" "3new")
    (let ((v3n (org-entry-get nil "VAR" nil))
          (v3ni (org-entry-get nil "VAR" 'inherit)))
      (search-backward "L2")
      (let ((v2i (org-entry-get nil "VAR" 'inherit))
            (v2 (org-entry-get nil "VAR" nil)))
        (list v3i v3 v3n v3ni v2i v2)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: table with formula, sort, transpose
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_formula_sort_transpose() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| 3 | c |\n| 1 | a |\n| 2 | b |\n|---|\n#+TBLFM: $3=$1*10")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (let ((d1 (org-table-to-lisp)))
    (org-table-sort-lines nil ?N)
    (let ((d2 (org-table-to-lisp)))
      (org-table-transpose)
      (let ((d3 (org-table-to-lisp)))
        (list d1 d2 d3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: checkbox hierarchy with statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_checkbox_hierarchy_statistics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [%]\n- [ ] item 1\n  - [ ] sub 1\n  - [ ] sub 2\n- [ ] item 2\n  - [ ] sub 3\n- [ ] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line 2)
    (org-toggle-checkbox)
    (forward-line 1)
    (org-toggle-checkbox)
    (forward-line 1)
    (org-toggle-checkbox)
    (org-update-statistics-cookies t)
    (goto-char (point-min))
    (let ((h1 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
      (list h0 h1))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: sparse tree with tags and visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_tags_visibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task 1 :work:\n* Task 2 :personal:\n* Task 3 :work:urgent:\n* Task 4")
  (goto-char (point-min))
  (org-match-sparse-tree nil "work")
  (let ((visible '())
        (hidden '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((h (org-get-heading t t t t)))
        (when h
          (if (get-char-property (point) 'invisible)
              (push h hidden)
            (push h visible))))
      (forward-line))
    (list (nreverse visible) (nreverse hidden))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: headline edit with context preservation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_edit_context() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Original :old:\n:PROPERTIES:\n:VAR: val\n:END:\nBody text")
  (goto-char (point-min))
  (let ((ctx1 (list (org-get-heading t t t t) (org-get-todo-state)
                    (org-get-priority (char-after)) (org-get-tags nil t)
                    (org-entry-get nil "VAR"))))
    (org-edit-headline "Changed")
    (org-set-tags '("new"))
    (let ((ctx2 (list (org-get-heading t t t t) (org-get-todo-state)
                      (org-get-priority (char-after)) (org-get-tags nil t)
                      (org-entry-get nil "VAR"))))
      (list ctx1 ctx2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: clock with effort and property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_effort_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\n:PROPERTIES:\n:EFFORT: 2:00\n:CATEGORY: work\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-15 10:00]--[2026-01-15 11:30] =>  1:30\nCLOCK: [2026-01-16 14:00]--[2026-01-16 15:00] =>  1:00\n:END:")
  (goto-char (point-min))
  (let ((effort (org-entry-get nil "EFFORT"))
        (category (org-entry-get nil "CATEGORY"))
        (clocked (org-clock-sum-current-entry)))
    (list effort category clocked)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: link with attributes and export
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_attributes_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: My image\n#+ATTR_HTML: :width 300px :class thumb\n#+NAME: fig1\n[[file:image.png]]\n\n#+CAPTION: Other\n#+ATTR_LATEX: :width 0.5\\textwidth\n[[file:other.png]]")
  (let* ((tree (org-element-parse-buffer))
         (links (org-element-map tree 'link
                  (lambda (l)
                    (let ((p (org-element-property :parent l)))
                      (list (org-element-property :path l)
                            (org-element-property :caption p)
                            (org-element-property :attr_html p)
                            (org-element-property :attr_latex p)))))))
    links))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: element chain with all modifications
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_chain_all_mods() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Test :tag:\n:PROPERTIES:\n:VAR: val\n:END:\nBody")
  (goto-char (point-min))
  (let* ((el (org-element-at-point))
         (p1 (list :type (org-element-type el)
                   :todo (org-element-property :todo-keyword el)
                   :pri (org-element-property :priority el)
                   :tags (org-element-property :tags el)
                   :var (org-entry-get nil "VAR"))))
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("newtag"))
    (org-entry-put nil "VAR" "newval")
    (org-edit-headline "Changed")
    (let* ((el2 (org-element-at-point))
           (p2 (list :type (org-element-type el2)
                     :todo (org-element-property :todo-keyword el2)
                     :pri (org-element-property :priority el2)
                     :tags (org-element-property :tags el2)
                     :var (org-entry-get nil "VAR")
                     :title (org-element-property :raw-value el2))))
      (list p1 p2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: multi-buffer with shared state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_buffer_shared() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((r '()))
  (with-temp-buffer
    (org-mode)
    (insert "* A\n** A1\nBody A")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h)))
          r))
  (with-temp-buffer
    (org-mode)
    (insert "* B\n** B1\n** B2\nBody B")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h)))
          r))
  (nreverse r))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: planning with repeaters and delays
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_repeaters_delays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Weekly\nSCHEDULED: <2026-01-15 Wed +1w -3d>\n* TODO Monthly\nDEADLINE: <2026-01-20 Mon +1m -1w>")
  (let* ((tree (org-element-parse-buffer))
         (plan (org-element-map tree 'planning
                 (lambda (p)
                   (let ((s (org-element-property :scheduled p))
                         (d (org-element-property :deadline p)))
                     (list (when s (org-element-property :repeater-type s))
                           (when s (org-element-property :repeater-value s))
                           (when d (org-element-property :repeater-type d))
                           (when d (org-element-property :repeater-value d))))))))
    plan))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: block with switches and parameters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_block_switches_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp -n :results value :exports both\n(+ 1 2)\n#+END_SRC\n\n#+BEGIN_EXAMPLE -n\nExample\n#+END_EXAMPLE")
  (let* ((tree (org-element-parse-buffer))
         (blocks (org-element-map tree '(src-block example-block)
                   (lambda (b)
                     (list (org-element-type b)
                           (org-element-property :language b)
                           (org-element-property :switches b)
                           (org-element-property :parameters b))))))
    blocks))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: headline with all element types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_all_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title :tag:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:VAR: val\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody\n** Sub\n- List\n| tbl |\n#+BEGIN_SRC\n(+ 1)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (h (car (org-element-map tree 'headline (lambda (h) h))))
         (ch (mapcar 'org-element-type (org-element-contents h))))
    (list (org-element-property :todo-keyword h)
          (org-element-property :priority h)
          (org-element-property :tags h)
          (org-element-property :raw-value h)
          ch)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: export with all options and attributes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_all_options_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+OPTIONS: toc:nil num:nil\n#+ATTR_HTML: :id main\n* H\n#+ATTR_LATEX: :options [fragile]\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (hl (org-element-map tree 'headline
               (lambda (h) (org-element-property :raw-value h))))
         (bl (org-element-map tree 'src-block
               (lambda (b)
                 (list (org-element-property :language b)
                       (org-element-property :attr_latex b))))))
    (list (plist-get info :title)
          (plist-get info :with-toc)
          (plist-get info :with-numbers)
          hl bl)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: element hierarchy deep
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_hierarchy_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* L1\n** L2a\n*** L3a\n*** L3b\n** L2b\n*** L3c\n**** L4a\n**** L4b\n* L1b\n** L2c")
  (let* ((tree (org-element-parse-buffer))
         (s (org-element-map tree 'headline
              (lambda (h)
                (list (org-element-property :level h)
                      (org-element-property :raw-value h)
                      (length (org-element-contents h)))))))
    s))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: table complex formulas
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_complex_formulas() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| A | 1 | 2 |\n| B | 3 | 4 |\n| C | 5 | 6 |\n|---+---+---|\n| Sum | 9 | 12 |\n#+TBLFM: $4=$2+$3::@5$2=vsum(@2..@4)::@5$3=vsum(@2..@4)")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (org-table-to-lisp))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: list with checkboxes and statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_checkboxes_statistics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [%]\n- [X] a\n- [ ] b\n  - [X] b1\n  - [ ] b2\n- [X] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: footnote with markup and links
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_markup_links() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] *bold* /italic/\n[fn:2] [[link][desc]]")
  (let* ((tree (org-element-parse-buffer))
         (fn (org-element-map tree 'footnote-reference
               (lambda (f) (org-element-property :label f))))
         (fd (org-element-map tree 'footnote-definition
               (lambda (d) (org-element-property :label d)))))
    (list fn fd)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: timestamp range and repeater
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timestamp_range_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* M\n<2026-01-15 10:00-11:30>\n<2026-01-16>--<2026-01-20>\n<2026-01-25 Wed +1w>")
  (let* ((tree (org-element-parse-buffer))
         (ts (org-element-map tree 'timestamp
               (lambda (t)
                 (list (org-element-property :type t)
                       (org-element-property :year-start t)
                       (org-element-property :day-start t)
                       (org-element-property :hour-start t)
                       (org-element-property :repeater-type t))))))
    ts))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: drawer with multiple types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_multiple_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\n:LOGBOOK:\n- Note\n:END:\n:CUSTOM:\n- Data\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (dr (org-element-map tree 'drawer
               (lambda (d)
                 (list (org-element-property :drawer-name d)
                       (org-element-property :begin d)
                       (org-element-property :end d))))))
    dr))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: inline task
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_inline_task() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-inlinetask)
  (insert "Body\n*************** TODO Inline\n*************** END\nMore")
  (let* ((tree (org-element-parse-buffer))
         (tasks (org-element-map tree 'headline
                  (lambda (h)
                    (when (= (org-element-property :level h) 15)
                      (list (org-element-property :raw-value h)
                            (org-element-property :todo-keyword h)))))))
    tasks))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: entity and radio target
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_entity_radio() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\alpha \\beta\n<<<target>>>\nSee target")
  (let ((before (buffer-string)))
    (org-toggle-pretty-entities)
    (let* ((after (buffer-string))
           (tree (org-element-parse-buffer))
           (targets (org-element-map tree 'radio-target
                      (lambda (rt) (org-element-property :value rt)))))
      (list before after targets))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: outline path and refile
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_outline_path_refile() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P\n** T1\n*** S1.1\n**** SS1.1.1\n** T2")
  (goto-char (point-min))
  (search-forward "SS1.1.1")
  (let ((path (org-get-outline-path))
        (level (org-current-level))
        (title (org-get-heading t t t t))
        (targets (org-refile-get-targets nil)))
    (list path level title (length targets))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: agenda and colview
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_agenda_colview() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %PRIORITY %TAGS %VAR\n* TODO [#A] T :tag:\n:PROPERTIES:\n:VAR: val\n:END:")
  (goto-char (point-min))
  (let ((fmt (org-columns-get-format))
        (entries (org-map-entries
                  (lambda ()
                    (list (org-get-heading t t t t)
                          (org-get-todo-state)
                          (org-get-tags nil t)))
                  nil 'file)))
    (list fmt entries)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: pcomplete and parse consistency
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_pcomplete_parse_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((r '()))
  (with-temp-buffer
    (org-mode)
    (insert "\\agr")
    (push (length (all-completions "\\ag" (pcomplete-entries))) r))
  (with-temp-buffer
    (org-mode)
    (insert "* B1\n** S1\nBody1")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h)))
          r))
  (with-temp-buffer
    (org-mode)
    (insert "* B2\n** S2a\n** S2b\nBody2")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h)))
          r))
  (nreverse r))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: dynamic block clocktable
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_dynamic_block_clocktable() {
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
// Mega: structure template expansion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_structure_template() {
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
// Mega: comment and fixed-width
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_comment_fixed_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# Comment\n: Fixed\nNormal")
  (let* ((tree (org-element-parse-buffer))
         (c (org-element-map tree 'comment
              (lambda (c) (org-element-property :value c))))
         (f (org-element-map tree 'fixed-width
              (lambda (f) (org-element-property :value f)))))
    (list c f)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: affiliated keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_affiliated_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: Cap\n#+ATTR_HTML: :width 300\n#+NAME: n\n[[file:img.png]]")
  (let* ((tree (org-element-parse-buffer))
         (l (car (org-element-map tree 'link (lambda (l) l))))
         (p (org-element-property :parent l)))
    (list (org-element-type p)
          (org-element-property :caption p)
          (org-element-property :attr_html p)
          (org-element-property :name p))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+AUTHOR: A\n#+EMAIL: e\n#+DATE: d\n#+OPTIONS: toc:nil")
  (org-element-map (org-element-parse-buffer) 'keyword
    (lambda (k) (list (org-element-property :key k)
                      (org-element-property :value k)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: macro expansion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_macro_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: g Hello $1 $2!\n{{{g(A, B)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: link types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[[https://x.com][w]] [[file:f.org][f]] [[id:abc][i]] [[elisp:(+ 1)][e]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l)
                      (org-element-property :path l)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: property operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (let ((p1 (org-entry-properties nil 'standard)))
    (org-entry-put nil "C" "3")
    (org-entry-delete nil "B")
    (let ((p2 (org-entry-properties nil 'standard)))
      (list p1 p2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: tag operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_tag_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H :a:b:")
  (goto-char (point-min))
  (let ((t1 (org-get-tags nil t)))
    (org-set-tags '("c" "d"))
    (let ((t2 (org-get-tags nil t)))
      (org-toggle-tag "e" 'on)
      (let ((t3 (org-get-tags nil t)))
        (list t1 t2 t3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: priority operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_priority_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] H\n* TODO [#B] H2\n* TODO H3")
  (goto-char (point-min))
  (let ((p1 (org-get-priority (char-after))))
    (org-priority 'down)
    (let ((p2 (org-get-priority (char-after))))
      (forward-line 2)
      (org-priority ?B)
      (let ((p3 (org-get-priority (char-after))))
        (list p1 p2 p3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: todo cycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_todo_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (setq org-todo-keywords '((sequence "TODO" "PROG" "DONE")))
  (insert "* TODO T")
  (goto-char (point-min))
  (let ((s '()))
    (dotimes (_ 3)
      (push (org-get-todo-state) s)
      (org-todo 'right))
    (push (org-get-todo-state) s)
    (nreverse s)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: visibility cycling
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_visibility_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (let ((s '()))
    (org-set-startup-visibility 'overview)
    (push (get-char-property (search-forward "H2") 'invisible) s)
    (org-set-startup-visibility 'content)
    (push (get-char-property (search-forward "H2") 'invisible) s)
    (org-set-startup-visibility 'all)
    (push (get-char-property (search-forward "H2") 'invisible) s)
    (nreverse s)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mega: sparse tree dates
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_dates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1\nSCHEDULED: <2026-01-15>\n* T2\nSCHEDULED: <2026-01-20>\n* T3\nSCHEDULED: <2026-02-01>\n* T4")
  (goto-char (point-min))
  (org-match-sparse-tree nil "SCHEDULED<=\"<2026-01-31>\"")
  (let ((v '()) (h '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((hd (org-get-heading t t t t)))
        (when hd
          (if (get-char-property (point) 'invisible)
              (push hd h) (push hd v))))
      (forward-line))
    (list (nreverse v) (nreverse h))))"##,
    );
}
