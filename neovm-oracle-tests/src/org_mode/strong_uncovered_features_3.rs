//! Strong uncovered-features-3 oracle tests — test features not yet tested.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-shifttab cycling
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_shifttab_cycling() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\nBody")
  (goto-char (point-min))
  (let ((s '()))
    (org-shifttab)
    (push (list :after-shifttab
                (get-char-property (search-forward "H2") 'invisible)
                (progn (forward-line) (get-char-property (point) 'invisible)))
          s)
    (nreverse s)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-meta-return at different positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_meta_return_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2")
  (goto-char (point-min))
  (end-of-line)
  (org-meta-return)
  (insert "New")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (org-element-property :raw-value h))))"##,
    );
}

#[test]
fn uf3_meta_return_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B")
  (goto-char (point-min))
  (end-of-line)
  (org-meta-return)
  (insert "New")
  (org-element-map (org-element-parse-buffer) 'item
    (lambda (i) (org-trim (buffer-substring-no-properties
                            (org-element-property :contents-begin i)
                            (org-element-property :contents-end i))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-shiftmetaright / org-shiftmetaleft
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_shiftmeta_right_left() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n* H2\n* H3")
  (goto-char (point-min))
  (forward-line 1)
  (org-shiftmetaright)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'headline
              (lambda (h) (list (org-element-property :level h)
                                (org-element-property :raw-value h))))))
    (org-shiftmetaleft)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'headline
                (lambda (h) (list (org-element-property :level h)
                                  (org-element-property :raw-value h))))))
      (list d1 d2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-shiftmetaup / org-shiftmetadown
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_shiftmeta_up_down() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n* B\n* C")
  (goto-char (point-min))
  (forward-line 1)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'headline
              (lambda (h) (org-element-property :raw-value h)))))
    (org-shiftmetadown)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'headline
                (lambda (h) (org-element-property :raw-value h)))))
      (org-shiftmetaup)
      (let ((d3 (org-element-map (org-element-parse-buffer) 'headline
                  (lambda (h) (org-element-property :raw-value h)))))
        (list d1 d2 d3)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-metaright / org-metaleft on list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_meta_right_left_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (forward-line 1)
  (org-metaright)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'item
              (lambda (i) (list (org-element-property :bullet i)
                                (org-element-property :level i))))))
    (org-metaleft)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'item
                (lambda (i) (list (org-element-property :bullet i)
                                  (org-element-property :level i))))))
      (list d1 d2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-metaup / org-metadown on list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_meta_up_down_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (forward-line 1)
  (let ((d1 (org-element-map (org-element-parse-buffer) 'item
              (lambda (i) (org-trim (buffer-substring-no-properties
                                      (org-element-property :contents-begin i)
                                      (org-element-property :contents-end i)))))))
    (org-metadown)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'item
                (lambda (i) (org-trim (buffer-substring-no-properties
                                        (org-element-property :contents-begin i)
                                        (org-element-property :contents-end i)))))))
      (list d1 d2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-return-at-point
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_return_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- item")
  (goto-char (point-max))
  (org-return)
  (insert "new")
  (org-element-map (org-element-parse-buffer) 'item
    (lambda (i) (org-trim (buffer-substring-no-properties
                            (org-element-property :contents-begin i)
                            (org-element-property :contents-end i))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-delete-backward-char in list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_delete_in_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (forward-line 1)
  (beginning-of-line)
  (org-delete-backward-char 1)
  (org-element-map (org-element-parse-buffer) 'item
    (lambda (i) (org-trim (buffer-substring-no-properties
                            (org-element-property :contents-begin i)
                            (org-element-property :contents-end i))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-open-at-point on link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_open_at_point_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[https://example.com][web]]")
  (goto-char (point-min))
  (search-forward "[[")
  (let ((ctx (org-element-context)))
    (list (org-element-type ctx)
          (org-element-property :type ctx)
          (org-element-property :path ctx))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-context at different positions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_element_context() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody *bold* text")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :h (org-element-type (org-element-context))) r)
    (search-forward "bold")
    (push (list :bold (org-element-type (org-element-context))) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-at-point vs org-element-context
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_element_at_point_vs_context() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody *bold* text")
  (let ((r '()))
    (goto-char (point-min))
    (push (list :at-pt (org-element-type (org-element-at-point))
                :ctx (org-element-type (org-element-context)))
          r)
    (search-forward "bold")
    (push (list :at-pt (org-element-type (org-element-at-point))
                :ctx (org-element-type (org-element-context)))
          r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-get-category at different headings
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_get_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CATEGORY: default\n* H1\n:PROPERTIES:\n:CATEGORY: custom\n:END:\n** H2")
  (let ((r '()))
    (goto-char (point-min))
    (search-forward "H1")
    (push (list :h1 (org-get-category)) r)
    (search-forward "H2")
    (push (list :h2 (org-get-category)) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-get with ITEM
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_entry_get_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Heading :tag:")
  (goto-char (point-min))
  (list (org-entry-get nil "ITEM")
        (org-entry-get nil "TODO")
        (org-entry-get nil "PRIORITY")
        (org-entry-get nil "TAGS")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-get with SCHEDULED/DEADLINE/CLOSED
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_entry_get_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\nCLOSED: [2026-01-10]")
  (goto-char (point-min))
  (list (org-entry-get nil "SCHEDULED")
        (org-entry-get nil "DEADLINE")
        (org-entry-get nil "CLOSED")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-get with custom properties
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_entry_get_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:CUSTOM_ID: myid\n:EFFORT: 2h\n:VAR: test\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "CUSTOM_ID")
        (org-entry-get nil "EFFORT")
        (org-entry-get nil "VAR")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-properties with 'standard flag
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_entry_properties_standard() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (org-entry-properties nil 'standard))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-entry-put / org-entry-get / org-entry-delete
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_entry_put_get_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (org-entry-put nil "A" "1")
  (org-entry-put nil "B" "2")
  (let ((v1 (org-entry-get nil "A"))
        (v2 (org-entry-get nil "B")))
    (org-entry-delete nil "A")
    (list v1 v2 (org-entry-get nil "A") (org-entry-get nil "B"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-get-repeat
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_get_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO W\nSCHEDULED: <2026-01-15 +1w>\n* TODO M\nDEADLINE: <2026-01-20 +1m>\n* TODO N")
  (goto-char (point-min))
  (let ((r1 (org-get-repeat)))
    (forward-line 2)
    (let ((r2 (org-get-repeat)))
      (forward-line 2)
      (list r1 r2 (org-get-repeat)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-deadline with remove
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_deadline_set_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T")
  (goto-char (point-min))
  (org-deadline nil "2026-01-20")
  (let ((dl1 (org-entry-get nil "DEADLINE")))
    (org-deadline nil nil)
    (list dl1 (org-entry-get nil "DEADLINE"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-schedule with remove
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_schedule_set_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T")
  (goto-char (point-min))
  (org-schedule nil "2026-01-15")
  (let ((sc1 (org-entry-get nil "SCHEDULED")))
    (org-schedule nil nil)
    (list sc1 (org-entry-get nil "SCHEDULED"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-clock-sum-current-entry
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_clock_sum_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:30] =>  1:30\n:END:")
  (goto-char (point-min))
  (org-clock-sum-current-entry))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-columns-get-format
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_columns_format() {
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
// org-refile-get-targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_refile_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P1\n** T1\n*** S1\n* P2\n** T2")
  (mapcar 'car (org-refile-get-targets nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-map-entries with match
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_map_entries_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C")
  (list (org-map-entries (lambda () (org-get-heading t t t t)) "TODO" 'file)
        (org-map-entries (lambda () (org-get-heading t t t t)) "DONE" 'file)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-toggle-tag with arg
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_toggle_tag_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H :a:")
  (goto-char (point-min))
  (let ((t1 (org-get-tags nil t)))
    (org-toggle-tag "b" 'on)
    (let ((t2 (org-get-tags nil t)))
      (org-toggle-tag "a" 'off)
      (list t1 t2 (org-get-tags nil t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-priority with specific value
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_priority_specific() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] H\n* TODO H2")
  (goto-char (point-min))
  (let ((p1 (org-get-priority (char-after))))
    (org-priority ?B)
    (forward-line)
    (org-priority ?A)
    (list p1 (org-get-priority (char-after)) (progn (forward-line -1) (org-get-priority (char-after))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-todo cycle with custom keywords
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_todo_custom_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (setq org-todo-keywords '((sequence "IDEA" "WORKING" "DONE")))
  (insert "* IDEA T")
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
// org-sort-entries different keys
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_sort_entries_alpha() {
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
// org-move-subtree-down / org-move-subtree-up
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_move_subtree() {
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
// org-promote-subtree / org-demote-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_promote_demote_subtree() {
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
// org-clone-subtree with count
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_clone_subtree_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task\n** Sub1\n** Sub2")
  (goto-char (point-min))
  (org-clone-subtree 3)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :level h)
                      (org-element-property :raw-value h)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-copy-subtree / org-paste-subtree with level
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_copy_paste_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** Sub1\n* H2\n** Sub2")
  (goto-char (point-min))
  (org-copy-subtree)
  (goto-char (point-max))
  (org-paste-subtree 2)
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :level h)
                      (org-element-property :raw-value h)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-mark-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_mark_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody\n** H2\nSub\n* H1b")
  (goto-char (point-min))
  (org-mark-subtree)
  (let ((m (mark))
        (p (point)))
    (list (< p m) p m)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-narrow-to-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_narrow_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\nBody 1\n** H2\nSub\n* H2b\nBody 2")
  (goto-char (point-min))
  (org-narrow-to-subtree)
  (let ((narrowed (buffer-string)))
    (widen)
    (list narrowed)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-end-of-subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_end_of_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
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
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-cycle-hide-drawers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_cycle_hide_drawers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\n:PROPERTIES:\n:A: 1\n:END:\nBody\n* H2")
  (goto-char (point-min))
  (org-cycle-hide-drawers 'all)
  (let ((hidden1 (get-char-property (search-forward "A") 'invisible)))
    (goto-char (point-max))
    (org-cycle-hide-drawers nil)
    (let ((hidden2 (get-char-property (search-forward "A") 'invisible)))
      (list hidden1 hidden2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-toggle-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_toggle_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Plain text\n* Existing heading")
  (goto-char (point-min))
  (org-toggle-heading)
  (let ((s1 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
    (forward-line)
    (org-toggle-heading)
    (let ((s2 (buffer-substring-no-properties (line-beginning-position) (line-end-position))))
      (list s1 s2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-move-item
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_move_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- A\n- B\n- C")
  (goto-char (point-min))
  (let ((d1 (org-element-map (org-element-parse-buffer) 'item
              (lambda (i) (org-trim (buffer-substring-no-properties
                                      (org-element-property :contents-begin i)
                                      (org-element-property :contents-end i)))))))
    (org-move-item-down)
    (let ((d2 (org-element-map (org-element-parse-buffer) 'item
                (lambda (i) (org-trim (buffer-substring-no-properties
                                        (org-element-property :contents-begin i)
                                        (org-element-property :contents-end i)))))))
      (list d1 d2))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-insert-todo-heading
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_insert_todo_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Existing")
  (goto-char (point-max))
  (org-insert-todo-heading nil)
  (insert "New task")
  (org-insert-todo-heading 'right)
  (insert "Right task")
  (org-element-map (org-element-parse-buffer) 'headline
    (lambda (h) (list (org-element-property :raw-value h)
                      (org-element-property :todo-keyword h)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-update-statistics-cookies
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_update_statistics() {
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
// org-match-sparse-tree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_match_sparse_tree() {
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

// ═══════════════════════════════════════════════════════════════════════
// org-dblock-update
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_dblock_update() {
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
// org-macro-replace-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_macro_replace() {
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
// org-try-structure-completion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_try_structure() {
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
// org-clock-sum
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf3_clock_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\n:END:\n* B\n:LOGBOOK:\nCLOCK: [2026-01-11 14:00]--[2026-01-11 15:30] =>  1:30\n:END:")
  (org-clock-sum))"##,
    );
}
