//! Strong divergence-hunt oracle tests — targeted at known bugs.
//!
//! Every test returns concrete structured data to surface divergences.
//! Specifically targets:
//! - parent property nesting in title/caption
//! - drawer parsing "Invalid search bound"
//! - clock-in "Invalid search bound"
//! - minibuffer message differences

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// Divergence: parent nesting in various element types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_parent_nesting_headline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test heading")
  (goto-char (point-min))
  (let* ((el (org-element-at-point))
         (title (org-element-property :raw-value el)))
    title))"##,
    );
}

#[test]
fn strong_parent_nesting_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: My Title")
  (let* ((tree (org-element-parse-buffer))
         (kw (car (org-element-map tree 'keyword (lambda (k) k))))
         (val (org-element-property :value kw)))
    val))"##,
    );
}

#[test]
fn strong_parent_nesting_caption() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: My caption\n[[file:test.png]]")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l))))
         (parent (org-element-property :parent link))
         (caption (org-element-property :caption parent)))
    caption))"##,
    );
}

#[test]
fn strong_parent_nesting_title_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Export Title\n* H1\n** H2")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment nil))
         (title (plist-get info :title)))
    title))"##,
    );
}

#[test]
fn strong_parent_nesting_multiple_headlines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3")
  (let* ((tree (org-element-parse-buffer))
         (titles (org-element-map tree 'headline
                   (lambda (h) (org-element-property :raw-value h)))))
    titles))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: drawer parsing with various content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_drawer_parse_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (drawers (org-element-map tree 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    drawers))"##,
    );
}

#[test]
fn strong_drawer_parse_with_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (drawers (org-element-map tree 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    drawers))"##,
    );
}

#[test]
fn strong_drawer_parse_logbook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\n- Note\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (drawers (org-element-map tree 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    drawers))"##,
    );
}

#[test]
fn strong_drawer_parse_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:\n:LOGBOOK:\n- Note\n:END:\nBody")
  (let* ((tree (org-element-parse-buffer))
         (drawers (org-element-map tree 'drawer
                    (lambda (d) (org-element-property :drawer-name d)))))
    drawers))"##,
    );
}

#[test]
fn strong_drawer_parse_properties_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
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
// Divergence: clock-in with various states
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_in_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Test")
  (goto-char (point-min))
  (let ((clocking (org-clocking-p)))
    clocking))"##,
    );
}

#[test]
fn strong_clock_in_out() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
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
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: export environment with various keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_env_title_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Only Title")
  (let* ((info (org-export-get-environment nil))
         (title (plist-get info :title)))
    title))"##,
    );
}

#[test]
fn strong_export_env_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+AUTHOR: A\n#+EMAIL: e\n#+OPTIONS: toc:nil")
  (let* ((info (org-export-get-environment nil)))
    (list (plist-get info :title)
          (plist-get info :author)
          (plist-get info :email)
          (plist-get info :with-toc))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: element map with various types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_map_headline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (org-element-property :raw-value h))))"##,
    );
}

#[test]
fn strong_element_map_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: T\n#+AUTHOR: A")
  (org-element-map (org-element-parse-buffer) 'keyword
    (lambda (k) (list (org-element-property :key k)
                      (org-element-property :value k)))))"##,
    );
}

#[test]
fn strong_element_map_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "[[https://x][web]] [[file:f.org][file]]")
  (org-element-map (org-element-parse-buffer) 'link
    (lambda (l) (list (org-element-property :type l)
                      (org-element-property :path l)))))"##,
    );
}

#[test]
fn strong_element_map_paragraph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Para 1\n\nPara 2")
  (org-element-map (org-element-parse-buffer) 'paragraph
    (lambda (p) (org-element-property :value p))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: property access patterns
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_property_access_standard() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (org-entry-properties nil 'standard))"##,
    );
}

#[test]
fn strong_property_access_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:")
  (goto-char (point-min))
  (org-entry-get nil "A"))"##,
    );
}

#[test]
fn strong_property_access_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P\n:PROPERTIES:\n:V: parent\n:END:\n** C")
  (goto-char (point-min))
  (search-forward "C")
  (org-entry-get nil "V" 'inherit))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: tag operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_tag_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H :a:b:")
  (goto-char (point-min))
  (org-get-tags nil t))"##,
    );
}

#[test]
fn strong_tag_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (goto-char (point-min))
  (org-set-tags '("a" "b"))
  (org-get-tags nil t))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: todo operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_todo_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T")
  (goto-char (point-min))
  (org-get-todo-state))"##,
    );
}

#[test]
fn strong_todo_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T")
  (goto-char (point-min))
  (let ((s1 (org-get-todo-state)))
    (org-todo 'right)
    (let ((s2 (org-get-todo-state)))
      (list s1 s2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: priority operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_priority_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] T")
  (goto-char (point-min))
  (org-get-priority (char-after)))"##,
    );
}

#[test]
fn strong_priority_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] T")
  (goto-char (point-min))
  (let ((p1 (org-get-priority (char-after))))
    (org-priority 'down)
    (list p1 (org-get-priority (char-after)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: heading operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_heading_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title :tag:")
  (goto-char (point-min))
  (org-get-heading t t t t))"##,
    );
}

#[test]
fn strong_heading_edit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Original")
  (goto-char (point-min))
  (org-edit-headline "Changed")
  (org-get-heading t t t t))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: planning operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_planning_deadline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\nDEADLINE: <2026-01-20>")
  (goto-char (point-min))
  (org-entry-get nil "DEADLINE"))"##,
    );
}

#[test]
fn strong_planning_scheduled() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\nSCHEDULED: <2026-01-15>")
  (goto-char (point-min))
  (org-entry-get nil "SCHEDULED"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: table operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (org-table-to-lisp))"##,
    );
}

#[test]
fn strong_table_formula() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| 1 | 2 |\n| 3 | 4 |\n#+TBLFM: $3=$1+$2")
  (goto-char (point-min))
  (org-table-recalculate 'all)
  (org-table-to-lisp))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: list operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_list_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- item 1\n- item 2\n  - sub 1")
  (org-element-map (org-element-parse-buffer) 'item
    (lambda (it) (org-element-property :bullet it))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Divergence: visibility operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_visibility_overview() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (org-set-startup-visibility 'overview)
  (get-char-property (search-forward "H2") 'invisible))"##,
    );
}

#[test]
fn strong_visibility_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (org-set-startup-visibility 'all)
  (get-char-property (search-forward "H2") 'invisible))"##,
    );
}
