//! Strong ultimate edge-case oracle tests — maximum coverage.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: empty buffer with various operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_empty_buffer_all_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert-heading)
  (insert "First heading")
  (let ((h (org-get-heading t t t t))
        (level (org-current-level))
        (type (org-element-type (org-element-at-point))))
    (list h level type)))"##,
        expect_test::expect![[r#""ERR (void-function insert-heading)""#]],
    );
}

#[test]
fn strong_empty_buffer_meta_return() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (org-meta-return)
  (insert "New item")
  (let ((content (buffer-string))
        (type (org-element-type (org-element-at-point))))
    (list content type)))"##,
        expect_test::expect![[r#""OK (\"* New item\" headline)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: deeply nested structure
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_deep_nesting_max() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* L1\n** L2\n*** L3\n**** L4\n***** L5\n****** L6\n******* L7\n******** L8\n********* L9\n********** L10")
  (goto-char (point-max))
  (let ((level (org-current-level))
        (title (org-get-heading t t t t)))
    (list level title)))"##,
        expect_test::expect![[r#""OK (10 \"L10\")""#]],
    );
}

#[test]
fn strong_deep_nesting_promote_demote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\n**** H4")
  (goto-char (point-min))
  (search-forward "H3")
  (let ((before (list (org-current-level) (org-get-heading t t t t))))
    (org-promote)
    (let ((after-promote (list (org-current-level) (org-get-heading t t t t))))
      (org-demote)
      (org-demote)
      (let ((after-demote (list (org-current-level) (org-get-heading t t t t))))
        (list before after-promote after-demote)))))"##,
        expect_test::expect![[r#""OK ((3 \"H3\") (2 \"H3\") (4 \"H3\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: special characters in content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_special_chars_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Headline with *special* /chars/ =code= :tag:")
  (goto-char (point-min))
  (let ((title (org-get-heading t t t t))
        (todo (org-get-todo-state))
        (priority (org-get-priority (char-after)))
        (tags (org-get-tags nil t)))
    (list title todo priority tags)))"##,
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

#[test]
fn strong_special_chars_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[https://example.com/path?q=1&r=2][link & more]] and [[file:name with spaces.org][file link]]")
  (let* ((tree (org-element-parse-buffer))
         (links (org-element-map tree 'link
                  (lambda (l)
                    (list (org-element-property :type l)
                          (org-element-property :path l)
                          (org-element-property :raw-link l))))))
    links))"##,
        expect_test::expect![[
            r#""OK ((\"https\" \"//example.com/path?q=1&r=2\" \"https://example.com/path?q=1&r=2\") (\"file\" \"name with spaces.org\" \"file:name with spaces.org\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: unicode content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_unicode_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO 任务一\n* DONE 任务二\n* WAITING 任务三")
  (goto-char (point-min))
  (let ((h1 (list (org-get-todo-state) (org-get-heading t t t t))))
    (forward-line)
    (let ((h2 (list (org-get-todo-state) (org-get-heading t t t t))))
      (forward-line)
      (let ((h3 (list (org-get-todo-state) (org-get-heading t t t t))))
        (list h1 h2 h3)))))"##,
        expect_test::expect![[
            r#""OK ((\"TODO\" \"任务一\") (\"DONE\" \"任务二\") (nil \"WAITING 任务三\"))""#
        ]],
    );
}

#[test]
fn strong_unicode_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:VAR: 値\n:NAME: 名前\n:END:")
  (goto-char (point-min))
  (let ((v1 (org-entry-get nil "VAR"))
        (v2 (org-entry-get nil "NAME")))
    (org-entry-put nil "VAR" "新しい値")
    (let ((v3 (org-entry-get nil "VAR")))
      (list v1 v2 v3))))"##,
        expect_test::expect![[r#""OK (\"値\" \"名前\" \"新しい値\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: table boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_single_cell() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| only |")
  (let ((data (org-table-to-lisp))
        (rows (org-table-current-line))
        (cols (org-table-current-column)))
    (list data rows cols)))"##,
        expect_test::expect![[r#""OK (((\"only\")) 1 2)""#]],
    );
}

#[test]
fn strong_table_empty_rows() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|   |   |\n| c | d |")
  (goto-char (point-min))
  (let ((data (org-table-to-lisp)))
    (org-table-next-row)
    (org-table-put 2 1 "X")
    (let ((data2 (org-table-to-lisp)))
      (list data data2))))"##,
        expect_test::expect![[
            r#""OK (((\"a\" \"b\") (\"\" \"\") (\"c\" \"d\")) ((#(\"a\" 0 1 (face org-table)) #(\"b\" 0 1 (face org-table))) (\"X\" \"\") (#(\"c\" 0 1 (face org-table)) #(\"d\" 0 1 (face org-table)))))""#
        ]],
    );
}

#[test]
fn strong_table_formula_division() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| 10 | 3 |\n| 7 | 2 |\n#+TBLFM: $3=$1/$2")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (let ((r1 (org-table-get 1 3))
        (r2 (org-table-get 2 3)))
    (list r1 r2)))"##,
        expect_test::expect![[
            r#""OK (#(\"3.3333333\" 0 9 (face org-table)) #(\"3.5\" 0 3 (face org-table)))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: list boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_single_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- single item")
  (let* ((tree (org-element-parse-buffer))
         (lists (org-element-map tree 'plain-list
                  (lambda (pl)
                    (list (org-element-property :type pl)
                          (length (org-element-contents pl)))))))
    lists))"##,
        expect_test::expect![[r#""OK ((unordered 1))""#]],
    );
}

#[test]
fn strong_list_deeply_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- L1\n  - L2\n    - L3\n      - L4\n        - L5")
  (let* ((tree (org-element-parse-buffer))
         (items (org-element-map tree 'item
                  (lambda (it)
                    (list (org-element-property :bullet it)
                          (org-element-property :level it))))))
    items))"##,
        expect_test::expect![[
            r#""OK ((\"- \" nil) (\"- \" nil) (\"- \" nil) (\"- \" nil) (\"- \" nil))""#
        ]],
    );
}

#[test]
fn strong_list_mixed_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- plain\n1. ordered\n+ bullet\n- another plain\n2. ordered 2")
  (let* ((tree (org-element-parse-buffer))
         (lists (org-element-map tree 'plain-list
                  (lambda (pl)
                    (list (org-element-property :type pl)
                          (length (org-element-contents pl)))))))
    lists))"##,
        expect_test::expect![[r#""OK ((unordered 5))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: footnote boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1: inline definition] more text")
  (let* ((tree (org-element-parse-buffer))
         (fn (car (org-element-map tree 'footnote-reference
                    (lambda (f) f)))))
    (list (org-element-property :label fn)
          (org-element-property :type fn))))"##,
        expect_test::expect![[r#""OK (\"1\" inline)""#]],
    );
}

#[test]
fn strong_footnote_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1]\n\n[fn:1] ")
  (let* ((tree (org-element-parse-buffer))
         (fn (car (org-element-map tree 'footnote-reference
                    (lambda (f) f))))
         (def (car (org-element-map tree 'footnote-definition
                     (lambda (fd) fd)))))
    (list (org-element-property :label fn)
          (org-element-property :label def))))"##,
        expect_test::expect![[r#""OK (\"1\" \"1\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: timestamp boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timestamp_active_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<2026-01-15>")
  (let* ((tree (org-element-parse-buffer))
         (ts (car (org-element-map tree 'timestamp (lambda (t) t)))))
    (list (org-element-property :type ts)
          (org-element-property :year-start ts)
          (org-element-property :month-start ts)
          (org-element-property :day-start ts))))"##,
        expect_test::expect![[r#""OK (active 2026 1 15)""#]],
    );
}

#[test]
fn strong_timestamp_inactive_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[2026-01-15 14:30]")
  (let* ((tree (org-element-parse-buffer))
         (ts (car (org-element-map tree 'timestamp (lambda (t) t)))))
    (list (org-element-property :type ts)
          (org-element-property :year-start ts)
          (org-element-property :hour-start ts)
          (org-element-property :minute-start ts))))"##,
        expect_test::expect![[r#""OK (inactive 2026 14 30)""#]],
    );
}

#[test]
fn strong_timestamp_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<2026-01-15>--<2026-01-20>")
  (let* ((tree (org-element-parse-buffer))
         (ts (car (org-element-map tree 'timestamp (lambda (t) t)))))
    (list (org-element-property :type ts)
          (org-element-property :year-start ts)
          (org-element-property :day-start ts)
          (org-element-property :year-end ts)
          (org-element-property :day-end ts))))"##,
        expect_test::expect![[r#""OK (active-range 2026 15 2026 20)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: block boundary conditions
// ═════════════════════0═════════════════════════════════════════════════

#[test]
fn strong_block_empty_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (block (car (org-element-map tree 'src-block (lambda (b) b)))))
    (list (org-element-property :language block)
          (org-element-property :value block))))"##,
        expect_test::expect![[r#""OK (\"emacs-lisp\" \"\")""#]],
    );
}

#[test]
fn strong_block_multiple_languages() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC python\nprint('hello')\n#+END_SRC\n\n#+BEGIN_SRC emacs-lisp\n(message \"hi\")\n#+END_SRC\n\n#+BEGIN_SRC shell\necho test\n#+END_SRC")
  (let* ((tree (org-element-parse-buffer))
         (blocks (org-element-map tree 'src-block
                   (lambda (b)
                     (list (org-element-property :language b)
                           (org-element-property :value b))))))
    blocks))"##,
        expect_test::expect![[
            r#""OK ((\"python\" \"print('hello')\n\") (\"emacs-lisp\" \"(message \\\"hi\\\")\n\") (\"shell\" \"echo test\n\"))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: drawer boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_empty_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:END:")
  (let* ((tree (org-element-parse-buffer))
         (drawer (car (org-element-map tree 'drawer (lambda (d) d)))))
    (list (org-element-property :drawer-name drawer))))"##,
        expect_test::expect![[r#""OK (nil)""#]],
    );
}

#[test]
fn strong_drawer_multiple_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:A: 1\n:B: 2\n:C: 3\n:D: 4\n:E: 5\n:END:")
  (goto-char (point-min))
  (let ((props (org-entry-properties nil 'standard)))
    (list (length props)
          (alist-get "A" props nil nil 'equal)
          (alist-get "E" props nil nil 'equal))))"##,
        expect_test::expect![[r#""OK (6 \"1\" \"5\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: link boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_link_no_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[https://example.com]]")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l)))))
    (list (org-element-property :type link)
          (org-element-property :path link)
          (org-element-property :raw-link link))))"##,
        expect_test::expect![[r#""OK (\"https\" \"//example.com\" \"https://example.com\")""#]],
    );
}

#[test]
fn strong_link_angle_brackets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<https://example.com>")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l)))))
    (list (org-element-property :type link)
          (org-element-property :path link))))"##,
        expect_test::expect![[r#""OK (\"https\" \"//example.com\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: planning boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_deadline_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\nDEADLINE: <2026-01-20>")
  (goto-char (point-min))
  (let ((dl (org-entry-get nil "DEADLINE"))
        (sched (org-entry-get nil "SCHEDULED"))
        (closed (org-entry-get nil "CLOSED")))
    (list dl sched closed)))"##,
        expect_test::expect![[r#""OK (\"<2026-01-20>\" nil nil)""#]],
    );
}

#[test]
fn strong_planning_closed_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* DONE Task\nCLOSED: [2026-01-15]")
  (goto-char (point-min))
  (let ((dl (org-entry-get nil "DEADLINE"))
        (sched (org-entry-get nil "SCHEDULED"))
        (closed (org-entry-get nil "CLOSED")))
    (list dl sched closed)))"##,
        expect_test::expect![[r#""OK (nil nil \"[2026-01-15]\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: tag boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_tag_no_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Simple heading")
  (goto-char (point-min))
  (let ((tags (org-get-tags nil t)))
    tags))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn strong_tag_multiple_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading :tag1:tag2:tag3:")
  (goto-char (point-min))
  (let ((tags (org-get-tags nil t)))
    (org-set-tags '("new1" "new2"))
    (let ((tags2 (org-get-tags nil t)))
      (list tags tags2))))"##,
        expect_test::expect![[r#""OK ((\"tag1\" \"tag2\" \"tag3\") (\"new1\" \"new2\"))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: priority boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_priority_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO No priority")
  (goto-char (point-min))
  (let ((p (org-get-priority (char-after))))
    p))"##,
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

#[test]
fn strong_priority_all_levels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] High\n* TODO [#B] Medium\n* TODO [#C] Low")
  (goto-char (point-min))
  (let ((p1 (org-get-priority (char-after))))
    (forward-line)
    (let ((p2 (org-get-priority (char-after))))
      (forward-line)
      (let ((p3 (org-get-priority (char-after))))
        (list p1 p2 p3)))))"##,
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: todo boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_todo_no_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Just a heading")
  (goto-char (point-min))
  (let ((todo (org-get-todo-state)))
    todo))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn strong_todo_custom_keywords() {
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
        (let ((s4 (org-get-todo-state)))
          (list s1 s2 s3 s4))))))"##,
        expect_test::expect![[
            r#""OK (\"TODO\" #(\"PROG\" 0 4 (org-todo-head \"TODO\")) #(\"DONE\" 0 4 (org-todo-head \"TODO\")) nil)""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: visibility boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_visibility_single_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Only heading")
  (goto-char (point-min))
  (let ((hidden (get-char-property (point) 'invisible)))
    (org-cycle)
    (let ((hidden2 (get-char-property (point) 'invisible)))
      (list hidden hidden2))))"##,
        expect_test::expect![[r#""OK (nil nil)""#]],
    );
}

#[test]
fn strong_visibility_multiple_cycles() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (let ((states '()))
    ;; overview
    (org-set-startup-visibility 'overview)
    (push (get-char-property (search-forward "H2") 'invisible) states)
    ;; content
    (org-set-startup-visibility 'content)
    (push (get-char-property (search-forward "H2") 'invisible) states)
    ;; all
    (org-set-startup-visibility 'all)
    (push (get-char-property (search-forward "H2") 'invisible) states)
    (nreverse states)))"##,
        expect_test::expect![[r#""ERR (wrong-number-of-arguments (0 . 0) 1)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: property boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_empty_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:EMPTY: \n:END:")
  (goto-char (point-min))
  (let ((v (org-entry-get nil "EMPTY")))
    (list v)))"##,
        expect_test::expect![[r#""OK (\"\")""#]],
    );
}

#[test]
fn strong_property_overwrite() {
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
      (let ((v3 (org-entry-get nil "VAR")))
        (list v1 v2 v3)))))"##,
        expect_test::expect![[r#""OK (\"old\" \"new\" \"final\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: macro boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_macro_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greeting Hello!\n{{{greeting}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (let ((expanded (buffer-string)))
      (list raw expanded))))"##,
        expect_test::expect![[r#""ERR (error \"Undefined Org macro: greeting; aborting\")""#]],
    );
}

#[test]
fn strong_macro_multiple_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greet Hello $1 and $2!\n{{{greet(Alice, Bob)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (let ((expanded (buffer-string)))
      (list raw expanded))))"##,
        expect_test::expect![[r#""ERR (error \"Undefined Org macro: greet; aborting\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: dynamic block boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_dynamic_block_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN: clocktable\n#+END:")
  (goto-char (point-min))
  (org-dblock-update)
  (let ((content (buffer-string)))
    content))"##,
        expect_test::expect![[
            r##""OK #(\"#+BEGIN: clocktable\n#+CAPTION: Clock summary at [FIXED-TIME]\n| Headline     | Time   |\n|--------------+--------|\n| *Total time* | *0:00* |\n#+END:\" 61 62 (face org-table) 62 63 (face org-table rear-nonsticky t display (space :relative-width 1)) 63 71 (face org-table) 71 75 (face org-table) 75 76 (face org-table display (space :relative-width 1.001)) 76 77 (face org-table) 77 78 (face org-table rear-nonsticky t display (space :relative-width 1)) 78 82 (face org-table) 82 84 (face org-table) 84 85 (face org-table display (space :relative-width 1.001)) 85 86 (face org-table) 86 87 (face org-table-row) 87 88 (face org-table) 88 112 (face org-table) 112 113 (face org-table-row) 113 114 (face org-table) 114 115 (face org-table rear-nonsticky t display (space :relative-width 1)) 115 127 (org-emphasis t font-lock-multiline t face (bold org-table)) 127 128 (face org-table display (space :relative-width 1.001)) 128 129 (face org-table) 129 130 (face org-table rear-nonsticky t display (space :relative-width 1)) 130 136 (org-emphasis t font-lock-multiline t face (bold org-table)) 136 137 (face org-table display (space :relative-width 1.001)) 137 138 (face org-table) 138 139 (face org-table-row))""##
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: structure template boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_structure_template_various() {
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
    (let ((s2 (buffer-string)))
      (list s1 s2))))"##,
        expect_test::expect![[r#""ERR (void-function org-try-structure-completion)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: comment boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_comment_single_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# Comment line")
  (let* ((tree (org-element-parse-buffer))
         (comments (org-element-map tree 'comment
                     (lambda (c) (org-element-property :value c)))))
    comments))"##,
        expect_test::expect![[r#""OK (\"Comment line\")""#]],
    );
}

#[test]
fn strong_comment_multiple_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# Comment 1\n# Comment 2\n# Comment 3")
  (let* ((tree (org-element-parse-buffer))
         (comments (org-element-map tree 'comment
                     (lambda (c) (org-element-property :value c)))))
    comments))"##,
        expect_test::expect![[r#""OK (\"Comment 1\nComment 2\nComment 3\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: fixed-width boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_fixed_width_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert ": Fixed width")
  (let* ((tree (org-element-parse-buffer))
         (fw (car (org-element-map tree 'fixed-width
                    (lambda (f) f)))))
    (org-element-property :value fw))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn strong_fixed_width_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert ": Line 1\n: Line 2\n: Line 3")
  (let* ((tree (org-element-parse-buffer))
         (fw (org-element-map tree 'fixed-width
               (lambda (f) (org-element-property :value f)))))
    fw))"##,
        expect_test::expect![[r#""OK (\"Line 1\nLine 2\nLine 3\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: export boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_empty_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil)))
    (list (plist-get info :title))))"##,
        expect_test::expect![[r#""OK (nil)""#]],
    );
}

#[test]
fn strong_export_title_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: My Title")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil)))
    (list (plist-get info :title))))"##,
        expect_test::expect![[
            r#""OK ((#(\"My Title\" 0 8 (:parent (#(\"My Title\" 0 8 (:parent #4)))))))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: element boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_buffer_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-min))
  (let* ((el (org-element-at-point))
         (type (org-element-type el))
         (begin (org-element-property :begin el)))
    (list type begin)))"##,
        expect_test::expect![[r#""OK (headline 1)""#]],
    );
}

#[test]
fn strong_element_buffer_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-max))
  (let* ((el (org-element-at-point))
         (type (org-element-type el)))
    type))"##,
        expect_test::expect![[r#""OK paragraph""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: clock boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_no_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task with no clock")
  (let ((clocking (org-clocking-p)))
    clocking))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: statistics boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_statistics_no_cookies() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task\n- [ ] item 1\n- [ ] item 2")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    h))"##,
        expect_test::expect![[r#""OK \"* Task\"""#]],
    );
}

#[test]
fn strong_statistics_all_checked() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [0/0]\n- [X] item 1\n- [X] item 2\n- [X] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (let ((h (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    h))"##,
        expect_test::expect![[r#""OK \"* Task [3/3]\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: sparse tree boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_no_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task 1\n* TODO Task 2")
  (goto-char (point-min))
  (org-match-sparse-tree nil "DONE")
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
        expect_test::expect![[r#""OK ((\"Task 1\" \"Task 2\") nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ultimate edge: table transpose boundary conditions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_single_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |")
  (let ((data (org-table-to-lisp)))
    data))"##,
        expect_test::expect![[r#""OK ((\"a\" \"b\" \"c\"))""#]],
    );
}

#[test]
fn strong_table_single_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a |\n| b |\n| c |")
  (let ((data (org-table-to-lisp)))
    data))"##,
        expect_test::expect![[r#""OK ((\"a\") (\"b\") (\"c\"))""#]],
    );
}
