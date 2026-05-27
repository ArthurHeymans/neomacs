//! Strong state-deep oracle tests — deep mutable state capture.
//!
//! These tests capture multiple pieces of mutable state after
//! operations to surface divergences. Every test returns structured
//! data, never bare booleans.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// State deep: headline + todo + tags + priority after edit
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_headline_edit_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Original :old:\nBody")
  (goto-char (point-min))
  (let ((s1 (list (org-get-heading t t t t) (org-get-todo-state)
                  (org-get-priority (char-after)) (org-get-tags nil t))))
    (org-edit-headline "Changed")
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("new"))
    (let ((s2 (list (org-get-heading t t t t) (org-get-todo-state)
                    (org-get-priority (char-after)) (org-get-tags nil t))))
      (list s1 s2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: property cycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_property_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:")
  (goto-char (point-min))
  (let ((v1 (org-entry-get nil "A")))
    (org-entry-put nil "A" "2")
    (org-entry-put nil "B" "3")
    (let ((v2 (org-entry-get nil "A"))
          (v3 (org-entry-get nil "B"))
          (p1 (org-entry-properties nil 'standard)))
      (org-entry-delete nil "A")
      (let ((v4 (org-entry-get nil "A"))
            (p2 (org-entry-properties nil 'standard)))
        (list v1 v2 v3 v4 p1 p2)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: property inheritance
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_property_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PROPERTY: V root\n* L1\n:PROPERTIES:\n:V: l1\n:END:\n** L2\n*** L3\n:PROPERTIES:\n:V: l3\n:END:")
  (goto-char (point-min))
  (search-forward "L3")
  (let ((v3i (org-entry-get nil "V" 'inherit))
        (v3 (org-entry-get nil "V" nil)))
    (search-backward "L2")
    (let ((v2i (org-entry-get nil "V" 'inherit))
          (v2 (org-entry-get nil "V" nil)))
      (search-backward "L1")
      (let ((v1i (org-entry-get nil "V" 'inherit))
            (v1 (org-entry-get nil "V" nil)))
        (list v1 v1i v2 v2i v3 v3i)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: table formula
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_table_formula() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| 1 | 2 |\n| 3 | 4 |\n#+TBLFM: $3=$1+$2")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (let ((d1 (org-table-to-lisp))
        (r1 (org-table-get 1 3))
        (r2 (org-table-get 2 3)))
    (org-table-put 1 1 "10")
    (org-table-recalculate 'all)
    (let ((d2 (org-table-to-lisp))
          (r3 (org-table-get 1 3))
          (r4 (org-table-get 2 3)))
      (list d1 r1 r2 d2 r3 r4))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: checkbox stats
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_checkbox_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [ ] a\n  - [ ] a1\n  - [ ] a2\n- [ ] b\n- [ ] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line 2)
    (org-toggle-checkbox)
    (forward-line 1)
    (org-toggle-checkbox)
    (org-update-statistics-cookies t)
    (goto-char (point-min))
    (let ((h1 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
      (forward-line 3)
      (org-toggle-checkbox)
      (org-update-statistics-cookies t)
      (goto-char (point-min))
      (let ((h2 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
        (list h0 h1 h2)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: sparse tree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_sparse_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C\n** TODO D\n** DONE E")
  (goto-char (point-min))
  (org-match-sparse-tree nil "TODO")
  (let ((vis '()) (hid '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((h (org-get-heading t t t t)))
        (when h
          (if (get-char-property (point) 'invisible)
              (push h hid) (push h vis))))
      (forward-line))
    (list (nreverse vis) (nreverse hid))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: element parse modify reparse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_element_parse_modify() {
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
// State deep: export environment
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_export_env() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: My Title\n#+AUTHOR: Author\n#+OPTIONS: toc:nil num:nil\n* H1\n** H2")
  (let* ((info (org-export-get-environment nil)))
    (list (plist-get info :title)
          (plist-get info :author)
          (plist-get info :with-toc)
          (plist-get info :with-numbers))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: link attributes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_link_attr() {
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
// State deep: planning repeaters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_planning_repeaters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO W\nSCHEDULED: <2026-01-15 +1w -3d>\n* TODO M\nDEADLINE: <2026-01-20 +1m -1w>")
  (let* ((tree (org-element-parse-buffer))
         (plan (org-element-map tree 'planning
                 (lambda (p)
                   (let ((s (org-element-property :scheduled p))
                         (d (org-element-property :deadline p)))
                     (list (when s (list (org-element-property :repeater-type s)
                                         (org-element-property :repeater-value s)
                                         (org-element-property :repeater-unit s)))
                           (when d (list (org-element-property :repeater-type d)
                                         (org-element-property :repeater-value d)
                                         (org-element-property :repeater-unit d)))))))))
    plan))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: timestamp repeater
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_timestamp_repeater() {
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
                       (org-element-property :minute-start t)
                       (org-element-property :year-end t)
                       (org-element-property :day-end t)
                       (org-element-property :repeater-type t)
                       (org-element-property :repeater-value t)
                       (org-element-property :repeater-unit t))))))
    ts))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: drawer types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_drawer_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\n:LOGBOOK:\n- Note\n:END:\n:MYDRAWER:\n- Data\n:END:\nBody")
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
// State deep: block switches
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_block_switches() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp -n :results value :exports both\n(+ 1 2)\n#+END_SRC\n\n#+BEGIN_EXAMPLE -n\nExample\n#+END_EXAMPLE")
  (let* ((tree (org-element-parse-buffer))
         (bl (org-element-map tree '(src-block example-block)
               (lambda (b)
                 (list (org-element-type b)
                       (org-element-property :language b)
                       (org-element-property :switches b)
                       (org-element-property :parameters b))))))
    bl))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: footnote markup
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_footnote_markup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] *bold* /italic/\n[fn:2] [[link][desc]]")
  (let* ((tree (org-element-parse-buffer))
         (fn (org-element-map tree 'footnote-reference
               (lambda (f) (org-element-property :label f))))
         (fd (org-element-map tree 'footnote-definition
               (lambda (d)
                 (list (org-element-property :label d)
                       (org-element-property :begin d)
                       (org-element-property :end d))))))
    (list fn fd)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: inline task
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_inline_task() {
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
                            (org-element-property :todo-keyword h)
                            (org-element-property :level h)))))))
    tasks))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: hierarchy contents
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_hierarchy_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* L1\n** L2a\n*** L3a\n*** L3b\n** L2b\n*** L3c\n* L1b\n** L2c")
  (let* ((tree (org-element-parse-buffer))
         (struct (org-element-map tree 'headline
                   (lambda (h)
                     (list (org-element-property :level h)
                           (org-element-property :raw-value h)
                           (length (org-element-contents h)))))))
    struct))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_visibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (let ((s '()))
    (org-set-startup-visibility 'overview)
    (push (list :overview
                (get-char-property (search-forward "H2") 'invisible)
                (progn (forward-line) (get-char-property (point) 'invisible)))
          s)
    (org-set-startup-visibility 'content)
    (push (list :content
                (get-char-property (search-forward "H2") 'invisible)
                (progn (forward-line) (get-char-property (point) 'invisible)))
          s)
    (org-set-startup-visibility 'all)
    (push (list :all
                (get-char-property (search-forward "H2") 'invisible)
                (progn (forward-line) (get-char-property (point) 'invisible)))
          s)
    (nreverse s)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: outline path
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_outline_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P\n** T1\n*** S1\n**** SS1\n** T2")
  (goto-char (point-min))
  (search-forward "SS1")
  (list (org-get-outline-path)
        (org-current-level)
        (org-get-heading t t t t)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: agenda entries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_agenda_entries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C\n* WAITING D")
  (org-map-entries
    (lambda ()
      (list (org-get-heading t t t t)
            (org-get-todo-state)
            (org-entry-get nil "PRIORITY")))
    nil 'file))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: colview format
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_colview_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %3PRIORITY %TAGS %V\n* TODO [#A] T :tag:\n:PROPERTIES:\n:V: val\n:END:")
  (goto-char (point-min))
  (org-columns-get-format))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: macro expansion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_macro_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greet Hello $1 and $2!\n{{{greet(Alice, Bob)}}}\n{{{greet(World, 42)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: dynamic block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_dynamic_block() {
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
// State deep: entity replacement
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_entity_replacement() {
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
// State deep: radio targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_radio_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<<<target1>>>\n<<<target2>>>\nSee target1 and target2")
  (let* ((tree (org-element-parse-buffer))
         (targets (org-element-map tree 'radio-target
                    (lambda (rt) (org-element-property :value rt)))))
    targets))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: structure template
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_structure_template() {
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
// State deep: comment fixed
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_comment_fixed() {
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
// State deep: link types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_link_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[[https://x][w]] [[file:f][f]] [[id:i][i]] [[elisp:(+ 1)][e]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l)
                      (org-element-property :path l)
                      (org-element-property :raw-link l)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+AUTHOR: A\n#+EMAIL: e\n#+OPTIONS: toc:nil")
  (org-element-map (org-element-parse-buffer) 'keyword
    (lambda (k) (list (org-element-property :key k)
                      (org-element-property :value k)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: refile targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_refile_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P1\n** T1\n* P2\n** T2")
  (mapcar 'car (org-refile-get-targets nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: pcomplete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_pcomplete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\agr")
  (length (all-completions "\\ag" (pcomplete-entries))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// State deep: sparse dates
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_sparse_dates() {
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

// ═══════════════════════════════════════════════════════════════════════
// State deep: multi-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sd_multi_buffer() {
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
