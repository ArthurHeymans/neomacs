//! Strong xiii combo oracle tests — extreme deep state.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Xiii: document
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_doc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: D\n* TODO H1 :t:\nSCHEDULED: <2026-01-15>\n:PROPERTIES:\n:V: v\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (types (org-element-map tree (lambda (el) (org-element-type el)))))
    types))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_prop() {
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
// Xiii: table
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_tbl() {
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
      (list d1 d2 (org-table-to-lisp)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: checkbox
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_cb() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [ ] a\n  - [ ] a1\n- [ ] b\n- [ ] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line 2)
    (org-toggle-checkbox)
    (org-update-statistics-cookies t)
    (goto-char (point-min))
    (list h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: sparse
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_sp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1 :w:\n* T2 :p:\n* T3 :w:\n* T4")
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
// Xiii: headline
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_hl() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] T :t:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\n:PROPERTIES:\n:V: v\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (h (car (org-element-map tree 'headline (lambda (h) h))))
         (p (car (org-element-map (org-element-contents h) 'planning
                   (lambda (p) p)))))
    (list (org-element-property :todo-keyword h)
          (org-element-property :priority h)
          (org-element-property :tags h)
          (org-element-property :scheduled p)
          (org-element-property :deadline p))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: export
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_exp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+OPTIONS: toc:nil\n* H")
  (let* ((info (org-export-get-environment nil)))
    (list (plist-get info :title) (plist-get info :with-toc))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: element chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_ec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] T :t:\n:PROPERTIES:\n:V: v\n:END:\nBody")
  (goto-char (point-min))
  (let* ((el (org-element-at-point))
         (p1 (list :todo (org-element-property :todo-keyword el)
                   :pri (org-element-property :priority el)
                   :tags (org-element-property :tags el))))
    (org-todo 'right)
    (org-priority 'down)
    (org-set-tags '("n"))
    (org-edit-headline "C")
    (let* ((el2 (org-element-at-point))
           (p2 (list :todo (org-element-property :todo-keyword el2)
                     :pri (org-element-property :priority el2)
                     :tags (org-element-property :tags el2)
                     :title (org-element-property :raw-value el2))))
      (list p1 p2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: multi-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_mb() {
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
// Xiii: planning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_pl() {
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
// Xiii: block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_bl() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp -n\n(+ 1 2)\n#+END_SRC")
  (org-element-map (org-element-parse-buffer) 'src-block
    (lambda (b) (list (org-element-property :language b)
                      (org-element-property :switches b)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: timestamp
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_ts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* M\n<2026-01-15 10:00-11:30>\n<2026-01-16>--<2026-01-20>")
  (org-element-map (org-element-parse-buffer) 'timestamp
    (lambda (t) (list (org-element-property :type t)
                      (org-element-property :year-start t)
                      (org-element-property :day-start t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_lnk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[[https://x][w]] [[file:f][f]] [[id:i][i]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l)
                      (org-element-property :path l)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: footnote
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_fn() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] *bold*")
  (let* ((tree (org-element-parse-buffer))
         (fn (org-element-map tree 'footnote-reference
               (lambda (f) (org-element-property :label f))))
         (fd (org-element-map tree 'footnote-definition
               (lambda (d) (org-element-property :label d)))))
    (list fn fd)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: outline
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_ol() {
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
// Xiii: visibility
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_vi() {
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
// Xiii: sparse dates
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_sd() {
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
// Xiii: macro
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_mc() {
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
// Xiii: dynamic block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_db() {
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
// Xiii: structure template
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_st() {
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
// Xiii: comment fixed
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_cf() {
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
// Xiii: pcomplete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_pc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\agr")
  (length (all-completions "\\ag" (pcomplete-entries))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: colview
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_cv() {
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
// Xiii: entity radio
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_er() {
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
// Xiii: inline
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_in() {
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
// Xiii: keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_kw() {
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
// Xiii: agenda
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_ag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\n* DONE T2\n* TODO T3")
  (org-map-entries
    (lambda ()
      (list (org-get-heading t t t t)
            (org-get-todo-state)))
    nil 'file))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: refile
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_rf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P1\n** T1\n* P2\n** T2")
  (mapcar 'car (org-refile-get-targets nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_sts() {
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
// Xiii: property inheritance
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_pi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PROPERTY: V 1\n* L1\n:PROPERTIES:\n:V: 2\n:END:\n** L2\n*** L3")
  (goto-char (point-min))
  (search-forward "L3")
  (list (org-entry-get nil "V" 'inherit)
        (org-entry-get nil "V" nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Xiii: hierarchy
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_xiii_hi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* L1\n** L2a\n*** L3a\n*** L3b\n** L2b\n* L1b")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h)
      (list (org-element-property :level h)
            (org-element-property :raw-value h)
            (length (org-element-contents h))))))"##,
    );
}
