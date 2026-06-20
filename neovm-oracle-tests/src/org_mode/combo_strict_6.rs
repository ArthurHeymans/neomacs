//! Combo-strict-6 oracle tests — deep cross-subsystem combos and strict
//! property/API contract verification.
//!
//! Exercises babel multisession, columns, outline, attachment links,
//! agenda-style mapping, planning mutations, drawer manipulation, radio
//! targets, macro expansion, and multi-table remote references together.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Babel multi-language pipeline and session persistence
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_babel_multilang_pipeline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-emacs-lisp)
  (require 'ob-sh)
  (let ((org-mode-hook nil)
        (org-confirm-babel-evaluate nil))
    (with-temp-buffer (org-mode)
      (insert "* Pipeline\n")
      (insert "#+name: step1\n")
      (insert "#+begin_src emacs-lisp :results value\n(+ 10 20 30)\n#+end_src\n\n")
      (insert "#+begin_src emacs-lisp :results value :var x=step1\n(* x 2)\n#+end_src\n\n")
      (insert "#+begin_src sh :results output :var y=step1\n")
      ;; for sh src block the :results body varies; just make sure it runs
      ;; without error. We capture the :stdin value.
      (insert "echo \"input: $y\"\n")
      (insert "#+end_src\n")
      (let ((r '()))
        (goto-char (point-min))
        ;; execute first block
        (search-forward "#+begin_src emacs-lisp")
        (push (org-babel-execute-src-block) r)
        ;; execute second block that depends on first
        (search-forward "#+begin_src emacs-lisp")
        (push (org-babel-execute-src-block) r)
        ;; execute sh block that depends on first
        (search-forward "#+begin_src sh")
        (push (org-babel-execute-src-block) r)
        (nreverse r)))))"##,
    );
}

#[test]
fn strict_babel_results_collection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-emacs-lisp)
  (let ((org-mode-hook nil)
        (org-confirm-babel-evaluate nil))
    (with-temp-buffer (org-mode)
      (insert "* Babel Results Test\n")
      (insert "#+begin_src emacs-lisp :results output\n(princ \"line1\\nline2\\nline3\")\n#+end_src\n")
      (goto-char (point-min))
      (search-forward "#+begin_src emacs-lisp")
      (let* ((result (org-babel-execute-src-block))
             (tree (org-element-parse-buffer))
             (srcs (org-element-map tree 'src-block #'identity))
             (results (org-element-map tree 'result #'identity)))
        (list
         ;; number of src-blocks
         (length srcs)
         ;; number of result blocks
         (length results)
         ;; result contents
         (when (car results)
           (substring-no-properties
            (buffer-substring-no-properties
             (org-element-property :contents-begin (car results))
             (org-element-property :contents-end (car results))))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Column view and property API
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_columns_view_property_extract() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-colview)
  (let ((org-mode-hook nil)
        (org-columns-default-format "%ITEM %TODO %PRIORITY %TAGS %EFFORT{:} %CLOCKSUM"))
    (with-temp-buffer (org-mode)
      (insert "* TODO [#A] Task1 :work:\n")
      (insert ":PROPERTIES:\n:EFFORT:   2:00\n:END:\n")
      (insert "** DONE Task2 :home:personal:\n")
      (insert ":PROPERTIES:\n:EFFORT:   1:30\n:END:\n")
      (insert "** TODO [#B] Task3 :work:\n")
      (insert ":PROPERTIES:\n:EFFORT:   1:00\n:END:\n")
      (insert "* TODO [#B] Task4 :personal:\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (headlines (org-element-map tree 'headline #'identity)))
        (list
         ;; count
         (length headlines)
         ;; todo keywords
         (mapcar (lambda (h) (org-element-property :todo-keyword h)) headlines)
         ;; priorities
         (mapcar (lambda (h) (org-element-property :priority h)) headlines)
         ;; effort property
         (mapcar (lambda (h) (org-entry-get (org-element-property :begin h) "EFFORT")) headlines)
         ;; raw values
         (mapcar (lambda (h) (substring-no-properties (org-element-property :raw-value h))) headlines))))))"##,
    );
}

#[test]
fn strict_property_api_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n* H2\n")
      (goto-char (point-min))
      (org-entry-put nil "Alpha" "value1")
      (org-entry-put nil "Beta"  "42")
      (org-entry-put nil "Gamma" "multi word value")
      (org-forward-heading-same-level 1)
      (org-entry-put nil "Alpha" "value2")
      (org-entry-put nil "Delta" "inherited?")
      (list
       ;; H1 properties
       (progn (goto-char (point-min))
              (list (org-entry-get nil "Alpha")
                    (org-entry-get nil "Beta")
                    (org-entry-get nil "Gamma")
                    (org-entry-get nil "Delta")))
       ;; H2 properties
       (progn (org-forward-heading-same-level 0)
              (list (org-entry-get nil "Alpha")
                    (org-entry-get nil "Delta")
                    (org-entry-get nil "Missing")))
       ;; H1 with inheritance
       (progn (goto-char (point-min))
              (org-entry-get nil "Delta" t))
       ;; Delete property
       (progn (org-entry-delete nil "Beta")
              (org-entry-get nil "Beta"))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Outline path, navigation, and structure editing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_outline_path_deep_hierarchy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* A\n** A1\n*** A1a\n**** A1a1\n***** A1a1a\n** A2\n*** A2a\n* B\n** B1\n*** B1a\n")
      (let ((r '()))
        (goto-char (point-min))
        ;; Outermost
        (push (list :top (org-get-outline-path)) r)
        ;; Deepest A1a1a
        (search-forward "A1a1a")
        (beginning-of-line)
        (push (list :deep-a (org-get-outline-path)) r)
        ;; B1a
        (search-forward "B1a")
        (beginning-of-line)
        (push (list :deep-b (org-get-outline-path)) r)
        ;; A2a
        (search-forward "A2a")
        (beginning-of-line)
        (push (list :mid (org-get-outline-path)) r)
        (nreverse r)))))"##,
    );
}

#[test]
fn strict_structure_edit_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* A\n** B\n** C\n* D\n* E\n")
      (let ((snapshots '()))
        (push (list :initial (mapcar (lambda (h) (substring-no-properties (org-element-property :raw-value h)))
                                     (org-element-map (org-element-parse-buffer) 'headline #'identity)))
              snapshots)
        ;; Move B (currently at position of first ** B) after D
        (goto-char (point-min))
        (forward-line 1)  ;; on ** B
        (org-metadown)
        (org-metadown)
        (org-metadown)
        (push (list :after-metadown
                    (mapcar (lambda (h) (substring-no-properties (org-element-property :raw-value h)))
                            (org-element-map (org-element-parse-buffer) 'headline #'identity)))
              snapshots)
        ;; Promote C (now the first child of A)
        (goto-char (point-min))
        (search-forward "C")
        (beginning-of-line)
        (org-metaleft)
        (push (list :after-promote
                    (mapcar (lambda (h) (list (org-element-property :level h)
                                              (substring-no-properties
                                               (org-element-property :raw-value h))))
                            (org-element-map (org-element-parse-buffer) 'headline #'identity)))
              snapshots)
        ;; Demote D to be under C (M-right)
        (goto-char (point-min))
        (search-forward "\n* D")
        (forward-char 1)
        (org-metaright)
        (org-metaright)
        (push (list :after-demote
                    (mapcar (lambda (h) (list (org-element-property :level h)
                                              (substring-no-properties
                                               (org-element-property :raw-value h))))
                            (org-element-map (org-element-parse-buffer) 'headline #'identity)))
              snapshots)
        ;; Final buffer content
        (push (list :buffer (buffer-substring-no-properties (point-min) (point-max))) snapshots)
        (nreverse snapshots))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Radio target + link resolution cross-check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_radio_target_link_resolution() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Target Section\n")
      (insert "<<<radio-target-one>>> This is a paragraph about radio-target-one.\n")
      (insert "<<<other-target>>> Another paragraph about other-target.\n\n")
      (insert "* Link Section\n")
      (insert "Refer to [[radio-target-one]] and foreign-target.\n")
      (insert "Also check <<<foreign-target>> here.\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (targets (org-element-map tree 'target #'identity))
             (links (org-element-map tree 'link #'identity))
             (radio-targets (org-element-map tree 'radio-target #'identity)))
        (list
         ;; Count targets
         (list :targets (length targets))
         ;; Count radio targets
         (list :radio-targets (length radio-targets))
         ;; Count links
         (list :links (length links))
         ;; Target raw values
         (list :target-values (mapcar (lambda (tgt)
                                       (substring-no-properties
                                        (buffer-substring-no-properties
                                         (org-element-property :begin tgt)
                                         (org-element-property :end tgt))))
                                     targets))
         ;; Link paths
         (list :link-paths (mapcar (lambda (l) (org-element-property :path l)) links))
         ;; Link types
         (list :link-types (mapcar (lambda (l) (org-element-property :type l)) links)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Macro expansion in complex document context
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_macro_expansion_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+MACRO: author John Doe\n")
      (insert "#+MACRO: date 2024\n")
      (insert "#+MACRO: proj (eval (upcase \"myproject\"))\n")
      (insert "\n* {{{author}}} Report — {{{proj}}} ({{{date}}})\n")
      (insert "Written by {{{author}}}. Published {{{date}}}.\n")
      (insert "#+begin_src emacs-lisp :results value :exports both\n")
      (insert ";; Author: {{{author}}}, Project: {{{proj}}}\n(+ 1 2)\n")
      (insert "#+end_src\n")
      (goto-char (point-min))
      (let* ((orig (buffer-substring-no-properties (point-min) (point-max)))
             (tree (org-element-parse-buffer))
             (interpreted (substring-no-properties (org-element-interpret-data tree))))
        (list
         ;; Original buffer (macros unexpanded)
         (list :original-has-macro (string-match-p "{{{" orig))
         ;; Interpreted data (macros expanded)
         (list :interpreted-has-john (string-match-p "John Doe" interpreted))
         (list :interpreted-has-date (string-match-p "2024" interpreted))
         ;; Element types present
         (list :types (delete-dups (mapcar #'org-element-type
                                           (org-element-map tree t #'identity))))
         ;; Headline raw value after interpretation
         (list :headline-value (substring-no-properties
                                (org-element-property :raw-value
                                 (car (org-element-map tree 'headline #'identity)))))
         ;; Source block value preserved
         (list :src-value (org-element-property :value
                            (car (org-element-map tree 'src-block #'identity)))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Table with remote references and multi-step formula recompute
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_table_remote_references() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+name: table-a\n")
      (insert "| x | y |\n| 1 | 2 |\n| 3 | 4 |\n\n")
      (insert "#+name: table-b\n")
      (insert "| sum | product |\n|     |         |\n")
      (insert "#+TBLFM: @2$1=vsum(remote(table-a, @2$1..@3$1))::@2$2=vprod(remote(table-a, @2$2..@3$2))\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (tables (org-element-map tree 'table #'identity))
             (cells (org-element-map tree 'table-cell #'identity)))
        (list
         ;; Number of tables
         (list :tables (length tables))
         ;; Number of cells in table-a
         (list :cells-table-a (length (org-element-map (nth 0 tables) 'table-cell #'identity)))
         ;; Number of cells in table-b
         (list :cells-table-b (length (org-element-map (nth 1 tables) 'table-cell #'identity)))
         ;; Table-a first row cell values
         (let ((rows (org-element-map (nth 0 tables) 'table-row #'identity))
               (vals '()))
           (dolist (row rows)
             (let ((cs (org-element-map row 'table-cell #'identity)))
               (dolist (c cs)
                 (push (substring-no-properties
                        (org-element-interpret-data (org-element-contents c)))
                       vals))))
           (list :table-a-vals (nreverse vals)))
         ;; Table-b after recalc
         (progn (search-forward "table-b")
                (forward-line 2)
                (org-table-recalculate t)
                (org-table-align)
                (list :table-b-recalc (buffer-substring-no-properties
                                       (org-element-property :begin (nth 1 tables))
                                       (org-element-property :end (nth 1 tables))))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Attachment link creation and extraction
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_attachment_api() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-attach)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Task with attachment\n")
      (goto-char (point-min))
      (let ((attach-dir (org-attach-dir t))
            (r '()))
        (push (list :attach-dir-exists-p (file-directory-p attach-dir)) r)
        (push (list :attach-dir attach-dir) r)
        ;; Write a test file into the attach dir
        (let ((f (expand-file-name "test.txt" attach-dir)))
          (with-temp-file f (insert "test content"))
          (push (list :test-file-exists-p (file-exists-p f)) r)
          (push (list :attached-files (mapcar #'file-name-nondirectory
                                              (org-attach-file-list attach-dir))) r)
          ;; Check that org-attach-open can find it
          (push (list :org-attach-open
                      (condition-case nil
                          (progn (org-attach-open "test.txt") t)
                        (error 'error)))
                r))
        (nreverse r)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Sparse tree building and tag matching
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_sparse_tree_tag_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-tags-column -80))
    (with-temp-buffer (org-mode)
      (insert "* H1 :work:urgent:\n** H1a :work:\n** H1b :urgent:\n")
      (insert "* H2 :home:\n** H2a :home:urgent:\n* H3 :personal:work:\n")
      (goto-char (point-min))
      (let ((r '()))
        ;; Match :work:
        (org-match-sparse-tree nil "work")
        (push (list :match-work
                    (mapcar (lambda (h) (substring-no-properties (org-element-property :raw-value h)))
                            (org-element-map (org-element-parse-buffer) 'headline #'identity)))
              r)
        (org-remove-occur-highlights)
        ;; Match :urgent:
        (org-match-sparse-tree nil "urgent")
        (push (list :match-urgent
                    (mapcar (lambda (h) (substring-no-properties (org-element-property :raw-value h)))
                            (org-element-map (org-element-parse-buffer) 'headline #'identity)))
              r)
        (org-remove-occur-highlights)
        ;; Match work-urgent
        (org-match-sparse-tree nil "work-urgent")
        (push (list :match-work-minus-urgent
                    (mapcar (lambda (h) (substring-no-properties (org-element-property :raw-value h)))
                            (org-element-map (org-element-parse-buffer) 'headline #'identity)))
              r)
        (nreverse r)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Planning and clock interaction
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_planning_clock_interaction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (let ((org-mode-hook nil)
        (org-clock-persist nil))
    (with-temp-buffer (org-mode)
      (insert "* Task\n")
      (goto-char (point-min))
      (let ((r '()))
        ;; Schedule
        (org-schedule nil "<2024-06-01 Sat>")
        (push (list :after-schedule
                    (org-element-map (org-element-parse-buffer) 'planning
                      (lambda (p) (when (org-element-property :scheduled p) "scheduled"))))
              r)
        ;; Set deadline
        (org-deadline nil "<2024-06-15 Sat>")
        (push (list :after-deadline
                    (org-element-map (org-element-parse-buffer) 'planning
                      (lambda (p) (list
                                   (when (org-element-property :scheduled p) "scheduled")
                                   (when (org-element-property :deadline p) "deadline")))))
              r)
        ;; Clock in then out
        (org-clock-in nil)
        (push (list :clocking-in-p (org-clocking-p)) r)
        (org-clock-out nil nil)
        (push (list :clocking-p-after-out (org-clocking-p)) r)
        ;; Verify logbook has one clock entry
        (push (list :clock-count (length (org-element-map (org-element-parse-buffer) 'clock #'identity))) r)
        ;; Clock another entry
        (org-clock-in nil)
        (org-clock-out nil nil)
        (push (list :clock-count-2 (length (org-element-map (org-element-parse-buffer) 'clock #'identity))) r)
        ;; Clock-sum
        (push (list :clock-minutes (org-clock-sum-current-item)) r)
        (nreverse r)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Agenda mapping across buffer entries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_agenda_buffer_mapping() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* TODO Task A\n")
      (insert "  SCHEDULED: <2024-07-01 Mon>\n")
      (insert "  :PROPERTIES:\n  :CATEGORY: cat-A\n  :EFFORT:   1:00\n  :END:\n\n")
      (insert "* DONE Task B\n")
      (insert "  CLOSED: [2024-06-15 Sun]\n")
      (insert "  :PROPERTIES:\n  :CATEGORY: cat-B\n  :END:\n\n")
      (insert "* TODO Task C\n")
      (insert "  DEADLINE: <2024-07-15 Mon>\n")
      (insert "  :PROPERTIES:\n  :CATEGORY: cat-A\n  :EFFORT:   0:30\n  :END:\n")
      (goto-char (point-min))
      (let ((r '()))
        ;; org-map-entries: get TODO keywords only
        (push (org-map-entries
               (lambda () (org-get-todo-state)))
              r)
        ;; org-map-entries: get headings with CATEGORY and TODO
        (push (org-map-entries
               (lambda () (list (org-get-heading t t t t)
                                (org-get-category)
                                (org-get-todo-state))))
              r)
        ;; org-map-entries: match only TODO items
        (push (org-map-entries
               (lambda () (org-get-heading t t t t))
               "TODO=\"TODO\"")
              r)
        (nreverse r)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Strict: org-element-set-element in deep tree, reparse, verify
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_set_element_deep_reparse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-element)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* A\n** B\nParagraph in B.\n*** C\nAnother paragraph.\n* D\nFinal.\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (sections (org-element-map tree 'section #'identity))
             (original-first-section (car sections)))
        (list
         ;; Counts before
         (list :headlines-before (length (org-element-map tree 'headline #'identity)))
         (list :sections-before (length sections))
         ;; Replace first section with a new paragraph
         (let ((new-para (org-element-create 'paragraph nil "Replaced paragraph.")))
           (org-element-set-element original-first-section new-para))
         ;; Counts after replacement (section content is now paragraph)
         (list :sections-after (length (org-element-map tree 'section #'identity)))
         (list :paragraphs-after (length (org-element-map tree 'paragraph #'identity)))
         ;; Interpret round-trip
         (list :interpreted (substring-no-properties (org-element-interpret-data tree))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Strict: draw-or-files extraction and manipulation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_drawer_insert_extract() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Task\n")
      (goto-char (point-min))
      ;; Create property drawer
      (org-insert-property-drawer)
      (org-entry-put nil "STATUS" "in-progress")
      (org-entry-put nil "ASSIGNEE" "user")
      ;; Insert logbook drawer
      (org-insert-drawer nil "LOGBOOK")
      (let ((pos (point)))
        (insert (format "CLOCK: [2024-01-01 Mon 10:00]--[2024-01-01 Mon 11:00] =>  1:00\n"))
        (insert "Note taken on 2024-01-01.\n")
        (let ((r '()))
          (push (list :buffer (buffer-substring-no-properties (point-min) (point-max))) r)
          ;; Count drawers
          (push (list :drawer-count
                      (length (org-element-map (org-element-parse-buffer) 'drawer #'identity)))
                r)
          ;; Count property drawers
          (push (list :prop-drawer-count
                      (length (org-element-map (org-element-parse-buffer) 'property-drawer #'identity)))
                r)
          ;; Get properties
          (push (list :status (org-entry-get nil "STATUS")) r)
          (push (list :assignee (org-entry-get nil "ASSIGNEE")) r)
          (nreverse r)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Strict: keyword parsing, including affiliated keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_keywords_affiliated() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: My Document\n")
      (insert "#+AUTHOR: Author Name\n")
      (insert "#+DATE: 2024-08-01\n")
      (insert "#+OPTIONS: num:nil toc:nil\n")
      (insert "#+FILETAGS: :project:\n")
      (insert "#+CAPTION: A famous quote\n")
      (insert "#+NAME: my-quote\n")
      (insert "#+ATTR_HTML: :class quote-class\n")
      (insert "#+BEGIN_QUOTE\nBe excellent to each other.\n#+END_QUOTE\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (keywords (org-element-map tree 'keyword #'identity)))
        (list
         ;; Number of keywords
         (list :key-count (length keywords))
         ;; Key names
         (list :keys (mapcar (lambda (k) (org-element-property :key k)) keywords))
         ;; Key values
         (list :values (mapcar (lambda (k) (org-element-property :value k)) keywords))
         ;; Quote block with affiliated keywords
         (let ((qb (car (org-element-map tree 'quote-block #'identity))))
           (list :quote-caption
                 (when qb (org-element-property :caption qb))
                 :quote-name
                 (when qb (org-element-property :name qb)))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Strict: export with custom filters and transcoders
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_export_custom_transcoders() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ox-latex)
  (let ((org-mode-hook nil)
        (org-export-with-toc nil)
        (org-export-with-author nil)
        (org-export-with-date nil))
    (with-temp-buffer (org-mode)
      (insert "* Section 1\n")
      (insert "Text with *bold* and /italic/ markup.\n\n")
      (insert "| Header1 | Header2 |\n|---------+--------|\n")
      (insert "| Value1  | Value2  |\n")
      (goto-char (point-min))
      (let* ((exported (org-export-as 'latex nil nil t))
             (tree (org-element-parse-buffer)))
        (list
         ;; Export succeeds non-empty
         (list :export-nonempty (> (length exported) 0))
         ;; Contains section
         (list :has-section1 (string-match-p "Section 1" exported))
         ;; Contains LaTeX markup for bold
         (list :has-bold (string-match-p "textbf" exported))
         ;; Contains table markup
         (list :has-table (string-match-p "tabular" exported))
         ;; Element-level: count sections
         (list :sections (length (org-element-map tree 'section #'identity)))
         ;; Element-level: count tables
         (list :tables (length (org-element-map tree 'table #'identity))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Strict: checkbox hierarchy with nested checkboxes and statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_checkbox_hierarchy_statistics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Tasks [%]\n")
      (insert "- [/] Top\n")
      (insert "  - [X] Child 1\n")
      (insert "  - [ ] Child 2\n")
      (insert "    - [X] Grandchild 1\n")
      (insert "    - [X] Grandchild 2\n")
      (insert "  - [ ] Child 3\n")
      (goto-char (point-min))
      (let ((r '()))
        ;; Initial stats
        (org-update-statistics-cookies t)
        (push (list :after-update (buffer-substring-no-properties (point-min) (point-max))) r)
        ;; Check elements
        (push (list :num-items (length (org-element-map (org-element-parse-buffer) 'item #'identity))) r)
        ;; Toggle Child 2
        (goto-char (point-min))
        (search-forward "Child 2")
        (beginning-of-line)
        (org-toggle-checkbox)
        (org-update-statistics-cookies t)
        (push (list :after-toggle-child2 (buffer-substring-no-properties (point-min) (point-max))) r)
        ;; Toggle Grandchild 1
        (goto-char (point-min))
        (search-forward "Grandchild 1")
        (beginning-of-line)
        (org-toggle-checkbox)
        (org-update-statistics-cookies t)
        (push (list :after-toggle-gc1 (buffer-substring-no-properties (point-min) (point-max))) r)
        (nreverse r)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Strict: timestamp comparison and range math
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_timestamp_range_math() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Event\n<2024-01-15 Mon>--<2024-01-20 Sat>\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (when ts
          (list
           ;; Timestamp type: active or active-range
           (list :type (org-element-property :type ts))
           ;; Start
           (list :year-start (org-element-property :year-start ts))
           (list :month-start (org-element-property :month-start ts))
           (list :day-start (org-element-property :day-start ts))
           ;; End (range)
           (list :year-end (org-element-property :year-end ts))
           (list :month-end (org-element-property :month-end ts))
           (list :day-end (org-element-property :day-end ts))
           ;; Has repeater?
           (list :repeater-type (org-element-property :repeater-type ts))
           ;; Format
           (list :formatted (org-timestamp-format ts "%Y-%m-%d"))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Strict: citation parsing with oc and oc-basic
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strict_citation_parse_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil)
        (org-cite-global-bibliography nil))
    (with-temp-buffer (org-mode)
      (insert "See [cite:@doe2024 for details].\n")
      (insert "Also [cite:see @smith2023; @jones2024, pp. 10-15].\n")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (citations (org-element-map tree 'citation #'identity))
             (refs (org-element-map tree 'citation-reference #'identity)))
        (list
         ;; Citation count
         (list :citation-count (length citations))
         ;; Reference count
         (list :ref-count (length refs))
         ;; Reference keys
         (list :ref-keys (mapcar (lambda (r) (org-element-property :key r)) refs))
         ;; Prefix/suffix
         (let ((r1 (car refs)))
           (list :ref1-prefix (when r1 (substring-no-properties
                                        (or (org-element-interpret-data
                                             (org-element-property :prefix r1)) "")))
                 :ref1-suffix (when r1 (substring-no-properties
                                        (or (org-element-interpret-data
                                             (org-element-property :suffix r1)) ""))))))))))"##,
    );
}
