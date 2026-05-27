//! Strong giga combo oracle tests — massive multi-operation sequences.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Giga: complete project workflow
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_complete_project_workflow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Project\n#+AUTHOR: Team\n* TODO Planning\nDEADLINE: <2026-02-01>\n:PROPERTIES:\n:EFFORT: 5d\n:END:\n** TODO Design\nSCHEDULED: <2026-01-20>\n** TODO Implement\nDEADLINE: <2026-02-15>\n* DONE Review\nCLOSED: [2026-01-10]\n** DONE Code review\n** DONE Test review")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (hl (org-element-map tree 'headline
               (lambda (h) (list (org-element-property :raw-value h)
                                 (org-element-property :todo-keyword h)
                                 (org-element-property :tags h))))))
    (list (plist-get info :title) hl)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: multi-step property chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_step_property_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (let ((p1 (org-entry-properties nil 'standard)))
    (org-entry-put nil "C" "3")
    (org-entry-put nil "A" "10")
    (org-entry-delete nil "B")
    (let ((p2 (org-entry-properties nil 'standard)))
      (org-entry-put nil "D" "4")
      (let ((p3 (org-entry-properties nil 'standard)))
        (list p1 p2 p3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: table with all operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_all_operations() {
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
// Giga: checkbox with statistics update
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_checkbox_statistics_update() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [%]\n- [ ] a\n  - [ ] a1\n  - [ ] a2\n- [ ] b\n  - [ ] b1\n- [ ] c")
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
// Giga: sparse tree with multiple criteria
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_multi_criteria() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1 :work:\n* T2 :personal:\n* T3 :work:urgent:\n* T4")
  (goto-char (point-min))
  (org-match-sparse-tree nil "work")
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

// ═══════════════════════════════════════════════════════════════════════
// Giga: headline with all metadata
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_all_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title :tag:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:VAR: val\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (h (car (org-element-map tree 'headline (lambda (h) h))))
         (p (car (org-element-map (org-element-contents h) 'planning
                   (lambda (p) p))))
         (dr (org-element-map (org-element-contents h) 'drawer
               (lambda (d) (org-element-property :drawer-name d)))))
    (list (org-element-property :todo-keyword h)
          (org-element-property :priority h)
          (org-element-property :tags h)
          (org-element-property :scheduled p)
          (org-element-property :deadline p)
          dr)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: export with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_all_features() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+OPTIONS: toc:nil num:nil\n#+ATTR_HTML: :id m\n* H\n#+ATTR_LATEX: :options [f]\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
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
// Giga: element hierarchy with all ops
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_hierarchy_all_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* L1\n** L2a\n*** L3a\n*** L3b\n** L2b\n*** L3c\n**** L4a\n**** L4b\n* L1b\n** L2c")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h)
      (list (org-element-property :level h)
            (org-element-property :raw-value h)
            (length (org-element-contents h))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: table with complex formulas
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
// Giga: list with all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_all_features() {
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
// Giga: footnote with markup and links
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
// Giga: clock with effort and property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_effort_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\n:PROPERTIES:\n:EFFORT: 2:00\n:CATEGORY: w\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-15 10:00]--[2026-01-15 11:30] =>  1:30\nCLOCK: [2026-01-16 14:00]--[2026-01-16 15:00] =>  1:00\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "EFFORT")
        (org-entry-get nil "CATEGORY")
        (org-clock-sum-current-entry)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: link with attributes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: C\n#+ATTR_HTML: :width 300\n#+NAME: n\n[[file:i.png]]")
  (let* ((tree (org-element-parse-buffer))
         (l (car (org-element-map tree 'link (lambda (l) l))))
         (p (org-element-property :parent l)))
    (list (org-element-property :path l)
          (org-element-property :caption p)
          (org-element-property :attr_html p)
          (org-element-property :name p))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: element chain with all mods
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_chain_all_mods() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] T :tag:\n:PROPERTIES:\n:V: val\n:END:\nBody")
  (goto-char (point-min))
  (let* ((el (org-element-at-point))
         (p1 (list :type (org-element-type el)
                   :todo (org-element-property :todo-keyword el)
                   :pri (org-element-property :priority el)
                   :tags (org-element-property :tags el)
                   :var (org-entry-get nil "V"))))
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("new"))
    (org-entry-put nil "V" "new")
    (org-edit-headline "Changed")
    (let* ((el2 (org-element-at-point))
           (p2 (list :type (org-element-type el2)
                     :todo (org-element-property :todo-keyword el2)
                     :pri (org-element-property :priority el2)
                     :tags (org-element-property :tags el2)
                     :var (org-entry-get nil "V")
                     :title (org-element-property :raw-value el2))))
      (list p1 p2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: multi-buffer parse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_buffer_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((r '()))
  (with-temp-buffer
    (org-mode)
    (insert "* A\n** A1\nBodyA")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h)))
          r))
  (with-temp-buffer
    (org-mode)
    (insert "* B\n** B1\n** B2\nBodyB")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h)))
          r))
  (nreverse r))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: planning with repeaters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_repeaters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO W\nSCHEDULED: <2026-01-15 Wed +1w -3d>\n* TODO M\nDEADLINE: <2026-01-20 Mon +1m -1w>")
  (org-element-map (org-element-parse-buffer) 'planning
    (lambda (p)
      (let ((s (org-element-property :scheduled p))
            (d (org-element-property :deadline p)))
        (list (when s (org-element-property :repeater-type s))
              (when s (org-element-property :repeater-value s))
              (when d (org-element-property :repeater-type d))
              (when d (org-element-property :repeater-value d)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: block with switches
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_block_switches() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp -n :results value :exports both\n(+ 1 2)\n#+END_SRC\n\n#+BEGIN_EXAMPLE -n\nExample\n#+END_EXAMPLE")
  (org-element-map (org-element-parse-buffer) '(src-block example-block)
    (lambda (b)
      (list (org-element-type b)
            (org-element-property :language b)
            (org-element-property :switches b)
            (org-element-property :parameters b)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: timestamp range and repeater
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timestamp_range_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* M\n<2026-01-15 10:00-11:30>\n<2026-01-16>--<2026-01-20>\n<2026-01-25 Wed +1w>")
  (org-element-map (org-element-parse-buffer) 'timestamp
    (lambda (t)
      (list (org-element-property :type t)
            (org-element-property :year-start t)
            (org-element-property :day-start t)
            (org-element-property :hour-start t)
            (org-element-property :repeater-type t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: drawer with multiple types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\n:LOGBOOK:\n- N\n:END:\n:CUSTOM:\n- D\n:END:\nBody")
  (org-element-map (org-element-parse-buffer) 'drawer
    (lambda (d)
      (list (org-element-property :drawer-name d)
            (org-element-property :begin d)
            (org-element-property :end d)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: inline task
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_inline_task() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-inlinetask)
  (insert "Body\n*************** TODO Inline\n*************** END\nMore")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h)
      (when (= (org-element-property :level h) 15)
        (list (org-element-property :raw-value h)
              (org-element-property :todo-keyword h))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: entity and radio
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
           (targets (org-element-map (org-element-parse-buffer) 'radio-target
                      (lambda (rt) (org-element-property :value rt)))))
      (list before after targets))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: outline path and refile
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_outline_refile() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P\n** T1\n*** S1\n**** SS1\n** T2")
  (goto-char (point-min))
  (search-forward "SS1")
  (list (org-get-outline-path)
        (org-current-level)
        (org-get-heading t t t t)
        (length (org-refile-get-targets nil))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: agenda and colview
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_agenda_colview() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %PRIORITY %TAGS %V\n* TODO [#A] T :tag:\n:PROPERTIES:\n:V: val\n:END:")
  (goto-char (point-min))
  (list (org-columns-get-format)
        (org-map-entries
          (lambda ()
            (list (org-get-heading t t t t)
                  (org-get-todo-state)
                  (org-get-tags nil t)))
          nil 'file)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: pcomplete and parse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_pcomplete_parse() {
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
  (nreverse r))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Giga: dynamic block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_dynamic_block() {
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
// Giga: structure template
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
// Giga: comment and fixed-width
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
// Giga: affiliated keywords
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
// Giga: keywords
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
// Giga: macro expansion
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
// Giga: link types
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
// Giga: property operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_ops() {
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
// Giga: tag operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_tag_ops() {
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
// Giga: priority operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_priority_ops() {
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
// Giga: todo cycle
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
// Giga: visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_visibility() {
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
// Giga: sparse tree dates
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
