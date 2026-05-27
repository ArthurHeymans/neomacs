//! Epsilon-strict combo tests for org-mode complex interactions.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex headline parsing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_headline_todo_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* TODO Task\n* DONE Done\n* Normal\n* WAIT Wait")
      (goto-char (point-min))
      (mapcar (lambda (h) (org-element-property :todo-keyword h))
              (org-element-map (org-element-parse-buffer) 'headline #'identity)))))"##,
    );
}

#[test]
fn epsilon_headline_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H :tag1:tag2:\n** H2 :tag3:\n* H3")
      (goto-char (point-min))
      (mapcar (lambda (h) (org-element-property :tags h))
              (org-element-map (org-element-parse-buffer) 'headline #'identity)))))"##,
    );
}

#[test]
fn epsilon_headline_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* [#A] High\n* [#B] Medium\n* [#C] Low\n* No priority")
      (goto-char (point-min))
      (mapcar (lambda (h) (org-element-property :priority h))
              (org-element-map (org-element-parse-buffer) 'headline #'identity)))))"##,
    );
}

#[test]
fn epsilon_headline_commented() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* COMMENT Hidden\n* Visible\n** COMMENT Also hidden")
      (goto-char (point-min))
      (mapcar (lambda (h)
                (list (substring-no-properties (org-element-property :raw-value h))
                      (member "COMMENT" (org-element-property :tags h))))
              (org-element-map (org-element-parse-buffer) 'headline #'identity)))))"##,
    );
}

#[test]
fn epsilon_headline_statistics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Project [1/3]\n** S1\n** S2\n** S3\n* Progress [50%]\n** Done\n** Todo")
      (goto-char (point-min))
      (mapcar (lambda (h) (substring-no-properties (org-element-property :raw-value h)))
              (org-element-map (org-element-parse-buffer) 'headline #'identity)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex planning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_planning_deadline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\nDEADLINE: <2024-01-15 Mon>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (planning (car (org-element-map tree 'planning #'identity))))
        (org-element-property :deadline planning)))))"##,
    );
}

#[test]
fn epsilon_planning_scheduled() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\nSCHEDULED: <2024-01-15 Mon>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (planning (car (org-element-map tree 'planning #'identity))))
        (org-element-property :scheduled planning)))))"##,
    );
}

#[test]
fn epsilon_planning_closed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* DONE H\nCLOSED: [2024-01-14 Sun]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (planning (car (org-element-map tree 'planning #'identity))))
        (org-element-property :closed planning)))))"##,
    );
}

#[test]
fn epsilon_planning_all_three() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\nDEADLINE: <2024-01-15 Mon> SCHEDULED: <2024-01-14 Sun> CLOSED: [2024-01-13 Sat]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (planning (car (org-element-map tree 'planning #'identity))))
        (list (org-element-property :deadline planning)
              (org-element-property :scheduled planning)
              (org-element-property :closed planning))))))"##,
    );
}

#[test]
fn epsilon_planning_with_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\nSCHEDULED: <2024-01-15 Mon +1w>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (planning (car (org-element-map tree 'planning #'identity)))
             (ts (org-element-property :scheduled planning)))
        (list (org-element-property :repeater-type ts)
              (org-element-property :repeater-value ts)
              (org-element-property :repeater-unit ts))))))"##,
    );
}

#[test]
fn epsilon_planning_with_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\nDEADLINE: <2024-01-15 Mon -3d>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (planning (car (org-element-map tree 'planning #'identity)))
             (ts (org-element-property :deadline planning)))
        (list (org-element-property :warning-type ts)
              (org-element-property :warning-value ts)
              (org-element-property :warning-unit ts))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex property drawers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_property_drawer_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:KEY: val\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (drawer (car (org-element-map tree 'property-drawer #'identity))))
        (org-element-type drawer)))))"##,
    );
}

#[test]
fn epsilon_property_drawer_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (drawer (car (org-element-map tree 'property-drawer #'identity))))
        (org-element-type drawer)))))"##,
    );
}

#[test]
fn epsilon_property_drawer_with_extended() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:A+: 2\n:A: 1\n:A+: 3\n:END:")
      (goto-char (point-min))
      (org-entry-get (point) "A"))))"##,
    );
}

#[test]
fn epsilon_property_drawer_custom_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:CUSTOM_ID: myid\n:END:")
      (goto-char (point-min))
      (org-entry-get (point) "CUSTOM_ID"))))"##,
    );
}

#[test]
fn epsilon_property_drawer_effort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:EFFORT: 2:30\n:END:")
      (goto-char (point-min))
      (org-entry-get (point) "EFFORT"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex drawers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_drawer_logbook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:LOGBOOK:\nNote\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (drawer (car (org-element-map tree 'drawer #'identity))))
        (org-element-property :drawer-name drawer)))))"##,
    );
}

#[test]
fn epsilon_drawer_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:MYDRAWER:\nContent\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (drawer (car (org-element-map tree 'drawer #'identity))))
        (org-element-property :drawer-name drawer)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex clocks
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_clock_closed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "CLOCK: [2024-01-15 Mon 09:00]--[2024-01-15 Mon 10:30] =>  1:30")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (clock (car (org-element-map tree 'clock #'identity))))
        (list (org-element-property :status clock)
              (org-element-property :duration clock))))))"##,
    );
}

#[test]
fn epsilon_clock_running() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "CLOCK: [2024-01-15 Mon 13:00]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (clock (car (org-element-map tree 'clock #'identity))))
        (org-element-property :status clock)))))"##,
    );
}

#[test]
fn epsilon_clock_in_logbook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:LOGBOOK:\nCLOCK: [2024-01-15 Mon 09:00]--[2024-01-15 Mon 10:00] =>  1:00\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (clocks (org-element-map tree 'clock #'identity)))
        (length clocks)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex blocks
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_src_block_with_language() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (src (car (org-element-map tree 'src-block #'identity))))
        (org-element-property :language src)))))"##,
    );
}

#[test]
fn epsilon_src_block_with_switches() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_SRC emacs-lisp -n -r\n(+ 1 2)\n#+END_SRC")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (src (car (org-element-map tree 'src-block #'identity))))
        (org-element-property :switches src)))))"##,
    );
}

#[test]
fn epsilon_src_block_with_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_SRC emacs-lisp :results output :exports code\n(+ 1 2)\n#+END_SRC")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (src (car (org-element-map tree 'src-block #'identity))))
        (org-element-property :parameters src)))))"##,
    );
}

#[test]
fn epsilon_quote_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_QUOTE\nQuoted text.\n#+END_QUOTE")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (length (org-element-map tree 'quote-block #'identity)))))"##,
    );
}

#[test]
fn epsilon_center_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_CENTER\nCentered.\n#+END_CENTER")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (length (org-element-map tree 'center-block #'identity)))))"##,
    );
}

#[test]
fn epsilon_example_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_EXAMPLE\nExample.\n#+END_EXAMPLE")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (length (org-element-map tree 'example-block #'identity)))))"##,
    );
}

#[test]
fn epsilon_export_block_html() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_EXPORT html\n<p>Text</p>\n#+END_EXPORT")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (block (car (org-element-map tree 'export-block #'identity))))
        (org-element-property :type block)))))"##,
    );
}

#[test]
fn epsilon_verse_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_VERSE\nLine one\nLine two\n#+END_VERSE")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (length (org-element-map tree 'verse-block #'identity)))))"##,
    );
}

#[test]
fn epsilon_comment_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_COMMENT\nComment.\n#+END_COMMENT")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (length (org-element-map tree 'comment-block #'identity)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex lists
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_list_unordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "- Item 1\n- Item 2\n  - Sub 1\n  - Sub 2\n- Item 3")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list (length (org-element-map tree 'plain-list #'identity))
              (length (org-element-map tree 'item #'identity))))))"##,
    );
}

#[test]
fn epsilon_list_ordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "1. First\n2. Second\n3. Third")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (lists (org-element-map tree 'plain-list #'identity)))
        (mapcar (lambda (l) (org-element-property :type l)) lists))))"##,
    );
}

#[test]
fn epsilon_list_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "- tag1 :: desc1\n- tag2 :: desc2")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (items (org-element-map tree 'item #'identity)))
        (mapcar (lambda (i)
                  (substring-no-properties
                   (org-element-interpret-data (org-element-property :tag i))))
                items))))"##,
    );
}

#[test]
fn epsilon_list_checkboxes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "- [ ] Unchecked\n- [X] Checked\n- [-] Partial\n- No box")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (items (org-element-map tree 'item #'identity)))
        (mapcar (lambda (i) (org-element-property :checkbox i)) items))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex tables
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_table_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| a | b |\n| c | d |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list (length (org-element-map tree 'table #'identity))
              (length (org-element-map tree 'table-row #'identity))
              (length (org-element-map tree 'table-cell #'identity))))))"##,
    );
}

#[test]
fn epsilon_table_with_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| a | b |\n|---+---|\n| c | d |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (rows (org-element-map tree 'table-row #'identity)))
        (mapcar (lambda (r) (org-element-property :type r)) rows))))"##,
    );
}

#[test]
fn epsilon_table_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| a | b |")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (table (car (org-element-map tree 'table #'identity))))
        (org-element-property :type table)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex links
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_link_https() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[https://orgmode.org][Org mode]]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (link (car (org-element-map tree 'link #'identity))))
        (list (org-element-property :type link)
              (org-element-property :path link))))))"##,
    );
}

#[test]
fn epsilon_link_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[file:path/to/file.org][file link]]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (link (car (org-element-map tree 'link #'identity))))
        (org-element-property :type link)))))"##,
    );
}

#[test]
fn epsilon_link_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[id:uuid-1234][id link]]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (link (car (org-element-map tree 'link #'identity))))
        (org-element-property :type link)))))"##,
    );
}

#[test]
fn epsilon_link_custom_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[#custom-id][custom link]]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (link (car (org-element-map tree 'link #'identity))))
        (org-element-property :type link)))))"##,
    );
}

#[test]
fn epsilon_link_plain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "https://orgmode.org")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (links (org-element-map tree 'link #'identity)))
        (length links))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex timestamps
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_timestamp_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2024-01-15 Mon>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (org-element-property :type ts))))"##,
    );
}

#[test]
fn epsilon_timestamp_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[2024-01-15 Mon]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (org-element-property :type ts))))"##,
    );
}

#[test]
fn epsilon_timestamp_with_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2024-01-15 Mon 14:30>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (list (org-element-property :hour-start ts)
              (org-element-property :minute-start ts))))))"##,
    );
}

#[test]
fn epsilon_timestamp_active_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2024-01-15 Mon>--<2024-01-16 Tue>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (list (org-element-property :type ts)
              (org-element-property :year-start ts)
              (org-element-property :year-end ts))))))"##,
    );
}

#[test]
fn epsilon_timestamp_timerange() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2024-01-15 Mon 14:30-15:30>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (list (org-element-property :hour-start ts)
              (org-element-property :minute-start ts)
              (org-element-property :hour-end ts)
              (org-element-property :minute-end ts))))))"##,
    );
}

#[test]
fn epsilon_timestamp_with_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2024-01-15 Mon +1w>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (list (org-element-property :repeater-type ts)
              (org-element-property :repeater-value ts)
              (org-element-property :repeater-unit ts))))))"##,
    );
}

#[test]
fn epsilon_timestamp_with_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2024-01-15 Mon -3d>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (list (org-element-property :warning-type ts)
              (org-element-property :warning-value ts)
              (org-element-property :warning-unit ts))))))"##,
    );
}

#[test]
fn epsilon_timestamp_diary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<%%(diary-float t 4 2)>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (ts (car (org-element-map tree 'timestamp #'identity))))
        (org-element-property :type ts))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex entities
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_entity_alpha() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\alpha")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (entity (car (org-element-map tree 'entity #'identity))))
        (org-element-property :name entity)))))"##,
    );
}

#[test]
fn epsilon_entity_beta() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\beta")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (entity (car (org-element-map tree 'entity #'identity))))
        (org-element-property :name entity)))))"##,
    );
}

#[test]
fn epsilon_entity_with_braces() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\alpha{}text")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (entities (org-element-map tree 'entity #'identity)))
        (length entities))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex LaTeX
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_latex_fragment_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "$x^2$")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (fragments (org-element-map tree 'latex-fragment #'identity)))
        (length fragments))))"##,
    );
}

#[test]
fn epsilon_latex_fragment_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "$$E = mc^2$$")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (fragments (org-element-map tree 'latex-fragment #'identity)))
        (length fragments))))"##,
    );
}

#[test]
fn epsilon_latex_environment_equation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\begin{equation}\nx^2 + y^2 = z^2\n\\end{equation}")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (envs (org-element-map tree 'latex-environment #'identity)))
        (length envs))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex footnotes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_footnote_standard() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:1]\n\n[fn:1] Definition.")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer)))
        (list (length (org-element-map tree 'footnote-reference #'identity))
              (length (org-element-map tree 'footnote-definition #'identity))))))"##,
    );
}

#[test]
fn epsilon_footnote_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:name:inline def]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (refs (org-element-map tree 'footnote-reference #'identity)))
        (mapcar (lambda (r) (org-element-property :type r)) refs))))"##,
    );
}

#[test]
fn epsilon_footnote_anonymous() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn::anonymous def]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (refs (org-element-map tree 'footnote-reference #'identity)))
        (mapcar (lambda (r) (org-element-property :type r)) refs))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_target_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<<my-target>>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (targets (org-element-map tree 'target #'identity)))
        (length targets))))"##,
    );
}

#[test]
fn epsilon_radio_target() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<<<radio>>>")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (targets (org-element-map tree 'radio-target #'identity)))
        (length targets))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex statistics cookies
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_statistics_cookie_fraction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H [1/3]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (cookies (org-element-map tree 'statistics-cookie #'identity)))
        (mapcar (lambda (c) (org-element-property :value c)) cookies))))"##,
    );
}

#[test]
fn epsilon_statistics_cookie_percent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H [50%]")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (cookies (org-element-map tree 'statistics-cookie #'identity)))
        (mapcar (lambda (c) (org-element-property :value c)) cookies))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex macros
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_macro_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "{{{test}}}")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (macros (org-element-map tree 'macro #'identity)))
        (length macros))))"##,
    );
}

#[test]
fn epsilon_macro_with_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "{{{test(arg1,arg2)}}}")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (macros (org-element-map tree 'macro #'identity)))
        (mapcar (lambda (m) (org-element-property :value m)) macros))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex export snippets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_export_snippet_html() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "@@html:<b>bold</b>@@")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (snippets (org-element-map tree 'export-snippet #'identity)))
        (mapcar (lambda (s) (org-element-property :back-end s)) snippets))))"##,
    );
}

#[test]
fn epsilon_export_snippet_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "@@latex:\\textbf{bold}@@")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (snippets (org-element-map tree 'export-snippet #'identity)))
        (mapcar (lambda (s) (org-element-property :back-end s)) snippets))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex inlinetasks
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_inlinetask_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-inlinetask)
  (let ((org-mode-hook nil)
        (org-inlinetask-min-level 15))
    (with-temp-buffer (org-mode)
      (insert "*************** TODO Inline task\nBody\n*************** END")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (tasks (org-element-map tree 'inlinetask #'identity)))
        (mapcar (lambda (t) (org-element-property :todo-keyword t)) tasks))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex diary sexps
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_diary_sexp_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "%%(org-anniversary 1956 5 14) Arthur Dent is %d years old")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (sexps (org-element-map tree 'diary-sexp #'identity)))
        (length sexps))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex horizontal rules
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_horizontal_rule_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Above\n\n-----\n\nBelow")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (rules (org-element-map tree 'horizontal-rule #'identity)))
        (length rules))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Epsilon: org-element with complex line breaks
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn epsilon_line_break_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Line 1\\\\\nLine 2")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (breaks (org-element-map tree 'line-break #'identity)))
        (length breaks))))"##,
    );
}
