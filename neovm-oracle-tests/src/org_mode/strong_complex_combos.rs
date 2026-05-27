//! Strong complex combo oracle tests — multi-operation sequences.
//!
//! Every test returns concrete structured data (lists, plists, numbers,
//! strings) to surface real divergences between Neomacs and GNU Emacs.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: headline + tags + priority + todo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_full_metadata_edit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Original :oldtag:")
  (goto-char (point-min))
  (let ((m1 (list (org-get-todo-state) (org-get-priority (char-after))
                  (org-get-tags nil t) (org-get-heading t t t t))))
    (org-todo 'right)
    (let ((m2 (list (org-get-todo-state) (org-get-priority (char-after))
                    (org-get-tags nil t) (org-get-heading t t t t))))
      (org-priority 'down)
      (org-set-tags '("newtag"))
      (org-edit-headline "New Title")
      (let ((m3 (list (org-get-todo-state) (org-get-priority (char-after))
                      (org-get-tags nil t) (org-get-heading t t t t))))
        (list m1 m2 m3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: property + planning + body
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_planning_body_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Test\nBody text here")
  (goto-char (point-min))
  (org-entry-put nil "CUSTOM_ID" "my-id")
  (org-entry-put nil "CATEGORY" "work")
  (org-deadline nil "2026-03-01")
  (org-schedule nil "2026-02-15")
  (let ((d1 (list (org-entry-get nil "CUSTOM_ID")
                  (org-entry-get nil "CATEGORY")
                  (org-entry-get nil "DEADLINE")
                  (org-entry-get nil "SCHEDULED")
                  (org-element-property :raw-value (org-element-at-point)))))
    (org-entry-delete nil "CATEGORY")
    (org-deadline nil nil)  ; remove deadline
    (let ((d2 (list (org-entry-get nil "CUSTOM_ID")
                    (org-entry-get nil "CATEGORY")
                    (org-entry-get nil "DEADLINE")
                    (org-entry-get nil "SCHEDULED"))))
      (list d1 d2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: table formula + sort + transpose
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
    (org-table-sort-lines nil ?N)  ; sort by first column numeric
    (let ((d2 (org-table-to-lisp)))
      (org-table-transpose)
      (let ((d3 (org-table-to-lisp)))
        (list d1 d2 d3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: list structure + checkbox + statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_checkbox_statistics_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [%]\n- [ ] item 1\n- [ ] item 2\n  - [ ] sub 1\n  - [ ] sub 2\n- [ ] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h1 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line 1)
    (org-toggle-checkbox)  ; check item 1
    (forward-line 2)
    (org-toggle-checkbox)  ; check sub 1
    (forward-line 1)
    (org-toggle-checkbox)  ; check sub 2
    (org-update-statistics-cookies t)
    (goto-char (point-min))
    (let ((h2 (buffer-substring-no-properties (line-beginning-position) (line-end-position)))
          (items '()))
      (forward-line 1)
      (dotimes (_ 5)
        (push (org-get-at-bol 'org-checkbox-stat) items)
        (forward-line))
      (list h1 h2 (nreverse items)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: footnote create + edit + renumber
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_create_renumber_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:2] more[fn:1] end\n\n[fn:1] First\n[fn:2] Second")
  (goto-char (point-min))
  (let ((before (buffer-string)))
    (org-footnote-action 'sort)
    (let ((after (buffer-string))
          (count (count-matches "\\[fn:" (point-min) (point-max))))
      (list before after count))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: sparse tree + visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_visibility_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task 1\n* DONE Task 2\n* TODO Task 3\n** TODO Sub 1\n** DONE Sub 2\n* WAITING Task 4")
  (goto-char (point-min))
  (let ((all (org-map-entries (lambda () (org-get-heading t t t t)) nil 'file)))
    (org-match-sparse-tree nil "TODO")
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
      ;; Now show all
      (org-set-startup-visibility 'all)
      (let ((all-visible '()))
        (goto-char (point-min))
        (while (not (eobp))
          (let ((h (org-get-heading t t t t)))
            (when h
              (unless (get-char-property (point) 'invisible)
                (push h all-visible))))
          (forward-line))
        (list all (nreverse visible) (nreverse hidden) (nreverse all-visible))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: clock table + dynamic block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_table_dynamic_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task 1\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\nCLOCK: [2026-01-11 14:00]--[2026-01-11 15:30] =>  1:30\n:END:\n* Task 2\n:LOGBOOK:\nCLOCK: [2026-01-12 09:00]--[2026-01-12 10:00] =>  1:00\n:END:")
  (let ((before (buffer-string)))
    (goto-char (point-max))
    (insert "\n#+BEGIN: clocktable :maxlevel 2\n#+END:")
    (org-dblock-update)
    (let ((after (buffer-string))
          (has-table (org-at-table-p)))
      (list before after has-table))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: link + target + radio
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_target_radio_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\n<<my-target>>\nSee [[my-target][here]]\n<<<radio>>>")
  (goto-char (point-min))
  (let* ((tree (org-element-parse-buffer))
         (links (org-element-map tree 'link
                  (lambda (l)
                    (list (org-element-property :type l)
                          (org-element-property :path l)
                          (org-element-property :raw-link l)))))
         (targets (org-element-map tree 'target
                    (lambda (t)
                      (org-element-property :value t)))))
    (list links targets)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: export backend with filters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_backend_with_filters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Test\n* Heading\n** Sub\nBody")
    (let* ((tree (org-element-parse-buffer))
           (info (org-export-get-environment nil))
           (headlines (org-element-map tree 'headline
                        (lambda (h)
                          (list (org-element-property :level h)
                                (org-element-property :raw-value h)))))
           (title (plist-get info :title)))
      (list title headlines))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: element chain with deferred operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_deferred_operations_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Test :tag:\n:PROPERTIES:\n:VAR: val\n:END:\nBody\n** Sub")
  (goto-char (point-min))
  (let* ((el (org-element-at-point))
         (type (org-element-type el))
         (todo (org-element-property :todo-keyword el))
         (priority (org-element-property :priority el))
         (tags (org-element-property :tags el))
         (var (org-entry-get nil "VAR"))
         (level (org-element-property :level el)))
    ;; Modify in sequence
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("newtag"))
    (org-entry-put nil "VAR" "newval")
    (org-edit-headline "Changed")
    ;; Read back
    (let* ((el2 (org-element-at-point))
           (type2 (org-element-type el2))
           (todo2 (org-element-property :todo-keyword el2))
           (priority2 (org-element-property :priority el2))
           (tags2 (org-element-property :tags el2))
           (var2 (org-entry-get nil "VAR"))
           (title2 (org-element-property :raw-value el2)))
      (list type todo priority tags var level
            type2 todo2 priority2 tags2 var2 title2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: multi-buffer with shared state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_buffer_shared_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((results '()))
  (with-temp-buffer
    (org-mode)
    (insert "* Buffer A\n** Sub A1\nBody A")
    (let* ((tree (org-element-parse-buffer))
           (headlines (org-element-map tree 'headline
                        (lambda (h) (org-element-property :raw-value h)))))
      (push headlines results)))
  (with-temp-buffer
    (org-mode)
    (insert "* Buffer B\n** Sub B1\n** Sub B2\nBody B")
    (let* ((tree (org-element-parse-buffer))
           (headlines (org-element-map tree 'headline
                        (lambda (h) (org-element-property :raw-value h)))))
      (push headlines results)))
  (nreverse results))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: table with mixed types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_mixed_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| Name | Age | Active |\n|------+-----+--------|\n| Alice | 30 | yes |\n| Bob | 25 | no |\n| Charlie | 35 | yes |")
  (goto-char (point-min))
  (let ((data1 (org-table-to-lisp)))
    (org-table-sort-lines nil ?a)  ; alphabetical by first column
    (let ((data2 (org-table-to-lisp)))
      (org-table-sort-lines nil ?N)  ; numeric by second column
      (let ((data3 (org-table-to-lisp)))
        (list data1 data2 data3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: headline promotion with subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_promote_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2a\n*** H3\n** H2b\n* H1b")
  (goto-char (point-min))
  (let ((before (org-map-entries (lambda ()
                                   (list (org-current-level)
                                         (org-get-heading t t t t)))
                                 nil 'file)))
    (search-forward "H2a")
    (org-promote-subtree)
    (let ((after1 (org-map-entries (lambda ()
                                     (list (org-current-level)
                                           (org-get-heading t t t t)))
                                   nil 'file)))
      (search-forward "H3")
      (org-demote-subtree)
      (let ((after2 (org-map-entries (lambda ()
                                       (list (org-current-level)
                                             (org-get-heading t t t t)))
                                     nil 'file)))
        (list before after1 after2)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: move subtree with siblings
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_move_subtree_with_siblings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n* B\n* C\n* D")
  (goto-char (point-min))
  (let ((order1 (org-map-entries (lambda () (org-get-heading t t t t)) nil 'file)))
    (org-forward-heading-same-level 1)
    (org-move-subtree-down)
    (let ((order2 (org-map-entries (lambda () (org-get-heading t t t t)) nil 'file)))
      (org-move-subtree-up)
      (org-move-subtree-up)
      (let ((order3 (org-map-entries (lambda () (org-get-heading t t t t)) nil 'file)))
        (list order1 order2 order3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: insert heading with content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_insert_heading_with_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Original\nBody text\n** Sub")
  (goto-char (point-min))
  (end-of-line)
  (org-insert-heading-respect-content)
  (insert "New heading")
  (let ((h1 (org-map-entries (lambda () (org-get-heading t t t t)) nil 'file)))
    (org-insert-todo-heading nil)
    (insert "TODO item")
    (let ((h2 (org-map-entries (lambda () (list (org-get-todo-state) (org-get-heading t t t t))) nil 'file)))
      (list h1 h2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: logbook with state changes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_logbook_state_changes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (setq org-log-into-drawer t)
  (insert "* TODO Test task")
  (goto-char (point-min))
  (org-todo 'done)
  (let ((state1 (org-get-todo-state))
        (logbook1 (org-entry-get nil "LOGBOOK")))
    (org-todo 'todo)
    (let ((state2 (org-get-todo-state))
          (logbook2 (org-entry-get nil "LOGBOOK")))
      (list state1 state2 logbook1 logbook2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: archive subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_archive_subtree_with_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Keep\n* DONE Archive me\nBody\n** Sub")
  (goto-char (point-min))
  (search-forward "Archive me")
  (let ((before (org-map-entries (lambda () (org-get-heading t t t t)) nil 'file)))
    (org-archive-subtree)
    (let ((after (org-map-entries (lambda () (org-get-heading t t t t)) nil 'file)))
      (list before after))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: column view with dynamic columns
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_colview_dynamic_columns() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %3PRIORITY %TAGS %VAR\n* TODO [#A] Test :tag:\n:PROPERTIES:\n:VAR: value\n:END:")
  (goto-char (point-min))
  (let ((fmt (org-columns-get-format))
        (props (org-entry-properties nil 'standard)))
    (list fmt props)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: babel tangle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_babel_tangle_named_blocks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+NAME: setup\n#+BEGIN_SRC emacs-lisp\n(setq x 1)\n#+END_SRC\n\n#+NAME: compute\n#+BEGIN_SRC emacs-lisp\n(+ x 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (blocks (org-element-map tree 'src-block
                   (lambda (b)
                     (list (org-element-property :name b)
                           (org-element-property :language b)
                           (org-element-property :value b))))))
    blocks))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: timestamp range
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timestamp_range_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Meeting\n<2026-01-15 10:00-11:30>\n<2026-01-16>--<2026-01-20>")
  (goto-char (point-min))
  (let* ((tree (org-element-parse-buffer))
         (timestamps (org-element-map tree 'timestamp
                       (lambda (ts)
                         (list (org-element-property :type ts)
                               (org-element-property :year-start ts)
                               (org-element-property :month-start ts)
                               (org-element-property :day-start ts)
                               (org-element-property :hour-start ts)
                               (org-element-property :minute-start ts)
                               (org-element-property :year-end ts)
                               (org-element-property :month-end ts)
                               (org-element-property :day-end ts)
                               (org-element-property :hour-end ts)
                               (org-element-property :minute-end ts))))))
    timestamps))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: drawer with content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_with_content_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:\n:LOGBOOK:\n- Note taken on [2026-01-15] \\\\\n  Test note\n:END:\nBody")
  (goto-char (point-min))
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
// Complex combo: inline task
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_inline_task_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-inlinetask)
  (insert "Body text\n*************** TODO Inline task\n*************** END\nMore body")
  (goto-char (point-min))
  (let* ((tree (org-element-parse-buffer))
         (tasks (org-element-map tree 'headline
                  (lambda (h)
                    (when (= (org-element-property :level h) 15)
                      (list (org-element-property :raw-value h)
                            (org-element-property :todo-keyword h)
                            (org-element-property :level h)))))))
    tasks))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: entity replacement
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
// Complex combo: radio targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_radio_target_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<<<my target>>>\n\nSee my target here")
  (goto-char (point-min))
  (let* ((tree (org-element-parse-buffer))
         (targets (org-element-map tree 'radio-target
                    (lambda (rt)
                      (org-element-property :value rt)))))
    targets))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: statistics with nested checkboxes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_statistics_nested_checkboxes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [%]\n- [ ] item 1\n  - [ ] sub 1a\n  - [ ] sub 1b\n- [ ] item 2\n  - [ ] sub 2a\n- [ ] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    ;; Check sub 1a and sub 1b
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
// Complex combo: planning with repeaters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_with_repeaters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Weekly task\nSCHEDULED: <2026-01-15 Wed +1w>\nDEADLINE: <2026-01-20 Mon +1w>")
  (goto-char (point-min))
  (let ((sched (org-entry-get nil "SCHEDULED"))
        (deadline (org-entry-get nil "DEADLINE"))
        (repeat (org-get-repeat)))
    (list sched deadline repeat)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: tag inheritance
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_tag_inheritance_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (setq org-use-tag-inheritance t)
  (insert "* Parent :work:\n** Child 1\n*** Grandchild\n** Child 2 :personal:")
  (goto-char (point-min))
  (search-forward "Child 1")
  (let ((tags1 (org-get-tags nil t)))
    (search-forward "Grandchild")
    (let ((tags2 (org-get-tags nil t)))
      (search-forward "Child 2")
      (let ((tags3 (org-get-tags nil t)))
        (list tags1 tags2 tags3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: block metadata
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_block_metadata_all_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+NAME: my-block\n#+BEGIN_SRC emacs-lisp -n :results value\n(+ 1 2)\n#+END_SRC\n\n#+BEGIN_EXAMPLE\nExample text\n#+END_EXAMPLE\n\n#+BEGIN_QUOTE\nQuoted text\n#+END_QUOTE")
  (let* ((tree (org-element-parse-buffer))
         (blocks (org-element-map tree '(src-block example-block quote-block)
                   (lambda (b)
                     (list (org-element-type b)
                           (org-element-property :name b)
                           (org-element-property :language b)
                           (org-element-property :switches b)
                           (org-element-property :parameters b))))))
    blocks))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: affiliated keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_affiliated_keywords_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: My caption\n#+ATTR_HTML: :width 300px\n#+NAME: my-image\n[[file:image.png]]")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l))))
         (parent (org-element-property :parent link)))
    (list (org-element-type parent)
          (org-element-property :caption parent)
          (org-element-property :attr_html parent)
          (org-element-property :name parent))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: export options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_options_all_fields() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+AUTHOR: Author\n#+EMAIL: test@example.com\n#+DATE: 2026-01-15\n#+DESCRIPTION: Desc\n#+KEYWORDS: kw1 kw2\n#+LANGUAGE: en\n#+SELECT_TAGS: export\n#+EXCLUDE_TAGS: noexport\n#+OPTIONS: toc:nil num:nil")
  (let* ((tree (org-element-parse-buffer))
         (keywords (org-element-map tree 'keyword
                     (lambda (k)
                       (list (org-element-property :key k)
                             (org-element-property :value k))))))
    keywords))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: node property operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_node_property_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:A: 1\n:B: 2\n:C: 3\n:END:")
  (goto-char (point-min))
  (let ((all (org-entry-properties nil 'standard)))
    (org-entry-put nil "D" "4")
    (org-entry-put nil "A" "10")
    (org-entry-delete nil "B")
    (let ((modified (org-entry-properties nil 'standard)))
      (list all modified))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: outline path
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_outline_path_navigation() {
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
// Complex combo: priority operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_priority_operations_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] High\n* TODO [#B] Medium\n* TODO Low\n* TODO [#C] Critical")
  (goto-char (point-min))
  (let ((priorities '()))
    (dotimes (_ 4)
      (push (org-get-priority (char-after)) priorities)
      (forward-line))
    ;; Cycle priorities
    (goto-char (point-min))
    (org-priority 'down)
    (forward-line 1)
    (org-priority 'up)
    (forward-line 2)
    (org-priority ?B)  ; set to B
    (let ((after '()))
      (goto-char (point-min))
      (dotimes (_ 4)
        (push (org-get-priority (char-after)) after)
        (forward-line))
      (list (nreverse priorities) (nreverse after)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: edit headline with body
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_edit_headline_preserve_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Original\nBody line 1\nBody line 2\n** Sub heading")
  (goto-char (point-min))
  (org-edit-headline "Changed")
  (let ((h1 (org-get-heading t t t t))
        (body1 (buffer-substring-no-properties
                 (progn (forward-line 1) (point))
                 (progn (forward-line 2) (point)))))
    (org-edit-headline "Final")
    (goto-char (point-min))
    (let ((h2 (org-get-heading t t t t))
          (body2 (buffer-substring-no-properties
                   (progn (forward-line 1) (point))
                   (progn (forward-line 2) (point)))))
      (list h1 h2 body1 body2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: sparse tree with multiple criteria
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_multiple_criteria() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task 1 :work:\n* DONE Task 2 :personal:\n* TODO Task 3 :work:\n* WAITING Task 4\n* TODO Task 5 :work:")
  (goto-char (point-min))
  (org-match-sparse-tree nil "TODO={work}")
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
// Complex combo: table with formulas and alignment
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_formula_alignment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | 1 |   |\n| b | 2 |   |\n| c | 3 |   |\n|---+---+---|\n|   | 6 |   |\n#+TBLFM: $3=$2*2::@>$2=vsum(@2..@-1)")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (let ((data (org-table-to-lisp)))
    data))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: link types and paths
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_types_and_paths() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[file:test.org][file]] and [[https://example.com][web]] and [[id:abc123][id]] and [[elisp:(message \"hi\")][elisp]]")
  (let* ((tree (org-element-parse-buffer))
         (links (org-element-map tree 'link
                  (lambda (l)
                    (list (org-element-property :type l)
                          (org-element-property :path l)
                          (org-element-property :raw-link l))))))
    links))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: keyword parsing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_keyword_parsing_all_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+TODO: TODO DONE\n#+FILETAGS: :tag1:tag2:\n#+STARTUP: overview\n#+CATEGORY: test")
  (let* ((tree (org-element-parse-buffer))
         (keywords (org-element-map tree 'keyword
                     (lambda (k)
                       (list (org-element-property :key k)
                             (org-element-property :value k))))))
    keywords))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: comment and fixed-width
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_comment_fixed_width_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# This is a comment\n: Fixed-width\n: Another line\nNormal text\n# Another comment")
  (let* ((tree (org-element-parse-buffer))
         (comments (org-element-map tree 'comment
                     (lambda (c) (org-element-property :value c))))
         (fixed (org-element-map tree 'fixed-width
                  (lambda (f) (org-element-property :value f)))))
    (list comments fixed)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: planning with repeaters and delays
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_repeaters_delays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\nSCHEDULED: <2026-01-15 Wed +1w -3d>\nDEADLINE: <2026-01-20 Mon +1m -1w>")
  (goto-char (point-min))
  (let ((sched (org-entry-get nil "SCHEDULED"))
        (deadline (org-entry-get nil "DEADLINE"))
        (repeat (org-get-repeat)))
    (list sched deadline repeat)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Complex combo: headline with all elements
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_headline_with_all_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title :tag1:tag2:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:VAR: val\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody text\n** Sub heading\n- List item\n| table |")
  (let* ((tree (org-element-parse-buffer))
         (headline (car (org-element-map tree 'headline (lambda (h) h))))
         (children (org-element-map (org-element-contents headline)
                     (lambda (el) (org-element-type el)))))
    (list (org-element-property :todo-keyword headline)
          (org-element-property :priority headline)
          (org-element-property :tags headline)
          (org-element-property :raw-value headline)
          children)))"##,
    );
}
