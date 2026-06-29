//! Strong divergence-hunt-11 oracle tests — targeted at specific bugs.
//!
//! These tests specifically target the known divergence patterns:
//! - Parent property nesting in title/caption
//! - Minibuffer message differences
//! - Drawer parsing "Invalid search bound"
//! - Clock-in "Invalid search bound"

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Targeted: parent nesting in export title
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_parent_nesting_export_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test Title\n* H")
  (let* ((info (org-export-get-environment nil))
         (title (plist-get info :title)))
    (list (stringp (car title)) (length title))))"##,
        expect_test::expect![[r#""OK (t 1)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: parent nesting in caption
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_parent_nesting_caption() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: My Caption\n[[file:test.png]]")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l))))
         (parent (org-element-property :parent link))
         (caption (org-element-property :caption parent)))
    (list (stringp (car (car caption))))))"##,
        expect_test::expect![[r#""OK (nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: drawer parsing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_drawer_parse_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (drawers (org-element-map tree 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    drawers))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn dh11_drawer_parse_logbook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\n- Note\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (drawers (org-element-map tree 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    drawers))"##,
        expect_test::expect![[r#""OK (\"LOGBOOK\")""#]],
    );
}

#[test]
fn dh11_drawer_parse_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (drawers (org-element-map tree 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    drawers))"##,
        expect_test::expect![[r#""OK (\"LOGBOOK\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: clock-in
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_clock_in_out() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Test")
  (goto-char (point-min))
  (let ((c0 (org-clocking-p)))
    (org-clock-in)
    (let ((c1 (org-clocking-p)))
      (org-clock-out)
      (let ((c2 (org-clocking-p)))
        (list c0 c1 c2)))))"##,
        expect_test::expect![[r#""OK (nil t nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: minibuffer message differences
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_minibuffer_deadline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO H")
  (goto-char (point-min))
  (org-deadline nil "2026-01-20")
  (org-entry-get nil "DEADLINE"))"##,
        expect_test::expect![[r#""OK \"<2026-01-20 Tue>\"""#]],
    );
}

#[test]
fn dh11_minibuffer_schedule() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO H")
  (goto-char (point-min))
  (org-schedule nil "2026-01-15")
  (org-entry-get nil "SCHEDULED"))"##,
        expect_test::expect![[r#""OK \"<2026-01-15 Thu>\"""#]],
    );
}

#[test]
fn dh11_minibuffer_insert_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-min))
  (org-store-link nil)
  (goto-char (point-max))
  (let ((stored (car org-stored-links)))
    (org-insert-link nil stored "click")
    (buffer-substring-no-properties (point-min) (point-max))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: element map with parent access
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_element_map_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n** Sub\nBody")
  (let* ((tree (org-element-parse-buffer))
         (para (car (org-element-map tree 'paragraph (lambda (p) p))))
         (parent (org-element-property :parent para))
         (grandparent (org-element-property :parent parent)))
    (list (org-element-type para)
          (org-element-type parent)
          (org-element-type grandparent))))"##,
        expect_test::expect![[r#""OK (paragraph section headline)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: export with nested elements
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_export_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Nested\n#+CAPTION: Cap\n#+ATTR_HTML: :width 300\n[[file:img.png]]\n* H\n** Sub")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (links (org-element-map tree 'link
                  (lambda (l)
                    (let ((p (org-element-property :parent l)))
                      (list (org-element-property :path l)
                            (org-element-property :caption p)))))))
    (list (plist-get info :title) links)))"##,
        expect_test::expect![[
            r#""OK ((#(\"Nested\" 0 6 (:parent (#(\"Nested\" 0 6 (:parent #4)))))) ((\"img.png\" (((#(\"Cap\" 0 3 (:parent (#(\"Cap\" 0 3 (:parent #8)))))))))))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: keyword value with parent
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_keyword_value_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: My Title\n#+AUTHOR: Author\n* H")
  (let* ((tree (org-element-parse-buffer))
         (kws (org-element-map tree 'keyword
                (lambda (k)
                  (list (org-element-property :key k)
                        (org-element-property :value k)
                        (stringp (org-element-property :value k)))))))
    kws))"##,
        expect_test::expect![[r#""OK ((\"TITLE\" \"My Title\" t) (\"AUTHOR\" \"Author\" t))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: headline with all planning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_headline_all_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\nCLOSED: [2026-01-10]")
  (let* ((tree (org-element-parse-buffer))
         (planning (car (org-element-map tree 'planning (lambda (p) p)))))
    (list (org-element-property :scheduled planning)
          (org-element-property :deadline planning)
          (org-element-property :closed planning))))"##,
        expect_test::expect![[
            r#""OK ((timestamp (:standard-properties [24 nil nil nil 36 0 nil nil nil nil nil nil nil nil nil nil nil nil] :type active :range-type nil :raw-value \"<2026-01-15>\" :year-start 2026 :month-start 1 :day-start 15 :hour-start nil :minute-start nil :year-end 2026 :month-end 1 :day-end 15 :hour-end nil :minute-end nil)) nil nil)""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: timestamp with repeater
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_timestamp_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* M\n<2026-01-15 Wed +1w -3d>")
  (let* ((tree (org-element-parse-buffer))
         (ts (car (org-element-map tree 'timestamp (lambda (t) t)))))
    (list (org-element-property :type ts)
          (org-element-property :repeater-type ts)
          (org-element-property :repeater-value ts)
          (org-element-property :repeater-unit ts)
          (org-element-property :warning-type ts))))"##,
        expect_test::expect![[r#""OK (active cumulate 1 week all)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: table with formula references
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_table_formula_refs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| A | 1 | 2 |\n| B | 3 | 4 |\n|---+---+---|\n| Sum | 4 | 6 |\n#+TBLFM: $3=$2*2::@4$2=vsum(@2..@3)")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (list (org-table-get 1 3)
        (org-table-get 2 3)
        (org-table-get 4 2)
        (org-table-get 4 3)))"##,
        expect_test::expect![[r#""ERR (args-out-of-range [nil 0 1 3] 4)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: block with header arguments
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_block_header_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+NAME: my-block\n#+HEADER: :var x=1\n#+BEGIN_SRC emacs-lisp :results value\n(+ x 1)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (block (car (org-element-map tree 'src-block (lambda (b) b)))))
    (list (org-element-property :language block)
          (org-element-property :name block)
          (org-element-property :parameters block)
          (org-element-property :value block))))"##,
        expect_test::expect![[
            r#""OK (\"emacs-lisp\" \"my-block\" \":results value\" \"(+ x 1)\n\")""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: footnote with multiple references
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_footnote_multi_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2] again[fn:1]\n\n[fn:1] First\n[fn:2] Second")
  (let* ((tree (org-element-parse-buffer))
         (refs (org-element-map tree 'footnote-reference
                 (lambda (f) (org-element-property :label f))))
         (defs (org-element-map tree 'footnote-definition
                 (lambda (d) (org-element-property :label d)))))
    (list refs defs)))"##,
        expect_test::expect![[r#""OK ((\"1\" \"2\" \"1\") (\"1\" \"2\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: link with search options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_link_search_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[file:test.org::*heading][h]] and [[file:test.org::#custom-id][id]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l)
      (list (org-element-property :type l)
            (org-element-property :path l)
            (org-element-property :search-option l)))))"##,
        expect_test::expect![[
            r##""OK ((\"file\" \"test.org\" \"*heading\") (\"file\" \"test.org\" \"#custom-id\"))""##
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: affiliated keywords all types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_affiliated_all_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[
            r#""OK (paragraph (((#(\"Cap\" 0 3 (:parent (#(\"Cap\" 0 3 (:parent #6)))))))) (\":width 300\") (\":width 0.5\\\\textwidth\") \"fig\")""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: element-map with first-match
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_element_map_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (let* ((tree (org-element-parse-buffer))
         (all (org-element-map tree 'headline
                (lambda (h) (org-element-property :raw-value h))))
         (first (org-element-map tree 'headline
                  (lambda (h) (org-element-property :raw-value h))
                  nil 'first-match)))
    (list all first)))"##,
        expect_test::expect![[r#""OK ((\"H1\" \"H2\" \"H3\") \"H1\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: element-map no recursive
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_element_map_no_recurse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2a\n*** H3\n** H2b")
  (let* ((tree (org-element-parse-buffer))
         (h1 (car (org-element-map tree 'headline (lambda (h) h))))
         (direct (org-element-map (org-element-contents h1) 'headline
                   (lambda (h) (org-element-property :raw-value h))
                   nil nil nil t))
         (recursive (org-element-map (org-element-contents h1) 'headline
                      (lambda (h) (org-element-property :raw-value h)))))
    (list direct recursive)))"##,
        expect_test::expect![[r#""OK ((\"H2a\" \"H3\" \"H2b\") (\"H2a\" \"H3\" \"H2b\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: export environment with all options
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_export_env_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Opts\n#+OPTIONS: toc:2 num:t ^:{} \\n:t")
  (let* ((info (org-export-get-environment nil)))
    (list (plist-get info :with-toc)
          (plist-get info :with-numbers))))"##,
        expect_test::expect![[r#""OK (2 nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: property inheritance deep
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_property_inherit_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+PROPERTY: VAR 1\n* L1\n:PROPERTIES:\n:VAR: 2\n:END:\n** L2\n*** L3\n:PROPERTIES:\n:VAR: 3\n:END:\n**** L4")
  (goto-char (point-min))
  (search-forward "L4")
  (list (org-entry-get nil "VAR" 'inherit)
        (org-entry-get nil "VAR" nil)))"##,
        expect_test::expect![[r#""OK (\"3\" nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: headline with inline markup
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_headline_inline_markup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title with *bold* and /italic/ :tag:")
  (goto-char (point-min))
  (let* ((tree (org-element-parse-buffer))
         (h (car (org-element-map tree 'headline (lambda (h) h))))
         (title (org-element-property :raw-value h))
         (todo (org-element-property :todo-keyword h))
         (priority (org-element-property :priority h))
         (tags (org-element-property :tags h)))
    (list title todo priority tags)))"##,
        expect_test::expect![[
            r#""OK (\"Title with *bold* and /italic/\" \"TODO\" 65 (\"tag\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: list with checkboxes and statistics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_list_checkbox_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [ ] a\n  - [ ] a1\n  - [ ] a2\n- [X] b\n- [ ] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line 2)
    (org-toggle-checkbox)
    (forward-line 1)
    (org-toggle-checkbox)
    (org-update-statistics-cookies t)
    (goto-char (point-min))
    (list h0 (buffer-substring-no-properties (line-beginning-position) (line-end-position)))))"##,
        expect_test::expect![[r#""OK (\"* T [33%]\" \"* T [66%]\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: sparse tree with dates
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_sparse_tree_dates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
          (if (get-char-property (point) 'invisible) (push hd h) (push hd v))))
      (forward-line))
    (list (nreverse v) (nreverse h))))"##,
        expect_test::expect![[r#""OK ((\"T1\" \"T2\" \"T3\" \"T4\") (\"T1\" \"T2\" \"T3\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: visibility cycling
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_visibility_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""ERR (wrong-number-of-arguments (0 . 0) 1)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: outline path deep
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_outline_path_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P\n** T1\n*** S1\n**** SS1\n***** SSS1")
  (goto-char (point-min))
  (search-forward "SSS1")
  (list (org-get-outline-path)
        (org-current-level)
        (org-get-heading t t t t)))"##,
        expect_test::expect![[r#""OK ((\"P\" \"T1\" \"S1\" \"SS1\") 5 \"SSS1\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: multi-buffer parse consistency
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_multi_buffer_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""OK ((\"A\" \"A1\") (\"B\" \"B1\" \"B2\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: element deferred chain (5 ops)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_element_deferred_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[
            r#""OK ((:todo \"TODO\" :pri 65 :tags (\"tag\") :var \"val\" :title \"Orig\") (:todo \"PROG\" :pri 66 :tags (\"newtag\") :var \"newval\" :title \"Changed\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: agenda with custom keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_agenda_custom_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (setq org-todo-keywords '((sequence "IDEA" "WORKING" "DONE")))
  (insert "* IDEA F1\n* WORKING F2\n* DONE F3")
  (org-map-entries
    (lambda () (list (org-get-heading t t t t) (org-get-todo-state)))
    nil 'file))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: refile targets with levels
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_refile_targets_levels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P1\n** T1\n*** S1\n* P2\n** T2")
  (let ((targets (org-refile-get-targets nil)))
    (mapcar (lambda (t) (list (car t) (cdr t))) targets)))"##,
        expect_test::expect![[
            r#""OK ((\"P1\" (nil \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|IDEA\\\\|WORKING\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(?:COMMENT +\\\\)?\\\\(?:\\\\[[0-9%/]+\\\\] *\\\\)*\\\\(P1\\\\)\\\\(?: *\\\\[[0-9%/]+\\\\]\\\\)*\\\\)\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" 1)) (\"P2\" (nil \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|IDEA\\\\|WORKING\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(?:COMMENT +\\\\)?\\\\(?:\\\\[[0-9%/]+\\\\] *\\\\)*\\\\(P2\\\\)\\\\(?: *\\\\[[0-9%/]+\\\\]\\\\)*\\\\)\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" 19)))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: colview with effort
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_colview_effort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %EFFORT\n* TODO Task 1\n:PROPERTIES:\n:EFFORT: 2h\n:END:\n* DONE Task 2\n:PROPERTIES:\n:EFFORT: 30m\n:END:")
  (goto-char (point-min))
  (org-columns-get-format))"##,
        expect_test::expect![[r#""ERR (void-function org-columns-get-format)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: block execution
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_block_execution() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (block (car (org-element-map tree 'src-block (lambda (b) b)))))
    (list (org-element-property :language block)
          (org-element-property :value block))))"##,
        expect_test::expect![[r#""OK (\"emacs-lisp\" \"(+ 1 2)\n\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: id generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_id_generation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2")
  (goto-char (point-min))
  (let ((id1 (org-id-get nil 'create)))
    (forward-line)
    (let ((id2 (org-id-get nil 'create)))
      (list (stringp id1) (stringp id2) (string= id1 id2)))))"##,
        expect_test::expect![[r#""ERR (error \"‘org-id-get’ expects a file-visiting buffer\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: clock sum
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_clock_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\n:END:\n* B\n:LOGBOOK:\nCLOCK: [2026-01-11 14:00]--[2026-01-11 15:30] =>  1:30\n:END:")
  (org-clock-sum))"##,
        expect_test::expect![[r#""OK 150""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: entity replacement
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_entity_replacement() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\alpha \\beta \\gamma")
  (let ((before (buffer-string)))
    (org-toggle-pretty-entities)
    (list before (buffer-string))))"##,
        expect_test::expect![[
            r#""OK (\"\\\\alpha \\\\beta \\\\gamma\" \"\\\\alpha \\\\beta \\\\gamma\")""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: radio targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_radio_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<<<target>>>\nSee target here")
  (let* ((tree (org-element-parse-buffer))
         (targets (org-element-map tree 'radio-target
                    (lambda (rt) (org-element-property :value rt)))))
    targets))"##,
        expect_test::expect![[r#""OK (\"target\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: structure template
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_structure_template() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<s")
  (org-try-structure-completion)
  (buffer-string))"##,
        expect_test::expect![[r#""ERR (void-function org-try-structure-completion)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: comment and fixed-width
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_comment_fixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# Comment\n: Fixed\nNormal")
  (let* ((tree (org-element-parse-buffer))
         (c (org-element-map tree 'comment
              (lambda (c) (org-element-property :value c))))
         (f (org-element-map tree 'fixed-width
              (lambda (f) (org-element-property :value f)))))
    (list c f)))"##,
        expect_test::expect![[r#""OK ((\"Comment\") (\"Fixed\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: dynamic block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_dynamic_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN: clocktable :maxlevel 2\n#+END:")
  (goto-char (point-min))
  (org-dblock-update)
  (buffer-string))"##,
        expect_test::expect![[
            r##""OK #(\"#+BEGIN: clocktable :maxlevel 2\n#+CAPTION: Clock summary at [FIXED-TIME]\n| Headline     | Time   |\n|--------------+--------|\n| *Total time* | *0:00* |\n#+END:\" 73 74 (face org-table) 74 75 (face org-table rear-nonsticky t display (space :relative-width 1)) 75 83 (face org-table) 83 87 (face org-table) 87 88 (face org-table display (space :relative-width 1.001)) 88 89 (face org-table) 89 90 (face org-table rear-nonsticky t display (space :relative-width 1)) 90 94 (face org-table) 94 96 (face org-table) 96 97 (face org-table display (space :relative-width 1.001)) 97 98 (face org-table) 98 99 (face org-table-row) 99 100 (face org-table) 100 124 (face org-table) 124 125 (face org-table-row) 125 126 (face org-table) 126 127 (face org-table rear-nonsticky t display (space :relative-width 1)) 127 139 (org-emphasis t font-lock-multiline t face (bold org-table)) 139 140 (face org-table display (space :relative-width 1.001)) 140 141 (face org-table) 141 142 (face org-table rear-nonsticky t display (space :relative-width 1)) 142 148 (org-emphasis t font-lock-multiline t face (bold org-table)) 148 149 (face org-table display (space :relative-width 1.001)) 149 150 (face org-table) 150 151 (face org-table-row))""##
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: macro expansion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_macro_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greet Hello $1!\n{{{greet(World)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
        expect_test::expect![[r#""ERR (error \"Undefined Org macro: greet; aborting\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: pcomplete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_pcomplete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\agr")
  (length (all-completions "\\ag" (pcomplete-entries))))"##,
        expect_test::expect![[r#""OK 0""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: narrow to subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_narrow_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody 1\n** H2\nSub\n* H2b\nBody 2")
  (goto-char (point-min))
  (org-narrow-to-subtree)
  (let ((narrowed (buffer-string)))
    (widen)
    (list narrowed)))"##,
        expect_test::expect![[r#""OK (\"* H1\nBody 1\n** H2\nSub\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: end of subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_end_of_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2a\n*** H3\nBody\n** H2b\n* H1b")
  (goto-char (point-min))
  (let ((p1 (progn (org-end-of-subtree) (point))))
    (goto-char (point-min))
    (search-forward "H2a")
    (beginning-of-line)
    (let ((p2 (progn (org-end-of-subtree) (point))))
      (list p1 p2))))"##,
        expect_test::expect![[r#""OK (31 24)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: mark subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_mark_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub\n* H1b")
  (goto-char (point-min))
  (org-mark-subtree)
  (let ((m (mark))
        (p (point)))
    (list (< p m) p m)))"##,
        expect_test::expect![[r#""OK (t 1 21)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: clone subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_clone_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task\n** Sub1\n** Sub2")
  (goto-char (point-min))
  (org-clone-subtree 2)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :level h)
                      (org-element-property :raw-value h)))))"##,
        expect_test::expect![[r#""ERR (void-function org-clone-subtree)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: sort entries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_sort_entries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Zebra\n* Apple\n* Mango\n* Banana")
  (org-sort-entries nil ?a)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (org-element-property :raw-value h))))"##,
        expect_test::expect![[r#""ERR (user-error \"Nothing to sort\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: move subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_move_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""OK ((\"A\" \"B\" \"C\" \"D\") (\"A\" \"C\" \"B\" \"D\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: copy paste subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_copy_paste_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[
            r#""OK ((1 \"H1\") (2 \"Sub1\") (1 \"H2\") (2 \"Sub2\") (1 \"H1\") (2 \"Sub1\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: table formula alignment
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_table_formula_align() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | 1 | 2 |\n| b | 3 | 4 |\n| c | 5 | 6 |\n|---+---+---|\n| Sum | 9 | 12 |\n#+TBLFM: $4=$2+$3::@5$2=vsum(@2..@4)::@5$3=vsum(@2..@4)")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (org-table-to-lisp))"##,
        expect_test::expect![[r#""ERR (args-out-of-range [nil 0 1 2 4] 5)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: list all features
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_list_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [X] a\n- [ ] b\n  - [X] b1\n  - [ ] b2\n- [X] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
        expect_test::expect![[r#""OK \"* T [66%]\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: footnote all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_footnote_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:2]\n\n[fn:1] *bold* /italic/\n[fn:2] [[link][desc]]")
  (let* ((tree (org-element-parse-buffer))
         (fn (org-element-map tree 'footnote-reference
               (lambda (f) (org-element-property :label f))))
         (fd (org-element-map tree 'footnote-definition
               (lambda (d) (org-element-property :label d)))))
    (list fn fd)))"##,
        expect_test::expect![[r#""OK ((\"1\" \"2\") (\"1\" \"2\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: clock all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_clock_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\n:PROPERTIES:\n:EFFORT: 2:00\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-15 10:00]--[2026-01-15 11:30] =>  1:30\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "EFFORT")
        (org-clock-sum-current-entry)))"##,
        expect_test::expect![[r#""ERR (void-function org-clock-sum-current-entry)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: link all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_link_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[[https://x][w]] [[file:f][f]] [[id:i][i]] [[elisp:(+ 1)][e]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l)
                      (org-element-property :path l)
                      (org-element-property :raw-link l)))))"##,
        expect_test::expect![[
            r#""OK ((\"https\" \"//x\" \"https://x\") (\"file\" \"f\" \"file:f\") (\"id\" \"i\" \"id:i\") (\"elisp\" \"(+ 1)\" \"elisp:(+ 1)\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: property all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_property_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (let ((p1 (org-entry-properties nil 'standard)))
    (org-entry-put nil "C" "3")
    (org-entry-delete nil "B")
    (list p1 (org-entry-properties nil 'standard))))"##,
        expect_test::expect![[
            r#""OK (((\"CATEGORY\" . \"???\") (\"B\" . \"2\") (\"A\" . \"1\")) ((\"CATEGORY\" . \"???\") (\"C\" . \"3\") (\"A\" . \"1\")))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: tag all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_tag_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H :a:b:")
  (goto-char (point-min))
  (let ((t1 (org-get-tags nil t)))
    (org-set-tags '("c" "d"))
    (let ((t2 (org-get-tags nil t)))
      (org-toggle-tag "e" 'on)
      (list t1 t2 (org-get-tags nil t)))))"##,
        expect_test::expect![[r#""OK ((\"a\" \"b\") (\"c\" \"d\") (\"c\" \"d\" \"e\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: priority all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_priority_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: todo all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_todo_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[
            r#""OK (nil #(\"IDEA\" 0 4 (org-todo-head \"IDEA\")) #(\"WORKING\" 0 7 (org-todo-head \"IDEA\")) #(\"DONE\" 0 4 (org-todo-head \"IDEA\")))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: visibility all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_visibility_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""ERR (wrong-number-of-arguments (0 . 0) 1)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Targeted: sparse dates
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn dh11_sparse_dates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
          (if (get-char-property (point) 'invisible) (push hd h) (push hd v))))
      (forward-line))
    (list (nreverse v) (nreverse h))))"##,
        expect_test::expect![[r#""OK ((\"T1\" \"T2\" \"T3\" \"T4\") (\"T1\" \"T2\" \"T3\"))""#]],
    );
}
