//! Strong tera combo oracle tests — ultimate multi-operation sequences.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Tera: full document lifecycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_full_lifecycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: L\n* TODO P\n** TODO T1\n** TODO T2\n* D\n** DONE S1\n** TODO S2")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (h1 (org-element-map tree 'headline
               (lambda (h) (list (org-element-property :raw-value h)
                                 (org-element-property :todo-keyword h))))))
    (goto-char (point-min))
    (search-forward "T1")
    (org-todo 'done)
    (org-set-tags '("d"))
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
// Tera: property inheritance chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PROPERTY: V 1\n* L1\n:PROPERTIES:\n:V: 2\n:END:\n** L2\n*** L3\n:PROPERTIES:\n:V: 3\n:END:")
  (goto-char (point-min))
  (search-forward "L3")
  (let ((v3i (org-entry-get nil "V" 'inherit))
        (v3 (org-entry-get nil "V" nil)))
    (org-entry-put nil "V" "3n")
    (let ((v3n (org-entry-get nil "V" nil))
          (v3ni (org-entry-get nil "V" 'inherit)))
      (search-backward "L2")
      (let ((v2i (org-entry-get nil "V" 'inherit))
            (v2 (org-entry-get nil "V" nil)))
        (list v3i v3 v3n v3ni v2i v2)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: table formula sort transpose
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_fst() {
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
// Tera: checkbox hierarchy stats
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_checkbox_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [ ] a\n  - [ ] a1\n  - [ ] a2\n- [ ] b\n  - [ ] b1\n- [ ] c")
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
// Tera: sparse tree tags visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tags_vis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1 :w:\n* T2 :p:\n* T3 :w:u:\n* T4")
  (goto-char (point-min))
  (org-match-sparse-tree nil "w")
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
// Tera: headline edit context
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_hl_edit_ctx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Orig :old:\n:PROPERTIES:\n:V: val\n:END:\nBody")
  (goto-char (point-min))
  (let ((c1 (list (org-get-heading t t t t) (org-get-todo-state)
                  (org-get-priority (char-after)) (org-get-tags nil t)
                  (org-entry-get nil "V"))))
    (org-edit-headline "New")
    (org-set-tags '("n"))
    (let ((c2 (list (org-get-heading t t t t) (org-get-todo-state)
                    (org-get-priority (char-after)) (org-get-tags nil t)
                    (org-entry-get nil "V"))))
      (list c1 c2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: clock effort property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_effort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\n:PROPERTIES:\n:EFFORT: 2:00\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-15 10:00]--[2026-01-15 11:30] =>  1:30\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "EFFORT")
        (org-clock-sum-current-entry)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: link attributes export
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_attr() {
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
// Tera: element chain mods
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_el_chain() {
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
    (org-set-tags '("n"))
    (org-entry-put nil "V" "n")
    (org-edit-headline "C")
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
// Tera: multi-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_multi_buf() {
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
// Tera: planning repeaters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_plan_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO W\nSCHEDULED: <2026-01-15 +1w -3d>\n* TODO M\nDEADLINE: <2026-01-20 +1m -1w>")
  (org-element-map (org-element-parse-buffer) 'planning
    (lambda (p)
      (let ((s (org-element-property :scheduled p))
            (d (org-element-property :deadline p)))
        (list (when s (org-element-property :repeater-type s))
              (when d (org-element-property :repeater-type d)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: block switches
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_block_sw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp -n :results value\n(+ 1 2)\n#+END_SRC\n\n#+BEGIN_EXAMPLE -n\nEx\n#+END_EXAMPLE")
  (org-element-map (org-element-parse-buffer) '(src-block example-block)
    (lambda (b)
      (list (org-element-type b)
            (org-element-property :language b)
            (org-element-property :switches b)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: timestamp range repeater
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_ts_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* M\n<2026-01-15 10:00-11:30>\n<2026-01-16>--<2026-01-20>\n<2026-01-25 +1w>")
  (org-element-map (org-element-parse-buffer) 'timestamp
    (lambda (t)
      (list (org-element-property :type t)
            (org-element-property :year-start t)
            (org-element-property :day-start t)
            (org-element-property :repeater-type t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: drawer types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\n:LOGBOOK:\n- N\n:END:\n:CUSTOM:\n- D\n:END:\nBody")
  (org-element-map (org-element-parse-buffer) 'drawer
    (lambda (d) (org-element-property :drawer-name d))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: inline task
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-inlinetask)
  (insert "B\n*************** TODO Inline\n*************** END\nM")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h)
      (when (= (org-element-property :level h) 15)
        (list (org-element-property :raw-value h)
              (org-element-property :todo-keyword h))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: entity radio
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_ent_radio() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\alpha \\beta\n<<<t>>>\nSee t")
  (let ((b (buffer-string)))
    (org-toggle-pretty-entities)
    (list b (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: outline refile
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_out_refile() {
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
// Tera: agenda colview
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_ag_col() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %PRIORITY\n* TODO [#A] T")
  (goto-char (point-min))
  (org-columns-get-format))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: pcomplete parse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_pcomp_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\agr")
  (length (all-completions "\\ag" (pcomplete-entries))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: dynamic block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_dyn_block() {
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
// Tera: structure template
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_struct_tpl() {
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
// Tera: comment fixed
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_comm_fix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# C\n: F\nN")
  (let* ((tree (org-element-parse-buffer))
         (c (org-element-map tree 'comment
              (lambda (c) (org-element-property :value c))))
         (f (org-element-map tree 'fixed-width
              (lambda (f) (org-element-property :value f)))))
    (list c f)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: affiliated kw
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_aff_kw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: C\n#+ATTR_HTML: :width 300\n#+NAME: n\n[[file:i.png]]")
  (let* ((tree (org-element-parse-buffer))
         (l (car (org-element-map tree 'link (lambda (l) l))))
         (p (org-element-property :parent l)))
    (list (org-element-property :caption p)
          (org-element-property :attr_html p)
          (org-element-property :name p))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_kw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+AUTHOR: A\n#+OPTIONS: toc:nil")
  (org-element-map (org-element-parse-buffer) 'keyword
    (lambda (k) (list (org-element-property :key k)
                      (org-element-property :value k)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: macro
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: g H $1 $2!\n{{{g(A, B)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: link types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_lnk_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[[https://x][w]] [[file:f][f]] [[id:i][i]] [[elisp:(+ 1)][e]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l)
                      (org-element-property :path l)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: property ops
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_prop_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (let ((p1 (org-entry-properties nil 'standard)))
    (org-entry-put nil "C" "3")
    (org-entry-delete nil "B")
    (list p1 (org-entry-properties nil 'standard))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: tag ops
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
      (list t1 t2 (org-get-tags nil t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: priority ops
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_pri_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] H\n* TODO H2")
  (goto-char (point-min))
  (let ((p1 (org-get-priority (char-after))))
    (org-priority 'down)
    (let ((p2 (org-get-priority (char-after))))
      (forward-line)
      (org-priority ?B)
      (list p1 p2 (org-get-priority (char-after))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tera: todo cycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_todo_cyc() {
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
// Tera: visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_vis() {
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
// Tera: sparse dates
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_dates() {
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
