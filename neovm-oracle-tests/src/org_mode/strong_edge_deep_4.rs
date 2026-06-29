//! Strong edge-deep-4 oracle tests — boundary conditions.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Edge: empty buffer operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_empty_buffer_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert-heading)
  (insert "First")
  (list (org-get-heading t t t t) (org-current-level)))"##,
        expect_test::expect![[r#""ERR (void-function insert-heading)""#]],
    );
}

#[test]
fn ed4_empty_buffer_meta_return() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (org-meta-return)
  (insert "Item")
  (list (buffer-string) (org-element-type (org-element-at-point))))"##,
        expect_test::expect![[r#""OK (\"* Item\" headline)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: deeply nested structure
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_deep_nesting_10() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* L1\n** L2\n*** L3\n**** L4\n***** L5\n****** L6\n******* L7\n******** L8\n********* L9\n********** L10")
  (goto-char (point-max))
  (list (org-current-level) (org-get-heading t t t t)))"##,
        expect_test::expect![[r#""OK (10 \"L10\")""#]],
    );
}

#[test]
fn ed4_deep_nesting_promote_demote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\n**** H4")
  (goto-char (point-min))
  (search-forward "H3")
  (let ((before (list (org-current-level) (org-get-heading t t t t))))
    (org-promote)
    (let ((after-p (list (org-current-level) (org-get-heading t t t t))))
      (org-demote)
      (org-demote)
      (list before after-p (list (org-current-level) (org-get-heading t t t t))))))"##,
        expect_test::expect![[r#""OK ((3 \"H3\") (2 \"H3\") (4 \"H3\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: special characters
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_special_chars_headline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Headline with *special* /chars/ =code= :tag:")
  (goto-char (point-min))
  (list (org-get-heading t t t t) (org-get-todo-state) (org-get-priority (char-after)) (org-get-tags nil t)))"##,
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

#[test]
fn ed4_special_chars_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[https://example.com/path?q=1&r=2][link & more]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l) (org-element-property :path l) (org-element-property :raw-link l)))))"##,
        expect_test::expect![[
            r#""OK ((\"https\" \"//example.com/path?q=1&r=2\" \"https://example.com/path?q=1&r=2\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: unicode content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_unicode_headlines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO 任务一\n* DONE 任务二\n* WAITING 任务三")
  (goto-char (point-min))
  (list (list (org-get-todo-state) (org-get-heading t t t t))
        (progn (forward-line) (list (org-get-todo-state) (org-get-heading t t t t)))
        (progn (forward-line) (list (org-get-todo-state) (org-get-heading t t t t)))))"##,
        expect_test::expect![[
            r#""OK ((\"TODO\" \"任务一\") (\"DONE\" \"任务二\") (nil \"WAITING 任务三\"))""#
        ]],
    );
}

#[test]
fn ed4_unicode_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:VAR: 値\n:NAME: 名前\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "VAR") (org-entry-get nil "NAME")
        (progn (org-entry-put nil "VAR" "新しい値") (org-entry-get nil "VAR"))))"##,
        expect_test::expect![[r#""OK (\"値\" \"名前\" \"新しい値\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: table boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_table_single_cell() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| only |")
  (list (org-table-to-lisp) (org-table-current-line) (org-table-current-column)))"##,
        expect_test::expect![[r#""OK (((\"only\")) 1 2)""#]],
    );
}

#[test]
fn ed4_table_empty_rows() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|   |   |\n| c | d |")
  (goto-char (point-min))
  (let ((d1 (org-table-to-lisp)))
    (org-table-next-row)
    (org-table-put 2 1 "X")
    (list d1 (org-table-to-lisp))))"##,
        expect_test::expect![[
            r#""OK (((\"a\" \"b\") (\"\" \"\") (\"c\" \"d\")) ((#(\"a\" 0 1 (face org-table)) #(\"b\" 0 1 (face org-table))) (\"X\" \"\") (#(\"c\" 0 1 (face org-table)) #(\"d\" 0 1 (face org-table)))))""#
        ]],
    );
}

#[test]
fn ed4_table_formula_division() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| 10 | 3 |\n| 7 | 2 |\n#+TBLFM: $3=$1/$2")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (list (org-table-get 1 3) (org-table-get 2 3)))"##,
        expect_test::expect![[
            r#""OK (#(\"3.3333333\" 0 9 (face org-table)) #(\"3.5\" 0 3 (face org-table)))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: list boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_list_single_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- single item")
  (org-element-map (org-element-parse-buffer) 'plain-list
    (lambda (pl) (list (org-element-property :type pl) (length (org-element-contents pl))))))"##,
        expect_test::expect![[r#""OK ((unordered 1))""#]],
    );
}

#[test]
fn ed4_list_deeply_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- L1\n  - L2\n    - L3\n      - L4\n        - L5")
  (org-element-map (org-element-parse-buffer) 'item
    (lambda (it) (list (org-element-property :bullet it) (org-element-property :level it)))))"##,
        expect_test::expect![[
            r#""OK ((\"- \" nil) (\"- \" nil) (\"- \" nil) (\"- \" nil) (\"- \" nil))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: footnote boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_footnote_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1: inline definition] more text")
  (let* ((tree (org-element-parse-buffer))
         (fn (car (org-element-map tree 'footnote-reference (lambda (f) f)))))
    (list (org-element-property :label fn) (org-element-property :type fn))))"##,
        expect_test::expect![[r#""OK (\"1\" inline)""#]],
    );
}

#[test]
fn ed4_footnote_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] ")
  (let* ((tree (org-element-parse-buffer))
         (fn (car (org-element-map tree 'footnote-reference (lambda (f) f))))
         (def (car (org-element-map tree 'footnote-definition (lambda (fd) fd)))))
    (list (org-element-property :label fn) (org-element-property :label def))))"##,
        expect_test::expect![[r#""OK (\"1\" \"1\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: timestamp boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_timestamp_active_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<2026-01-15>")
  (let* ((tree (org-element-parse-buffer))
         (ts (car (org-element-map tree 'timestamp (lambda (t) t)))))
    (list (org-element-property :type ts) (org-element-property :year-start ts) (org-element-property :day-start ts))))"##,
        expect_test::expect![[r#""OK (active 2026 15)""#]],
    );
}

#[test]
fn ed4_timestamp_inactive_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[2026-01-15 14:30]")
  (let* ((tree (org-element-parse-buffer))
         (ts (car (org-element-map tree 'timestamp (lambda (t) t)))))
    (list (org-element-property :type ts) (org-element-property :hour-start ts) (org-element-property :minute-start ts))))"##,
        expect_test::expect![[r#""OK (inactive 14 30)""#]],
    );
}

#[test]
fn ed4_timestamp_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<2026-01-15>--<2026-01-20>")
  (let* ((tree (org-element-parse-buffer))
         (ts (car (org-element-map tree 'timestamp (lambda (t) t)))))
    (list (org-element-property :type ts) (org-element-property :year-start ts) (org-element-property :day-start ts) (org-element-property :year-end ts) (org-element-property :day-end ts))))"##,
        expect_test::expect![[r#""OK (active-range 2026 15 2026 20)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: block boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_block_empty_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (block (car (org-element-map tree 'src-block (lambda (b) b)))))
    (list (org-element-property :language block) (org-element-property :value block))))"##,
        expect_test::expect![[r#""OK (\"emacs-lisp\" \"\")""#]],
    );
}

#[test]
fn ed4_block_multiple_languages() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC python\nprint('hello')\n#+END_SRC\n\n#+BEGIN_SRC emacs-lisp\n(message \"hi\")\n#+END_SRC\n\n#+BEGIN_SRC shell\necho test\n#+END_SRC")
  (org-element-map (org-element-parse-buffer) 'src-block
    (lambda (b) (list (org-element-property :language b) (org-element-property :value b)))))"##,
        expect_test::expect![[
            r#""OK ((\"python\" \"print('hello')\n\") (\"emacs-lisp\" \"(message \\\"hi\\\")\n\") (\"shell\" \"echo test\n\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: drawer boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_drawer_empty_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:END:")
  (let* ((tree (org-element-parse-buffer))
         (drawer (car (org-element-map tree 'drawer (lambda (d) d)))))
    (org-element-property :drawer-name drawer))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn ed4_drawer_multiple_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:A: 1\n:B: 2\n:C: 3\n:D: 4\n:E: 5\n:END:")
  (goto-char (point-min))
  (let ((props (org-entry-properties nil 'standard)))
    (list (length props) (alist-get "A" props nil nil 'equal) (alist-get "E" props nil nil 'equal))))"##,
        expect_test::expect![[r#""OK (6 \"1\" \"5\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: link boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_link_no_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[https://example.com]]")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l)))))
    (list (org-element-property :type link) (org-element-property :path link) (org-element-property :raw-link link))))"##,
        expect_test::expect![[r#""OK (\"https\" \"//example.com\" \"https://example.com\")""#]],
    );
}

#[test]
fn ed4_link_angle_brackets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<https://example.com>")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l)))))
    (list (org-element-property :type link) (org-element-property :path link))))"##,
        expect_test::expect![[r#""OK (\"https\" \"//example.com\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: planning boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_planning_deadline_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\nDEADLINE: <2026-01-20>")
  (goto-char (point-min))
  (list (org-entry-get nil "DEADLINE") (org-entry-get nil "SCHEDULED") (org-entry-get nil "CLOSED")))"##,
        expect_test::expect![[r#""OK (\"<2026-01-20>\" nil nil)""#]],
    );
}

#[test]
fn ed4_planning_closed_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* DONE Task\nCLOSED: [2026-01-15]")
  (goto-char (point-min))
  (list (org-entry-get nil "DEADLINE") (org-entry-get nil "SCHEDULED") (org-entry-get nil "CLOSED")))"##,
        expect_test::expect![[r#""OK (nil nil \"[2026-01-15]\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: tag boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_tag_no_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Simple heading")
  (goto-char (point-min))
  (org-get-tags nil t))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn ed4_tag_multiple_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading :tag1:tag2:tag3:")
  (goto-char (point-min))
  (let ((tags (org-get-tags nil t)))
    (org-set-tags '("new1" "new2"))
    (list tags (org-get-tags nil t))))"##,
        expect_test::expect![[r#""OK ((\"tag1\" \"tag2\" \"tag3\") (\"new1\" \"new2\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: priority boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_priority_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO No priority")
  (goto-char (point-min))
  (org-get-priority (char-after)))"##,
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

#[test]
fn ed4_priority_all_levels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] High\n* TODO [#B] Medium\n* TODO [#C] Low")
  (goto-char (point-min))
  (list (org-get-priority (char-after))
        (progn (forward-line) (org-get-priority (char-after)))
        (progn (forward-line) (org-get-priority (char-after)))))"##,
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: todo boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_todo_no_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Just a heading")
  (goto-char (point-min))
  (org-get-todo-state))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn ed4_todo_custom_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (setq org-todo-keywords '((sequence "TODO" "IN-PROGRESS" "REVIEW" "DONE")))
  (insert "* TODO Task")
  (goto-char (point-min))
  (let ((s1 (org-get-todo-state)))
    (org-todo 'right)
    (let ((s2 (org-get-todo-state)))
      (org-todo 'right)
      (let ((s3 (org-get-todo-state)))
        (org-todo 'right)
        (list s1 s2 s3 (org-get-todo-state))))))"##,
        expect_test::expect![[
            r#""OK (\"TODO\" #(\"IN-PROGRESS\" 0 11 (org-todo-head \"TODO\")) #(\"REVIEW\" 0 6 (org-todo-head \"TODO\")) #(\"DONE\" 0 4 (org-todo-head \"TODO\")))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: visibility boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_visibility_single_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Only heading")
  (goto-char (point-min))
  (let ((hidden (get-char-property (point) 'invisible)))
    (org-cycle)
    (list hidden (get-char-property (point) 'invisible))))"##,
        expect_test::expect![[r#""OK (nil nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: property boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_property_empty_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:EMPTY: \n:END:")
  (goto-char (point-min))
  (org-entry-get nil "EMPTY"))"##,
        expect_test::expect![[r#""OK \"\"""#]],
    );
}

#[test]
fn ed4_property_overwrite() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:VAR: old\n:END:")
  (goto-char (point-min))
  (let ((v1 (org-entry-get nil "VAR")))
    (org-entry-put nil "VAR" "new")
    (let ((v2 (org-entry-get nil "VAR")))
      (org-entry-put nil "VAR" "final")
      (list v1 v2 (org-entry-get nil "VAR")))))"##,
        expect_test::expect![[r#""OK (\"old\" \"new\" \"final\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: macro boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_macro_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greeting Hello!\n{{{greeting}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
        expect_test::expect![[r#""ERR (error \"Undefined Org macro: greeting; aborting\")""#]],
    );
}

#[test]
fn ed4_macro_multiple_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greet Hello $1 and $2!\n{{{greet(Alice, Bob)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
        expect_test::expect![[r#""ERR (error \"Undefined Org macro: greet; aborting\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: dynamic block boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_dynamic_block_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN: clocktable\n#+END:")
  (goto-char (point-min))
  (org-dblock-update)
  (buffer-string))"##,
        expect_test::expect![[
            r##""OK #(\"#+BEGIN: clocktable\n#+CAPTION: Clock summary at [FIXED-TIME]\n| Headline     | Time   |\n|--------------+--------|\n| *Total time* | *0:00* |\n#+END:\" 61 62 (face org-table) 62 63 (face org-table rear-nonsticky t display (space :relative-width 1)) 63 71 (face org-table) 71 75 (face org-table) 75 76 (face org-table display (space :relative-width 1.001)) 76 77 (face org-table) 77 78 (face org-table rear-nonsticky t display (space :relative-width 1)) 78 82 (face org-table) 82 84 (face org-table) 84 85 (face org-table display (space :relative-width 1.001)) 85 86 (face org-table) 86 87 (face org-table-row) 87 88 (face org-table) 88 112 (face org-table) 112 113 (face org-table-row) 113 114 (face org-table) 114 115 (face org-table rear-nonsticky t display (space :relative-width 1)) 115 127 (org-emphasis t font-lock-multiline t face (bold org-table)) 127 128 (face org-table display (space :relative-width 1.001)) 128 129 (face org-table) 129 130 (face org-table rear-nonsticky t display (space :relative-width 1)) 130 136 (org-emphasis t font-lock-multiline t face (bold org-table)) 136 137 (face org-table display (space :relative-width 1.001)) 137 138 (face org-table) 138 139 (face org-table-row))""##
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: structure template boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_structure_template_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<s")
  (org-try-structure-completion)
  (let ((s1 (buffer-string)))
    (erase-buffer)
    (insert "<e")
    (org-try-structure-completion)
    (list s1 (buffer-string))))"##,
        expect_test::expect![[r#""ERR (void-function org-try-structure-completion)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: comment boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_comment_single_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# Comment line")
  (org-element-map (org-element-parse-buffer) 'comment
    (lambda (c) (org-element-property :value c))))"##,
        expect_test::expect![[r#""OK (\"Comment line\")""#]],
    );
}

#[test]
fn ed4_comment_multiple_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# Comment 1\n# Comment 2\n# Comment 3")
  (org-element-map (org-element-parse-buffer) 'comment
    (lambda (c) (org-element-property :value c))))"##,
        expect_test::expect![[r#""OK (\"Comment 1\nComment 2\nComment 3\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: fixed-width boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_fixed_width_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert ": Fixed width")
  (let* ((tree (org-element-parse-buffer))
         (fw (car (org-element-map tree 'fixed-width (lambda (f) f)))))
    (org-element-property :value fw))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn ed4_fixed_width_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert ": Line 1\n: Line 2\n: Line 3")
  (org-element-map (org-element-parse-buffer) 'fixed-width
    (lambda (f) (org-element-property :value f))))"##,
        expect_test::expect![[r#""OK (\"Line 1\nLine 2\nLine 3\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: export boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_export_empty_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (let* ((info (org-export-get-environment nil)))
    (plist-get info :title)))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn ed4_export_title_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: My Title")
  (let* ((info (org-export-get-environment nil)))
    (plist-get info :title)))"##,
        expect_test::expect![[
            r#""OK (#(\"My Title\" 0 8 (:parent (#(\"My Title\" 0 8 (:parent #3))))))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: element boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_element_buffer_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-min))
  (let* ((el (org-element-at-point)))
    (list (org-element-type el) (org-element-property :begin el))))"##,
        expect_test::expect![[r#""OK (headline 1)""#]],
    );
}

#[test]
fn ed4_element_buffer_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-max))
  (org-element-type (org-element-at-point)))"##,
        expect_test::expect![[r#""OK paragraph""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: clock boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_clock_no_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task with no clock")
  (org-clocking-p))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: statistics boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_statistics_no_cookies() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task\n- [ ] item 1\n- [ ] item 2")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
        expect_test::expect![[r#""OK \"* Task\"""#]],
    );
}

#[test]
fn ed4_statistics_all_checked() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [0/0]\n- [X] item 1\n- [X] item 2\n- [X] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
        expect_test::expect![[r#""OK \"* Task [3/3]\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: sparse tree boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_sparse_tree_no_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task 1\n* TODO Task 2")
  (goto-char (point-min))
  (org-match-sparse-tree nil "DONE")
  (let ((v '()) (h '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((hd (org-get-heading t t t t)))
        (when hd
          (if (get-char-property (point) 'invisible) (push hd h) (push hd v))))
      (forward-line))
    (list (nreverse v) (nreverse h))))"##,
        expect_test::expect![[r#""OK ((\"Task 1\" \"Task 2\") nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Edge: table transpose boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ed4_table_single_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |")
  (org-table-to-lisp))"##,
        expect_test::expect![[r#""OK ((\"a\" \"b\" \"c\"))""#]],
    );
}

#[test]
fn ed4_table_single_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a |\n| b |\n| c |")
  (org-table-to-lisp))"##,
        expect_test::expect![[r#""OK ((\"a\") (\"b\") (\"c\"))""#]],
    );
}
