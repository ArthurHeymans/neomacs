//! Strong integration combo oracle tests — cross-feature interactions.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Integration: table + formula + export
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_formula_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Sales Report\n| Product | Q1 | Q2 | Total |\n|---------+----+----+-------|\n| A | 100 | 150 | |\n| B | 200 | 250 | |\n| C | 300 | 350 | |\n|---------+----+----+-------|\n| Sum | 600 | 750 | |\n#+TBLFM: $4=$2+$3::@5$2=vsum(@2..@4)::@5$3=vsum(@2..@4)::@5$4=vsum(@2..@4)")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (let ((data (org-table-to-lisp))
        (title (org-element-property :value
                 (car (org-element-map (org-element-parse-buffer) 'keyword
                        (lambda (k) (equal (org-element-property :key k) "TITLE")))))))
    (list title data)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: headline + property + clock
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_property_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task with properties\n:PROPERTIES:\n:EFFORT: 2:00\n:CATEGORY: work\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-15 10:00]--[2026-01-15 11:30] =>  1:30\n:END:")
  (goto-char (point-min))
  (let ((todo (org-get-todo-state))
        (effort (org-entry-get nil "EFFORT"))
        (category (org-entry-get nil "CATEGORY"))
        (clocked (org-clock-sum-current-entry)))
    (list todo effort category clocked)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: link + export + attributes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_export_attributes() {
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
// Integration: todo + tag + property + planning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_todo_tag_property_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Important task :work:urgent:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:EFFORT: 3:00\n:ASSIGNEE: Alice\n:END:")
  (goto-char (point-min))
  (let ((todo (org-get-todo-state))
        (priority (org-get-priority (char-after)))
        (tags (org-get-tags nil t))
        (sched (org-entry-get nil "SCHEDULED"))
        (deadline (org-entry-get nil "DEADLINE"))
        (effort (org-entry-get nil "EFFORT"))
        (assignee (org-entry-get nil "ASSIGNEE")))
    (list todo priority tags sched deadline effort assignee)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: list + checkbox + statistics + export
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_checkbox_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Task List\n* Project [%]\n- [ ] Task 1\n- [X] Task 2\n- [ ] Task 3\n  - [X] Subtask 3.1\n  - [ ] Subtask 3.2")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h (buffer-substring-no-properties (line-beginning-position) (line-end-position)))
        (title (org-element-property :value
                 (car (org-element-map (org-element-parse-buffer) 'keyword
                        (lambda (k) (equal (org-element-property :key k) "TITLE")))))))
    (list title h)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: footnote + link + inline
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_link_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text with footnote[fn:1] and *bold* and /italic/ and [[link][desc]].\n\n[fn:1] Footnote with *markup* and [[link2][desc2]].")
  (let* ((tree (org-element-parse-buffer))
         (footnotes (org-element-map tree 'footnote-reference
                      (lambda (fn) (org-element-property :label fn))))
         (links (org-element-map tree 'link
                  (lambda (l) (org-element-property :path l))))
         (bold (org-element-map tree 'bold
                 (lambda (b) (org-element-property :value b)))))
    (list footnotes links bold)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: block + result + link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_block_result_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+NAME: calc\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC\n\n#+RESULTS: calc\n: 3\n\nSee [[calc][the calculation]].")
  (let* ((tree (org-element-parse-buffer))
         (blocks (org-element-map tree 'src-block
                   (lambda (b)
                     (list (org-element-property :name b)
                           (org-element-property :language b)))))
         (results (org-element-map tree 'fixed-width
                    (lambda (f) (org-element-property :value f))))
         (links (org-element-map tree 'link
                  (lambda (l) (org-element-property :path l)))))
    (list blocks results links)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: drawer + property + visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_property_visibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:VAR: val\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody")
  (goto-char (point-min))
  (org-cycle-hide-drawers 'overview)
  (let ((props-hidden (get-char-property (search-forward "VAR") 'invisible))
        (log-hidden (get-char-property (search-forward "Note") 'invisible)))
    (list props-hidden log-hidden)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: sparse tree + property + tag
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_property_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task 1 :work:\n:PROPERTIES:\n:PRI: high\n:END:\n* DONE Task 2 :personal:\n:PROPERTIES:\n:PRI: low\n:END:\n* TODO Task 3 :work:\n:PROPERTIES:\n:PRI: high\n:END:\n* WAITING Task 4")
  (goto-char (point-min))
  (org-match-sparse-tree nil "work+TODO=\"TODO\"")
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
// Integration: clock + effort + property
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
// Integration: headline + timestamp + planning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_timestamp_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Meeting <2026-01-15 Wed>\nSCHEDULED: <2026-01-14 Tue 10:00>\nDEADLINE: <2026-01-16 Thu 17:00>\nBody with <2026-01-17 Fri> date")
  (let* ((tree (org-element-parse-buffer))
         (headline (car (org-element-map tree 'headline (lambda (h) h))))
         (planning (car (org-element-map (org-element-contents headline) 'planning
                         (lambda (p) p))))
         (timestamps (org-element-map tree 'timestamp
                       (lambda (ts)
                         (list (org-element-property :type ts)
                               (org-element-property :year-start ts)
                               (org-element-property :day-start ts))))))
    (list (org-element-property :raw-value headline)
          (org-element-property :scheduled planning)
          (org-element-property :deadline planning)
          timestamps)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: table + list + block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_list_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| A | B |\n| 1 | 2 |\n\n- Item 1\n- Item 2\n\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (tables (org-element-map tree 'table
                   (lambda (t) (length (org-element-contents t)))))
         (lists (org-element-map tree 'plain-list
                  (lambda (l) (length (org-element-contents l)))))
         (blocks (org-element-map tree 'src-block
                   (lambda (b) (org-element-property :language b)))))
    (list tables lists blocks)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: export + filter + attribute
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_filter_attribute() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+ATTR_HTML: :id main\n* Heading\n#+ATTR_LATEX: :options [fragile]\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (headlines (org-element-map tree 'headline
                      (lambda (h) (org-element-property :raw-value h))))
         (blocks (org-element-map tree 'src-block
                   (lambda (b)
                     (list (org-element-property :language b)
                           (org-element-property :attr_latex b))))))
    (list (plist-get info :title) headlines blocks)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: element + deferred + chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_deferred_chain() {
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
    ;; Chain of modifications
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("newtag"))
    (org-entry-put nil "VAR" "newval")
    (org-edit-headline "Changed")
    ;; Read back all
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
// Integration: multi-buffer + shared state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_buffer_shared() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((results '()))
  (with-temp-buffer
    (org-mode)
    (insert "* Buffer A\n** Sub A1\nBody A")
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
// Integration: headline + drawer + planning + clock
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_drawer_planning_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Complex task\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:EFFORT: 3:00\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-14 10:00]--[2026-01-14 11:00] =>  1:00\n:END:\nBody text")
  (let* ((tree (org-element-parse-buffer))
         (headline (car (org-element-map tree 'headline (lambda (h) h))))
         (planning (car (org-element-map (org-element-contents headline) 'planning
                         (lambda (p) p))))
         (drawers (org-element-map (org-element-contents headline) 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    (list (org-element-property :todo-keyword headline)
          (org-element-property :scheduled planning)
          (org-element-property :deadline planning)
          drawers)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: table + formula + sort + transpose
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
// Integration: list + checkbox + statistics + hierarchy
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_checkbox_hierarchy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [%]\n- [ ] item 1\n  - [ ] sub 1\n  - [ ] sub 2\n- [ ] item 2\n  - [ ] sub 3\n- [ ] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    ;; Check sub 1 and sub 2
    (forward-line 2)
    (org-toggle-checkbox)
    (forward-line 1)
    (org-toggle-checkbox)
    ;; Check item 2
    (forward-line 1)
    (org-toggle-checkbox)
    (org-update-statistics-cookies t)
    (goto-char (point-min))
    (let ((h1 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
      (list h0 h1))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: property + inheritance + columns
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_inheritance_columns() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %PRIORITY %TAGS %VAR\n* Parent :work:\n:PROPERTIES:\n:VAR: parent-val\n:END:\n** Child 1\n*** Grandchild\n** Child 2 :personal:")
  (goto-char (point-min))
  (search-forward "Grandchild")
  (let ((var (org-entry-get nil "VAR" 'inherit))
        (tags (org-get-tags nil t))
        (fmt (org-columns-get-format)))
    (list var tags fmt)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: export + options + attributes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_options_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+OPTIONS: toc:nil num:nil\n#+ATTR_HTML: :id main\n* Heading\n#+ATTR_LATEX: :options [fragile]\n#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (headlines (org-element-map tree 'headline
                      (lambda (h) (org-element-property :raw-value h))))
         (blocks (org-element-map tree 'src-block
                   (lambda (b)
                     (list (org-element-property :language b)
                           (org-element-property :attr_latex b))))))
    (list (plist-get info :title)
          (plist-get info :with-toc)
          (plist-get info :with-numbers)
          headlines blocks)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: element + hierarchy + operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_hierarchy_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2a\n*** H3a\n*** H3b\n** H2b\n* H1b")
  (goto-char (point-min))
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
// Integration: clock + duration + effort
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_duration_effort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\n:PROPERTIES:\n:EFFORT: 2:00\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-15 10:00]--[2026-01-15 11:30] =>  1:30\nCLOCK: [2026-01-16 14:00]--[2026-01-16 15:00] =>  1:00\n:END:")
  (goto-char (point-min))
  (let ((effort (org-entry-get nil "EFFORT"))
        (clocked (org-clock-sum-current-entry)))
    (list effort clocked)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: link + search + radio
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_search_radio() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<<<target>>>\n\nSee target and [[file:test.org::*heading][heading]]")
  (let* ((tree (org-element-parse-buffer))
         (targets (org-element-map tree 'radio-target
                    (lambda (rt) (org-element-property :value rt))))
         (links (org-element-map tree 'link
                  (lambda (l)
                    (list (org-element-property :type l)
                          (org-element-property :search-option l))))))
    (list targets links)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Integration: planning + repeater + delay
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_repeater_delay() {
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
// Integration: block + switch + parameter
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_block_switch_parameter() {
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
// Integration: headline + all elements
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_all_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title :tag1:tag2:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:VAR: val\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody text\n** Sub heading\n- List item\n| table |\n#+BEGIN_SRC\n(+ 1 2)\n#+END_SRC")
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
