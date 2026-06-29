//! Strong org-functions oracle tests — test org-mode functions.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn of_org_current_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3")
  (goto-char (point-min))
  (let ((l1 (org-current-level)))
    (forward-line)
    (let ((l2 (org-current-level)))
      (forward-line)
      (let ((l3 (org-current-level)))
        (list l1 l2 l3)))))"##,
        expect_test::expect![[r#""OK (1 2 3)""#]],
    );
}

#[test]
fn of_org_get_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Title :tag:")
  (goto-char (point-min))
  (list (org-get-heading t t t t)
        (org-get-heading nil nil nil t)
        (org-get-heading t nil nil t)
        (org-get-heading nil t nil t)))"##,
        expect_test::expect![[
            r#""OK (\"Title\" \"TODO [#A] Title :tag:\" \"TODO [#A] Title\" \"[#A] Title :tag:\")""#
        ]],
    );
}

#[test]
fn of_org_get_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H :a:b:c:")
  (goto-char (point-min))
  (list (org-get-tags nil t)
        (org-get-tags nil nil)))"##,
        expect_test::expect![[r#""OK ((\"a\" \"b\" \"c\") (\"a\" \"b\" \"c\"))""#]],
    );
}

#[test]
fn of_org_get_todo_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\n* DONE D\n* H")
  (goto-char (point-min))
  (list (org-get-todo-state)
        (progn (forward-line) (org-get-todo-state))
        (progn (forward-line) (org-get-todo-state))))"##,
        expect_test::expect![[r#""OK (\"TODO\" \"DONE\" nil)""#]],
    );
}

#[test]
fn of_org_get_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] H1\n* TODO [#B] H2\n* TODO H3")
  (goto-char (point-min))
  (list (org-get-priority (char-after))
        (progn (forward-line) (org-get-priority (char-after)))
        (progn (forward-line) (org-get-priority (char-after)))))"##,
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

#[test]
fn of_org_entry_get_special() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Heading :tag:\nSCHEDULED: <2026-01-15>\nDEADLINE: <2026-01-20>\nCLOSED: [2026-01-10]\n:PROPERTIES:\n:CUSTOM_ID: myid\n:EFFORT: 2h\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "TODO")
        (org-entry-get nil "PRIORITY")
        (org-entry-get nil "TAGS")
        (org-entry-get nil "SCHEDULED")
        (org-entry-get nil "DEADLINE")
        (org-entry-get nil "CLOSED")
        (org-entry-get nil "CUSTOM_ID")
        (org-entry-get nil "EFFORT")
        (org-entry-get nil "ITEM")))"##,
        expect_test::expect![[
            r#""OK (\"TODO\" \"A\" \":tag:\" \"<2026-01-15>\" nil nil nil nil \"Heading\")""#
        ]],
    );
}

#[test]
fn of_org_entry_properties_standard() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:")
  (goto-char (point-min))
  (org-entry-properties nil 'standard))"##,
        expect_test::expect![[
            r#""OK ((\"CATEGORY\" . \"???\") (\"B\" . \"2\") (\"A\" . \"1\"))""#
        ]],
    );
}

#[test]
fn of_org_entry_put_get_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""OK (\"1\" \"2\" nil \"2\")""#]],
    );
}

#[test]
fn of_org_get_outline_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P\n** T1\n*** S1\n**** SS1")
  (goto-char (point-min))
  (search-forward "SS1")
  (list (org-get-outline-path)
        (org-current-level)
        (org-get-heading t t t t)))"##,
        expect_test::expect![[r#""OK ((\"P\" \"T1\" \"S1\") 4 \"SS1\")""#]],
    );
}

#[test]
fn of_org_map_entries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C\n* WAITING D")
  (org-map-entries
    (lambda () (list (org-get-heading t t t t) (org-get-todo-state)))
    nil 'file))"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn of_org_map_entries_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C\n* WAITING D")
  (list (org-map-entries (lambda () (org-get-heading t t t t)) "TODO" 'file)
        (org-map-entries (lambda () (org-get-heading t t t t)) "DONE" 'file)))"##,
        expect_test::expect![[r#""OK (nil nil)""#]],
    );
}

#[test]
fn of_org_clock_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* A\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\n:END:\n* B\n:LOGBOOK:\nCLOCK: [2026-01-11 14:00]--[2026-01-11 15:30] =>  1:30\n:END:")
  (org-clock-sum))"##,
        expect_test::expect![[r#""OK 150""#]],
    );
}

#[test]
fn of_org_clock_sum_current_entry() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\n:PROPERTIES:\n:EFFORT: 2:00\n:END:\n:LOGBOOK:\nCLOCK: [2026-01-15 10:00]--[2026-01-15 11:30] =>  1:30\n:END:")
  (goto-char (point-min))
  (org-clock-sum-current-entry))"##,
        expect_test::expect![[r#""ERR (void-function org-clock-sum-current-entry)""#]],
    );
}

#[test]
fn of_org_refile_get_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* P1\n** T1\n*** S1\n* P2\n** T2")
  (mapcar 'car (org-refile-get-targets nil)))"##,
        expect_test::expect![[r#""OK (\"P1\" \"P2\")""#]],
    );
}

#[test]
fn of_org_id_get_create() {
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

#[test]
fn of_org_store_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-min))
  (org-store-link nil)
  (list (car org-stored-links)))"##,
        expect_test::expect![[r#""OK (nil)""#]],
    );
}

#[test]
fn of_org_insert_link() {
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

#[test]
fn of_org_sort_entries() {
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

#[test]
fn of_org_move_subtree() {
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

#[test]
fn of_org_clone_subtree() {
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

#[test]
fn of_org_copy_paste_subtree() {
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

#[test]
fn of_org_mark_subtree() {
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

#[test]
fn of_org_narrow_to_subtree() {
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

#[test]
fn of_org_end_of_subtree() {
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

#[test]
fn of_org_cycle_hide_drawers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""ERR (search-failed \"A\")""#]],
    );
}

#[test]
fn of_org_toggle_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""OK (\"* Plain text\" \"Existing heading\")""#]],
    );
}

#[test]
fn of_org_move_item() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""OK ((\"A\" \"B\" \"C\") (\"B\" \"A\" \"C\"))""#]],
    );
}

#[test]
fn of_org_insert_todo_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[
            r#""OK ((\"Existing\" nil) (\"New task\" \"TODO\") (\"Right task\" \"TODO\"))""#
        ]],
    );
}

#[test]
fn of_org_toggle_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H :a:")
  (goto-char (point-min))
  (let ((t1 (org-get-tags nil t)))
    (org-toggle-tag "b" 'on)
    (let ((t2 (org-get-tags nil t)))
      (org-toggle-tag "a" 'off)
      (list t1 t2 (org-get-tags nil t)))))"##,
        expect_test::expect![[r#""OK ((\"a\") (\"a\" \"b\") (\"b\"))""#]],
    );
}

#[test]
fn of_org_priority_cycle() {
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

#[test]
fn of_org_todo_cycle() {
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
            r#""OK (\"TODO\" #(\"DONE\" 0 4 (org-todo-head \"TODO\")) nil #(\"TODO\" 0 4 (org-todo-head \"TODO\")))""#
        ]],
    );
}

#[test]
fn of_org_visibility_cycle() {
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

#[test]
fn of_org_update_statistics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T [%]\n- [X] a\n- [ ] b\n- [X] c")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
        expect_test::expect![[r#""OK \"* T [66%]\"""#]],
    );
}

#[test]
fn of_org_match_sparse_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
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
        expect_test::expect![[r#""OK ((\"A\" \"B\" \"C\") nil)""#]],
    );
}

#[test]
fn of_org_dblock_update() {
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

#[test]
fn of_org_macro_replace() {
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

#[test]
fn of_org_try_structure() {
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

#[test]
fn of_org_pcomplete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "\\agr")
  (length (all-completions "\\ag" (pcomplete-entries))))"##,
        expect_test::expect![[r#""OK 0""#]],
    );
}
