//! Strong final edge-case oracle tests — comprehensive boundary coverage.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Final edge: complete document structure
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_complete_document() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Complete Document\n#+AUTHOR: Test Author\n#+DATE: 2026-01-15\n#+OPTIONS: toc:nil\n* Chapter 1\n** Section 1.1\nBody text\n*** Subsection 1.1.1\nMore text\n** Section 1.2\n- List item 1\n- List item 2\n* Chapter 2\n| Table | Data |\n|-------+------|\n| A | 1 |\n| B | 2 |\n* Chapter 3\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (headlines (org-element-map tree 'headline
                      (lambda (h)
                        (list (org-element-property :level h)
                              (org-element-property :raw-value h))))))
    (list (plist-get info :title) headlines)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: complex editing sequence
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_complex_editing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Original\nBody")
  (goto-char (point-min))
  (let ((s1 (buffer-string)))
    (org-edit-headline "Changed 1")
    (org-set-tags '("tag1"))
    (let ((s2 (buffer-string)))
      (org-todo 'right)
      (org-priority 'down)
      (let ((s3 (buffer-string)))
        (org-entry-put nil "VAR" "val")
        (org-edit-headline "Changed 2")
        (let ((s4 (buffer-string)))
          (list s1 s2 s3 s4))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: multi-element document
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_element_doc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\n:PROPERTIES:\n:VAR: val\n:END:\nBody text\n- List item\n| table |\n#+BEGIN_SRC\n(+ 1 2)\n#+END_SRC\n# comment\n: fixed-width")
  (let* ((tree (org-element-parse-buffer))
         (types (org-element-map tree (lambda (el) (org-element-type el)))))
    types))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: export with all options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_all_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+AUTHOR: Author\n#+EMAIL: test@example.com\n#+DATE: 2026-01-15\n#+DESCRIPTION: Desc\n#+KEYWORDS: kw1 kw2\n#+LANGUAGE: en\n#+SELECT_TAGS: export\n#+EXCLUDE_TAGS: noexport\n#+OPTIONS: toc:nil num:nil ^:nil\n* Heading\n** Sub")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil)))
    (list (plist-get info :title)
          (plist-get info :author)
          (plist-get info :email)
          (plist-get info :with-toc)
          (plist-get info :with-numbers))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: element hierarchy deep
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_hierarchy_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* L1\n** L2a\n*** L3a\n*** L3b\n** L2b\n*** L3c\n**** L4a\n**** L4b\n* L1b\n** L2c")
  (let* ((tree (org-element-parse-buffer))
         (structure (org-element-map tree 'headline
                      (lambda (h)
                        (list (org-element-property :level h)
                              (org-element-property :raw-value h)
                              (length (org-element-contents h)))))))
    structure))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: table with complex formulas
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
  (let ((data (org-table-to-lisp)))
    data))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: list with checkboxes and statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_checkboxes_statistics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [%]\n- [X] item 1\n- [ ] item 2\n  - [X] sub 1\n  - [ ] sub 2\n- [X] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    h))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: footnote with markup
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_with_markup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] Footnote with *bold* and /italic/\n[fn:2] Footnote with [[link][desc]]")
  (let* ((tree (org-element-parse-buffer))
         (footnotes (org-element-map tree 'footnote-reference
                      (lambda (fn) (org-element-property :label fn))))
         (defs (org-element-map tree 'footnote-definition
                 (lambda (fd) (org-element-property :label fd)))))
    (list footnotes defs)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: clock with effort and property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_effort_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\n:PROPERTIES:\n:EFFORT: 2:00\n:CATEGORY: work\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-15 10:00]--[2026-01-15 11:30] =>  1:30\n:END:")
  (goto-char (point-min))
  (let ((effort (org-entry-get nil "EFFORT"))
        (category (org-entry-get nil "CATEGORY"))
        (clocked (org-clock-sum-current-entry)))
    (list effort category clocked)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: link with attributes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_with_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: My image\n#+ATTR_HTML: :width 300px :class thumbnail\n#+NAME: fig1\n[[file:image.png]]")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l))))
         (parent (org-element-property :parent link)))
    (list (org-element-property :path link)
          (org-element-property :caption parent)
          (org-element-property :attr_html parent)
          (org-element-property :name parent))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: planning with repeaters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_with_repeaters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Weekly\nSCHEDULED: <2026-01-15 Wed +1w>\n* TODO Monthly\nDEADLINE: <2026-01-20 Mon +1m>")
  (let* ((tree (org-element-parse-buffer))
         (planning (org-element-map tree 'planning
                     (lambda (p)
                       (let ((sched (org-element-property :scheduled p))
                             (dl (org-element-property :deadline p)))
                         (list (when sched (org-element-property :repeater-type sched))
                               (when dl (org-element-property :repeater-type dl))))))))
    planning))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: block with switches
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_block_with_switches() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp -n :results value :exports both\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (block (car (org-element-map tree 'src-block (lambda (b) b)))))
    (list (org-element-property :language block)
          (org-element-property :switches block)
          (org-element-property :parameters block))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: headline with all elements
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_all_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title :tag:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:VAR: val\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (headline (car (org-element-map tree 'headline (lambda (h) h))))
         (planning (car (org-element-map (org-element-contents headline) 'planning
                         (lambda (p) p))))
         (drawers (org-element-map (org-element-contents headline) 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    (list (org-element-property :todo-keyword headline)
          (org-element-property :priority headline)
          (org-element-property :tags headline)
          (org-element-property :scheduled planning)
          (org-element-property :deadline planning)
          drawers)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: multi-buffer parse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_buffer_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((results '()))
  (with-temp-buffer
    (org-mode)
    (insert "* Buffer A\n** Sub A\nBody A")
    (let ((tree (org-element-parse-buffer)))
      (push (org-element-map tree 'headline
              (lambda (h) (org-element-property :raw-value h)))
            results)))
  (with-temp-buffer
    (org-mode)
    (insert "* Buffer B\n** Sub B1\n** Sub B2\nBody B")
    (let ((tree (org-element-parse-buffer)))
      (push (org-element-map tree 'headline
              (lambda (h) (org-element-property :raw-value h)))
            results)))
  (nreverse results))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: sparse tree with tags
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_tags() {
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
// Final edge: property inheritance
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_inheritance() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PROPERTY: VAR 1\n* Level 1\n:PROPERTIES:\n:VAR: 2\n:END:\n** Level 2\n*** Level 3")
  (goto-char (point-min))
  (search-forward "Level 3")
  (let ((v3 (org-entry-get nil "VAR" 'inherit))
        (v3nil (org-entry-get nil "VAR" nil)))
    (search-backward "Level 2")
    (let ((v2 (org-entry-get nil "VAR" 'inherit))
          (v2nil (org-entry-get nil "VAR" nil)))
      (search-backward "Level 1")
      (let ((v1 (org-entry-get nil "VAR" 'inherit))
            (v1nil (org-entry-get nil "VAR" nil)))
        (list v1 v1nil v2 v2nil v3 v3nil)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: element deferred operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_deferred_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Test :tag:\n:PROPERTIES:\n:VAR: val\n:END:\nBody")
  (goto-char (point-min))
  (let* ((el (org-element-at-point))
         (props1 (list :type (org-element-type el)
                       :todo (org-element-property :todo-keyword el)
                       :priority (org-element-property :priority el)
                       :tags (org-element-property :tags el)
                       :var (org-entry-get nil "VAR"))))
    ;; Modify all
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("newtag"))
    (org-entry-put nil "VAR" "newval")
    (org-edit-headline "Changed")
    ;; Read back
    (let* ((el2 (org-element-at-point))
           (props2 (list :type (org-element-type el2)
                         :todo (org-element-property :todo-keyword el2)
                         :priority (org-element-property :priority el2)
                         :tags (org-element-property :tags el2)
                         :var (org-entry-get nil "VAR")
                         :title (org-element-property :raw-value el2))))
      (list props1 props2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: table with mixed content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_mixed_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| *bold* | /italic/ |\n| =code= | _underlined_ |\n| [[link][desc]] | 123 |")
  (let* ((tree (org-element-parse-buffer))
         (cells (org-element-map tree 'table-cell
                  (lambda (c) (org-element-property :value c)))))
    cells))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: timestamp range
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timestamp_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Meeting\n<2026-01-15 10:00-11:30>\n<2026-01-16>--<2026-01-20>")
  (let* ((tree (org-element-parse-buffer))
         (timestamps (org-element-map tree 'timestamp
                       (lambda (ts)
                         (list (org-element-property :type ts)
                               (org-element-property :year-start ts)
                               (org-element-property :day-start ts)
                               (org-element-property :hour-start ts)
                               (org-element-property :minute-start ts))))))
    timestamps))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: drawer with content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_with_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:\n:LOGBOOK:\n- Note taken on [2026-01-15] \\\\\n  Test note\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (drawers (org-element-map tree 'drawer
                    (lambda (d)
                      (list (org-element-property :drawer-name d)
                            (org-element-property :begin d)
                            (org-element-property :end d))))))
    drawers))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: inline task
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_inline_task() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-inlinetask)
  (insert "Body text\n*************** TODO Inline task\n*************** END\nMore body")
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
// Final edge: entity replacement
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_entity_replacement() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Hello \\alpha and \\beta and \\gamma")
  (let ((before (buffer-string)))
    (org-toggle-pretty-entities)
    (let ((after (buffer-string)))
      (list before after))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: radio targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_radio_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<<<target1>>>\n<<<target2>>>\n<<<target3>>>")
  (let* ((tree (org-element-parse-buffer))
         (targets (org-element-map tree 'radio-target
                    (lambda (rt) (org-element-property :value rt)))))
    targets))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: statistics cookies
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_statistics_cookies() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [/]\n- [X] item 1\n- [ ] item 2\n- [X] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    h))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: sparse tree with dates
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_dates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task 1\nSCHEDULED: <2026-01-15>\n* Task 2\nSCHEDULED: <2026-01-20>\n* Task 3\nSCHEDULED: <2026-02-01>\n* Task 4")
  (goto-char (point-min))
  (org-match-sparse-tree nil "SCHEDULED<=\"<2026-01-31>\"")
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
// Final edge: outline path
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_outline_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Project\n** Task 1\n*** Subtask 1.1\n**** Subsub 1.1.1\n** Task 2")
  (goto-char (point-min))
  (search-forward "Subsub 1.1.1")
  (let ((path (org-get-outline-path))
        (level (org-current-level))
        (title (org-get-heading t t t t)))
    (list path level title)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: refile targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_refile_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Project A\n** Task 1\n** Task 2\n* Project B\n** Task 3")
  (let ((targets (org-refile-get-targets nil)))
    (mapcar (lambda (t) (car t)) targets)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: agenda todo list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_agenda_todo_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task 1\n* DONE Task 2\n* TODO Task 3\n* WAITING Task 4")
  (let ((entries (org-map-entries
                  (lambda ()
                    (list (org-get-heading t t t t)
                          (org-get-todo-state)
                          (org-entry-get nil "PRIORITY")))
                  nil 'file)))
    entries))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: colview format
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_colview_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %3PRIORITY %TAGS %VAR\n* TODO [#A] Test :tag:")
  (goto-char (point-min))
  (let ((fmt (org-columns-get-format)))
    fmt))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: pcomplete entity
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_pcomplete_entity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\agr")
  (let ((completions (all-completions "\\ag" (pcomplete-entries))))
    (length completions)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final edge: parse consistency
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_parse_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((results '()))
  (with-temp-buffer
    (org-mode)
    (insert "* Buffer 1\n** Sub 1\nBody 1")
    (let ((tree (org-element-parse-buffer)))
      (push (org-element-map tree 'headline
              (lambda (h) (org-element-property :raw-value h)))
            results)))
  (with-temp-buffer
    (org-mode)
    (insert "* Buffer 2\n** Sub 2\nBody 2")
    (let ((tree (org-element-parse-buffer)))
      (push (org-element-map tree 'headline
              (lambda (h) (org-element-property :raw-value h)))
            results)))
  (nreverse results))"##,
    );
}
