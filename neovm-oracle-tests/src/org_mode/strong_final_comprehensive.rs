//! Strong final comprehensive oracle tests — ultimate coverage.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: complete workflow
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_complete_workflow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Project Plan\n#+AUTHOR: Team\n#+DATE: 2026-01-15\n#+OPTIONS: toc:nil\n* TODO Phase 1\nDEADLINE: <2026-02-01>\n:PROPERTIES:\n:EFFORT: 5d\n:END:\n** TODO Task 1.1\nSCHEDULED: <2026-01-20>\n** TODO Task 1.2\nDEADLINE: <2026-01-25>\n* DONE Phase 2\nCLOSED: [2026-01-10]\n** DONE Task 2.1\n** DONE Task 2.2\n* Phase 3\n** TODO Task 3.1\n** TODO Task 3.2")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (headlines (org-element-map tree 'headline
                      (lambda (h)
                        (list (org-element-property :level h)
                              (org-element-property :raw-value h)
                              (org-element-property :todo-keyword h))))))
    (list (plist-get info :title) headlines)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: complex editing with undo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_complex_editing_undo() {
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
          (undo)
          (undo)
          (undo)
          (let ((s5 (buffer-string)))
            (list s1 s2 s3 s4 s5)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: multi-element with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_element_all_features() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\n:PROPERTIES:\n:VAR: val\n:END:\nBody text\n- List item\n| table |\n#+BEGIN_SRC\n(+ 1 2)\n#+END_SRC\n# comment\n: fixed-width\n#+TITLE: Test\n<2026-01-15>\n[2026-01-20]\n[[link][desc]]\n*bold*\n/italic/\n_code_\n~verbatim~\n=code=")
  (let* ((tree (org-element-parse-buffer))
         (types (org-element-map tree (lambda (el) (org-element-type el)))))
    types))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: export with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_all_features() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+AUTHOR: Author\n#+EMAIL: test@example.com\n#+DATE: 2026-01-15\n#+DESCRIPTION: Desc\n#+KEYWORDS: kw1 kw2\n#+LANGUAGE: en\n#+SELECT_TAGS: export\n#+EXCLUDE_TAGS: noexport\n#+OPTIONS: toc:nil num:nil ^:nil\n#+CAPTION: My image\n#+ATTR_HTML: :width 300px\n#+NAME: fig1\n[[file:image.png]]\n* Heading\n** Sub\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (headlines (org-element-map tree 'headline
                      (lambda (h) (org-element-property :raw-value h))))
         (blocks (org-element-map tree 'src-block
                   (lambda (b) (org-element-property :language b))))
         (links (org-element-map tree 'link
                  (lambda (l) (org-element-property :path l)))))
    (list (plist-get info :title)
          (plist-get info :author)
          (plist-get info :email)
          headlines blocks links)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: element hierarchy with all operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_hierarchy_all_ops() {
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
// Final comprehensive: table with all formulas
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_all_formulas() {
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
// Final comprehensive: list with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_all_features() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [%]\n- [X] item 1\n- [ ] item 2\n  - [X] sub 1\n  - [ ] sub 2\n- [X] item 3\n  - [ ] sub 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    h))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: footnote with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_all_features() {
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
// Final comprehensive: clock with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_all_features() {
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
// Final comprehensive: link with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_all_features() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: My image\n#+ATTR_HTML: :width 300px :class thumbnail\n#+NAME: fig1\n[[file:image.png]]\n\n#+CAPTION: Another image\n#+ATTR_LATEX: :width 0.5\\textwidth\n[[file:other.png]]")
  (let* ((tree (org-element-parse-buffer))
         (links (org-element-map tree 'link
                  (lambda (l)
                    (let ((parent (org-element-property :parent l)))
                      (list (org-element-property :path l)
                            (org-element-property :caption parent)
                            (org-element-property :attr_html parent)
                            (org-element-property :attr_latex parent)))))))
    links))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: planning with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_all_features() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Weekly\nSCHEDULED: <2026-01-15 Wed +1w -3d>\n* TODO Monthly\nDEADLINE: <2026-01-20 Mon +1m -1w>")
  (let* ((tree (org-element-parse-buffer))
         (planning (org-element-map tree 'planning
                     (lambda (p)
                       (let ((sched (org-element-property :scheduled p))
                             (dl (org-element-property :deadline p)))
                         (list (when sched (org-element-property :repeater-type sched))
                               (when sched (org-element-property :repeater-value sched))
                               (when dl (org-element-property :repeater-type dl))
                               (when dl (org-element-property :repeater-value dl))))))))
    planning))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: block with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_block_all_features() {
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
// Final comprehensive: headline with all elements
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_all_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title :tag:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:VAR: val\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody\n** Sub\n- List\n| table |\n#+BEGIN_SRC\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (headline (car (org-element-map tree 'headline (lambda (h) h))))
         (children (mapcar 'org-element-type (org-element-contents headline))))
    (list (org-element-property :todo-keyword headline)
          (org-element-property :priority headline)
          (org-element-property :tags headline)
          (org-element-property :raw-value headline)
          children)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Final comprehensive: multi-buffer with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_buffer_all_features() {
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
// Final comprehensive: sparse tree with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_all_features() {
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
// Final comprehensive: property with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_all_features() {
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
// Final comprehensive: element with all operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_all_operations() {
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
// Final comprehensive: table with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_all_features() {
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
// Final comprehensive: timestamp with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timestamp_all_features() {
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
// Final comprehensive: drawer with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_all_features() {
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
// Final comprehensive: inline task with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_inline_task_all_features() {
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
// Final comprehensive: entity with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_entity_all_features() {
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
// Final comprehensive: radio targets with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_radio_targets_all_features() {
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
// Final comprehensive: statistics with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_statistics_all_features() {
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
// Final comprehensive: sparse tree with dates
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
// Final comprehensive: outline path with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_outline_path_all_features() {
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
// Final comprehensive: refile targets with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_refile_targets_all_features() {
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
// Final comprehensive: agenda todo with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_agenda_todo_all_features() {
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
// Final comprehensive: colview format with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_colview_format_all_features() {
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
// Final comprehensive: pcomplete entity with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_pcomplete_entity_all_features() {
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
// Final comprehensive: parse consistency with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_parse_consistency_all_features() {
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
