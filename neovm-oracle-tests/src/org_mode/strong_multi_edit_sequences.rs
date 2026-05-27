//! Strong multi-edit-sequences oracle tests — multi-step editing sequences.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Create document, edit headlines, reparse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_create_edit_reparse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Original\nBody")
  (let* ((tree1 (org-element-parse-buffer))
         (h1 (car (org-element-map tree1 'headline
                    (lambda (h) (org-element-property :raw-value h))))))
    (goto-char (point-min))
    (org-edit-headline "Changed")
    (let* ((tree2 (org-element-parse-buffer))
           (h2 (car (org-element-map tree2 'headline
                      (lambda (h) (org-element-property :raw-value h))))))
      (list h1 h2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Create document, add properties, reparse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_add_properties_reparse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (org-entry-put nil "A" "1")
  (org-entry-put nil "B" "2")
  (let ((p1 (org-entry-properties nil 'standard)))
    (org-entry-put nil "A" "10")
    (org-entry-delete nil "B")
    (list p1 (org-entry-properties nil 'standard))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Create table, add formula, recalculate
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_table_formula_recalc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| 1 | 2 |\n| 3 | 4 |\n#+TBLFM: $3=$1+$2")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (let ((d1 (org-table-to-lisp)))
    (org-table-put 1 1 "10")
    (org-table-recalculate 'all)
    (list d1 (org-table-to-lisp))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Create list, toggle checkboxes, update statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_list_checkbox_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [ ] a\n- [ ] b\n- [ ] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line 1)
    (org-toggle-checkbox)
    (forward-line 1)
    (org-toggle-checkbox)
    (org-update-statistics-cookies t)
    (goto-char (point-min))
    (list h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Create sparse tree, check visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_sparse_tree_visibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C\n** TODO D\n** DONE E")
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

// ═══════════════════════════════════════════════════════════════════════
// Element parse, modify, reparse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_element_modify_reparse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] T :tag:\nBody")
  (let* ((tree1 (org-element-parse-buffer))
         (h1 (car (org-element-map tree1 'headline
                    (lambda (h) (list (org-element-property :raw-value h)
                                      (org-element-property :todo-keyword h)
                                      (org-element-property :priority h)
                                      (org-element-property :tags h)))))))
    (goto-char (point-min))
    (org-edit-headline "New")
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("newtag"))
    (let* ((tree2 (org-element-parse-buffer))
           (h2 (car (org-element-map tree2 'headline
                      (lambda (h) (list (org-element-property :raw-value h)
                                        (org-element-property :todo-keyword h)
                                        (org-element-property :priority h)
                                        (org-element-property :tags h)))))))
      (list h1 h2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Export environment with options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_export_env_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+AUTHOR: Author\n#+OPTIONS: toc:2 num:t\n* H1\n** H2")
  (let* ((info (org-export-get-environment nil)))
    (list (plist-get info :title)
          (plist-get info :author)
          (plist-get info :with-toc)
          (plist-get info :with-numbers))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Link attributes with caption
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_link_attr_caption() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: Cap\n#+ATTR_HTML: :width 300\n#+NAME: fig\n[[file:img.png]]")
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
// Planning repeater delay
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_planning_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO W\nSCHEDULED: <2026-01-15 +1w -3d>\n* TODO M\nDEADLINE: <2026-01-20 +1m -1w>")
  (org-element-map (org-element-parse-buffer) 'planning
    (lambda (p)
      (let ((s (org-element-property :scheduled p))
            (d (org-element-property :deadline p)))
        (list (when s (list (org-element-property :repeater-type s)
                            (org-element-property :repeater-value s)
                            (org-element-property :repeater-unit s)))
              (when d (list (org-element-property :repeater-type d)
                            (org-element-property :repeater-value d)
                            (org-element-property :repeater-unit d)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Timestamp range repeater
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_timestamp_repeater() {
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
            (org-element-property :minute-start t)
            (org-element-property :year-end t)
            (org-element-property :day-end t)
            (org-element-property :repeater-type t)
            (org-element-property :repeater-value t)
            (org-element-property :repeater-unit t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Drawer multiple types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_drawer_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\n:LOGBOOK:\n- Note\n:END:\n:MYDRAWER:\n- Data\n:END:\nBody")
  (org-element-map (org-element-parse-buffer) 'drawer
    (lambda (d) (org-element-property :drawer-name d))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Block switches params
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_block_switches() {
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
// Footnote multi ref
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_footnote_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2] again[fn:1]\n\n[fn:1] First\n[fn:2] Second")
  (let* ((tree (org-element-parse-buffer))
         (fn (org-element-map tree 'footnote-reference
               (lambda (f) (org-element-property :label f))))
         (fd (org-element-map tree 'footnote-definition
               (lambda (d) (org-element-property :label d)))))
    (list fn fd)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Link search options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_link_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[file:test.org::*heading][h]] and [[file:test.org::#custom-id][id]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l)
      (list (org-element-property :type l)
            (org-element-property :path l)
            (org-element-property :search-option l)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Affiliated keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_affiliated() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: Cap\n#+ATTR_HTML: :width 300\n#+ATTR_LATEX: :width 0.5\\textwidth\n#+NAME: fig\n#+RESULTS: res\n[[file:img.png]]")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l))))
         (p (org-element-property :parent link)))
    (list (org-element-type p)
          (org-element-property :caption p)
          (org-element-property :attr_html p)
          (org-element-property :attr_latex p)
          (org-element-property :name p))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Element map with info
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_element_map_info() {
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
// Property inheritance deep
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_property_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PROPERTY: VAR 1\n* L1\n:PROPERTIES:\n:VAR: 2\n:END:\n** L2\n*** L3\n:PROPERTIES:\n:VAR: 3\n:END:\n**** L4")
  (goto-char (point-min))
  (search-forward "L4")
  (list (org-entry-get nil "VAR" 'inherit)
        (org-entry-get nil "VAR" nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Outline path deep
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_outline_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P\n** T1\n*** S1\n**** SS1\n***** SSS1")
  (goto-char (point-min))
  (search-forward "SSS1")
  (list (org-get-outline-path)
        (org-current-level)
        (org-get-heading t t t t)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi buffer parse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_multi_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((r '()))
  (with-temp-buffer
    (org-mode)
    (insert "* A\n** A1\nBodyA")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h))) r))
  (with-temp-buffer
    (org-mode)
    (insert "* B\n** B1\n** B2\nBodyB")
    (push (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (org-element-property :raw-value h))) r))
  (nreverse r))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Deferred chain 5 ops
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_deferred_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Orig :tag:\n:PROPERTIES:\n:VAR: val\n:END:\nBody")
  (goto-char (point-min))
  (let* ((el (org-element-at-point))
         (p1 (list :todo (org-element-property :todo-keyword el)
                   :pri (org-element-property :priority el)
                   :tags (org-element-property :tags el)
                   :var (org-entry-get nil "VAR")
                   :title (org-element-property :raw-value el))))
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("newtag"))
    (org-entry-put nil "VAR" "newval")
    (org-edit-headline "Changed")
    (let* ((el2 (org-element-at-point))
           (p2 (list :todo (org-element-property :todo-keyword el2)
                     :pri (org-element-property :priority el2)
                     :tags (org-element-property :tags el2)
                     :var (org-entry-get nil "VAR")
                     :title (org-element-property :raw-value el2))))
      (list p1 p2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Agenda custom keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_agenda_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (setq org-todo-keywords '((sequence "IDEA" "WORKING" "DONE")))
  (insert "* IDEA F1\n* WORKING F2\n* DONE F3")
  (org-map-entries
    (lambda () (list (org-get-heading t t t t) (org-get-todo-state)))
    nil 'file))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Refile targets levels
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_refile_levels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P1\n** T1\n*** S1\n* P2\n** T2")
  (let ((targets (org-refile-get-targets nil)))
    (mapcar (lambda (t) (list (car t) (cdr t))) targets)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Colview effort
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_colview_effort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %EFFORT\n* TODO Task 1\n:PROPERTIES:\n:EFFORT: 2h\n:END:\n* DONE Task 2\n:PROPERTIES:\n:EFFORT: 30m\n:END:")
  (goto-char (point-min))
  (org-columns-get-format))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Block execution
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_block_exec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (block (car (org-element-map tree 'src-block (lambda (b) b)))))
    (list (org-element-property :language block)
          (org-element-property :value block))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Clock sum
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_clock_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\n:END:\n* B\n:LOGBOOK:\nCLOCK: [2026-01-11 14:00]--[2026-01-11 15:30] =>  1:30\n:END:")
  (org-clock-sum))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Entity replacement
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_entity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\alpha \\beta \\gamma")
  (let ((before (buffer-string)))
    (org-toggle-pretty-entities)
    (list before (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Radio targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_radio() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<<<target>>>\nSee target here")
  (let* ((tree (org-element-parse-buffer))
         (targets (org-element-map tree 'radio-target
                    (lambda (rt) (org-element-property :value rt)))))
    targets))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Structure template
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_structure() {
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
// Dynamic block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_dynamic() {
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
// Macro expansion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_macro() {
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
// Comment fixed
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_comment_fixed() {
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
// Sort entries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_sort() {
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
// Clone subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_clone() {
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
// Copy paste subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_copy_paste() {
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
// Move subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_move() {
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
// Promote demote subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_promote_demote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3")
  (goto-char (point-min))
  (forward-line 1)
  (org-promote-subtree)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'headline
              (lambda (h) (list (org-element-property :level h)
                                (org-element-property :raw-value h))))))
    (org-demote-subtree)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'headline
                (lambda (h) (list (org-element-property :level h)
                                  (org-element-property :raw-value h))))))
      (list d1 d2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Mark narrow end subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_mark_narrow_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub\n* H1b")
  (goto-char (point-min))
  (org-mark-subtree)
  (let ((m (mark)) (p (point)))
    (list (< p m) p m)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Dblock update
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_dblock() {
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
// Macro replace
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_macro_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: g H $1!\n{{{g(A)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Try structure
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_try_structure() {
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
// Update statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_update_stats() {
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
// Sparse tree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn mes_sparse_tree() {
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
