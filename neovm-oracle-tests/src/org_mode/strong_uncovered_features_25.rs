//! Strong uncovered-features-25 oracle tests — org-agenda and org-capture.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{
    assert_oracle_parity, assert_oracle_parity_with_shared_tempdir,
    return_if_neovm_enable_oracle_proptest_not_set,
};

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_agenda_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK #(\"Week-agenda (W27):\nMonday     29 June 2026 W27\n  test:       Sched.165x:  TODO T\nTuesday    30 June 2026\nWednesday   1 July 2026\nThursday    2 July 2026\nFriday      3 July 2026\nSaturday    4 July 2026\nSunday      5 July 2026\n\" 0 18 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda org-agenda-structural-header t org-date-line t face org-agenda-structure) 18 19 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 19 46 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739796 org-today t org-day-cnt 1 org-agenda-date-header t org-date-line t face org-agenda-date-today) 46 47 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda org-day-cnt 1 day 739796) 47 74 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda org-day-cnt 1 day 739796 org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"TODO T\" 0 6 (org-heading t effort-minutes nil effort nil)) level \" \" time \"\" extra \"Sched.165x: \" format (((org-prefix-has-time t) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s%s%s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))) (if (member time '(\"\" nil)) \"\" (format \"%-12s\" (concat time \"\"))) (format \"%s\" (if (member extra '(\"\" nil)) \"\" (concat extra \" \" (get-text-property 0 'extra-space extra)))))) dotime nil org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" done-face org-agenda-done mouse-face highlight help-echo \"mouse-2 or RET jump to Org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32791/test.org\" undone-face org-scheduled-previously face org-scheduled-previously org-marker #<marker (moves after insertion) at 21 in test.org> org-hd-marker #<marker (moves after insertion) at 1 in test.org> type \"past-scheduled\" date 739631 ts-date 739631 warntime nil effort nil effort-minutes nil urgency 1264 priority 1000 org-habit-p nil todo-state \"TODO\") 74 78 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda org-day-cnt 1 day 739796 todo-state \"TODO\" org-habit-p nil priority 1000 urgency 1264 warntime nil ts-date 739631 date 739631 type \"past-scheduled\" org-hd-marker #<marker (moves after insertion) at 1 in test.org> org-marker #<marker (moves after insertion) at 21 in test.org> face org-todo undone-face org-scheduled-previously help-echo \"mouse-2 or RET jump to Org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32791/test.org\" mouse-face highlight done-face org-agenda-done org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-not-done-regexp \"\\\\(TODO\\\\)\" dotime nil format (((org-prefix-has-time t) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s%s%s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))) (if (member time '(\"\" nil)) \"\" (format \"%-12s\" (concat time \"\"))) (format \"%s\" (if (member extra '(\"\" nil)) \"\" (concat extra \" \" (get-text-property 0 'extra-space extra)))))) extra \"Sched.165x: \" time \"\" level \" \" txt #(\"TODO T\" 0 6 (org-heading t effort-minutes nil effort nil)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags nil org-category \"test\" effort nil effort-minutes nil org-heading t) 78 79 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda org-day-cnt 1 day 739796 todo-state \"TODO\" org-habit-p nil priority 1000 urgency 1264 effort-minutes nil effort nil warntime nil ts-date 739631 date 739631 type \"past-scheduled\" org-hd-marker #<marker (moves after insertion) at 1 in test.org> org-marker #<marker (moves after insertion) at 21 in test.org> face org-scheduled-previously undone-face org-scheduled-previously help-echo \"mouse-2 or RET jump to Org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32791/test.org\" mouse-face highlight done-face org-agenda-done org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-not-done-regexp \"\\\\(TODO\\\\)\" dotime nil format (((org-prefix-has-time t) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s%s%s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))) (if (member time '(\"\" nil)) \"\" (format \"%-12s\" (concat time \"\"))) (format \"%s\" (if (member extra '(\"\" nil)) \"\" (concat extra \" \" (get-text-property 0 'extra-space extra)))))) extra \"Sched.165x: \" time \"\" level \" \" txt #(\"TODO T\" 0 6 (org-heading t effort-minutes nil effort nil)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags nil org-category \"test\") 79 80 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda org-day-cnt 1 day 739796 org-heading t effort-minutes nil effort nil org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"TODO T\" 0 6 (org-heading t effort-minutes nil effort nil)) level \" \" time \"\" extra \"Sched.165x: \" format (((org-prefix-has-time t) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s%s%s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))) (if (member time '(\"\" nil)) \"\" (format \"%-12s\" (concat time \"\"))) (format \"%s\" (if (member extra '(\"\" nil)) \"\" (concat extra \" \" (get-text-property 0 'extra-space extra)))))) dotime nil org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" done-face org-agenda-done mouse-face highlight help-echo \"mouse-2 or RET jump to Org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32791/test.org\" undone-face org-scheduled-previously face org-scheduled-previously org-marker #<marker (moves after insertion) at 21 in test.org> org-hd-marker #<marker (moves after insertion) at 1 in test.org> type \"past-scheduled\" date 739631 ts-date 739631 warntime nil urgency 1264 priority 1000 org-habit-p nil todo-state \"TODO\") 80 81 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 81 104 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739797 org-day-cnt 2 org-agenda-date-header t org-date-line t face org-agenda-date) 104 105 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 105 128 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739798 org-day-cnt 3 org-agenda-date-header t org-date-line t face org-agenda-date) 128 129 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 129 152 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739799 org-day-cnt 4 org-agenda-date-header t org-date-line t face org-agenda-date) 152 153 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 153 176 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739800 org-day-cnt 5 org-agenda-date-header t org-date-line t face org-agenda-date) 176 177 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 177 200 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739801 org-day-cnt 6 org-agenda-date-header t org-date-line t face org-agenda-date-weekend) 200 201 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 201 224 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739802 org-day-cnt 7 org-agenda-date-header t org-date-line t face org-agenda-date-weekend) 224 225 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda))""#
    ]];
    crate::common::assert_oracle_parity_with_shared_tempdir_expect(
        r##"(let* ((file (expand-file-name "test.org" (getenv "NEOVM_ORACLE_TEST_TMPDIR")))
       (org-agenda-files (list file)))
  (with-temp-file file
    (insert "* TODO T\nSCHEDULED: <2026-01-15>\n* DONE D"))
  (condition-case nil
      (org-agenda-list)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-todo-list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_todo_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK #(\"Global list of TODO items of type: ALL\nPress ‘N r’ (e.g. ‘0 r’) to search again: (0)[ALL] (1)DONE (2)TODO\n  test:       TODO T1\n  test:       TODO T2\n\" 0 34 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-agenda-structural-header t short-heading \"ToDo: ALL\" face org-agenda-structure) 34 35 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-agenda-structural-header t) 35 38 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-agenda-structural-header t face org-agenda-structure-filter) 38 39 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo) 39 48 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 48 49 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo font-lock-face help-key-binding face org-agenda-structure-secondary) 49 50 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 50 57 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 57 58 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 58 60 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 60 61 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo font-lock-face help-key-binding face org-agenda-structure-secondary) 61 62 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 62 89 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 89 90 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 90 97 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 97 98 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 98 105 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 105 106 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo) 106 120 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"TODO T1\" 0 7 (org-heading t effort-minutes nil effort nil)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime t face nil done-face org-agenda-done org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" mouse-face highlight help-echo \"mouse-2 or RET jump to org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32792/test.org\" org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32792>> org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32792>> priority 1000 urgency 1001 effort nil effort-minutes nil ts-date nil type \"todo\" todo-state \"TODO\") 120 124 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo todo-state \"TODO\" type \"todo\" ts-date nil urgency 1001 priority 1000 org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32792>> org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32792>> help-echo \"mouse-2 or RET jump to org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32792/test.org\" mouse-face highlight org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-not-done-regexp \"\\\\(TODO\\\\)\" done-face org-agenda-done face org-todo dotime t format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) extra \"\" time \"\" level \" \" txt #(\"TODO T1\" 0 7 (org-heading t effort-minutes nil effort nil)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags nil org-category \"test\" effort nil effort-minutes nil org-heading t) 124 125 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo todo-state \"TODO\" type \"todo\" ts-date nil effort-minutes nil effort nil urgency 1001 priority 1000 org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32792>> org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32792>> help-echo \"mouse-2 or RET jump to org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32792/test.org\" mouse-face highlight org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-not-done-regexp \"\\\\(TODO\\\\)\" done-face org-agenda-done face nil dotime t format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) extra \"\" time \"\" level \" \" txt #(\"TODO T1\" 0 7 (org-heading t effort-minutes nil effort nil)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags nil org-category \"test\") 125 127 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-heading t effort-minutes nil effort nil org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"TODO T1\" 0 7 (org-heading t effort-minutes nil effort nil)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime t face nil done-face org-agenda-done org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" mouse-face highlight help-echo \"mouse-2 or RET jump to org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32792/test.org\" org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32792>> org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32792>> priority 1000 urgency 1001 ts-date nil type \"todo\" todo-state \"TODO\") 127 128 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo) 128 142 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"TODO T2\" 0 7 (org-heading t effort-minutes nil effort nil)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime t face nil done-face org-agenda-done org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" mouse-face highlight help-echo \"mouse-2 or RET jump to org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32792/test.org\" org-marker #<marker (moves after insertion) at 21 in test.org<tmp-32792>> org-hd-marker #<marker (moves after insertion) at 21 in test.org<tmp-32792>> priority 1000 urgency 1001 effort nil effort-minutes nil ts-date nil type \"todo\" todo-state \"TODO\") 142 146 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo todo-state \"TODO\" type \"todo\" ts-date nil urgency 1001 priority 1000 org-hd-marker #<marker (moves after insertion) at 21 in test.org<tmp-32792>> org-marker #<marker (moves after insertion) at 21 in test.org<tmp-32792>> help-echo \"mouse-2 or RET jump to org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32792/test.org\" mouse-face highlight org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-not-done-regexp \"\\\\(TODO\\\\)\" done-face org-agenda-done face org-todo dotime t format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) extra \"\" time \"\" level \" \" txt #(\"TODO T2\" 0 7 (org-heading t effort-minutes nil effort nil)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags nil org-category \"test\" effort nil effort-minutes nil org-heading t) 146 147 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo todo-state \"TODO\" type \"todo\" ts-date nil effort-minutes nil effort nil urgency 1001 priority 1000 org-hd-marker #<marker (moves after insertion) at 21 in test.org<tmp-32792>> org-marker #<marker (moves after insertion) at 21 in test.org<tmp-32792>> help-echo \"mouse-2 or RET jump to org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32792/test.org\" mouse-face highlight org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-not-done-regexp \"\\\\(TODO\\\\)\" done-face org-agenda-done face nil dotime t format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) extra \"\" time \"\" level \" \" txt #(\"TODO T2\" 0 7 (org-heading t effort-minutes nil effort nil)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags nil org-category \"test\") 147 149 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-heading t effort-minutes nil effort nil org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"TODO T2\" 0 7 (org-heading t effort-minutes nil effort nil)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime t face nil done-face org-agenda-done org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" mouse-face highlight help-echo \"mouse-2 or RET jump to org file /tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32792/test.org\" org-marker #<marker (moves after insertion) at 21 in test.org<tmp-32792>> org-hd-marker #<marker (moves after insertion) at 21 in test.org<tmp-32792>> priority 1000 urgency 1001 ts-date nil type \"todo\" todo-state \"TODO\") 149 150 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo))""#
    ]];
    crate::common::assert_oracle_parity_with_shared_tempdir_expect(
        r##"(let* ((file (expand-file-name "test.org" (getenv "NEOVM_ORACLE_TEST_TMPDIR")))
       (org-agenda-files (list file)))
  (with-temp-file file
    (insert "* TODO T1\n* DONE D1\n* TODO T2"))
  (condition-case nil
      (org-todo-list)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-tags-view
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_tags_view() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK #(\"Headlines with TAGS match: work\nPress ‘C-u r’ to search again\n  test:       T1                                                         :work:\n  test:       T3                                                         :work:\n\" 0 26 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-agenda-structural-header t short-heading \"Match: work\" face org-agenda-structure) 26 27 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-agenda-structural-header t) 27 31 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-agenda-structural-header t face org-agenda-structure-filter) 31 32 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags) 32 39 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags face org-agenda-structure-secondary) 39 42 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags font-lock-face help-key-binding face org-agenda-structure-secondary) 42 43 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags face org-agenda-structure-secondary) 43 44 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags font-lock-face help-key-binding face org-agenda-structure-secondary) 44 45 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags face org-agenda-structure-secondary) 45 61 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags face org-agenda-structure-secondary) 61 62 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags) 62 76 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-category \"test\" tags (\"work\") org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T1                                         :work:\" 0 2 (org-heading t effort-minutes nil effort nil) 2 49 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime nil effort nil effort-minutes nil face default done-face org-agenda-done undone-face default mouse-face highlight org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" help-echo \"mouse-2 or RET jump to Org file \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32793/test.org\\\"\" org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32793>> org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32793>> todo-state nil ts-date nil priority 1000 urgency 1000 type \"tagsmatch\") 76 78 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-heading t effort-minutes nil effort nil org-category \"test\" tags (\"work\") org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T1                                         :work:\" 0 2 (org-heading t effort-minutes nil effort nil) 2 49 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime nil face default done-face org-agenda-done undone-face default mouse-face highlight org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" help-echo \"mouse-2 or RET jump to Org file \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32793/test.org\\\"\" org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32793>> org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32793>> todo-state nil ts-date nil priority 1000 urgency 1000 type \"tagsmatch\") 78 135 (type \"tagsmatch\" urgency 1000 priority 1000 ts-date nil todo-state nil org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32793>> org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32793>> help-echo \"mouse-2 or RET jump to Org file \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32793/test.org\\\"\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-not-done-regexp \"\\\\(TODO\\\\)\" mouse-face highlight undone-face default done-face org-agenda-done face nil effort-minutes nil effort nil dotime nil format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) extra \"\" time \"\" level \" \" txt #(\"T1                                         :work:\" 0 2 (org-heading t effort-minutes nil effort nil) 2 49 (org-heading t)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags (\"work\") org-category \"test\" org-heading t org-agenda-type tags org-last-args (nil \"work\") org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-series-cmd nil) 135 141 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-heading t org-category \"test\" tags (\"work\") org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T1                                         :work:\" 0 2 (org-heading t effort-minutes nil effort nil) 2 49 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime nil effort nil effort-minutes nil face (org-tag default) done-face org-agenda-done undone-face default mouse-face highlight org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" help-echo \"mouse-2 or RET jump to Org file \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32793/test.org\\\"\" org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32793>> org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32793>> todo-state nil ts-date nil priority 1000 urgency 1000 type \"tagsmatch\") 141 142 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags) 142 156 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-category \"test\" tags (\"work\") org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T3                                         :work:\" 0 2 (org-heading t effort-minutes nil effort nil) 2 49 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime nil effort nil effort-minutes nil face default done-face org-agenda-done undone-face default mouse-face highlight org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" help-echo \"mouse-2 or RET jump to Org file \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32793/test.org\\\"\" org-marker #<marker (moves after insertion) at 25 in test.org<tmp-32793>> org-hd-marker #<marker (moves after insertion) at 25 in test.org<tmp-32793>> todo-state nil ts-date nil priority 1000 urgency 1000 type \"tagsmatch\") 156 158 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-heading t effort-minutes nil effort nil org-category \"test\" tags (\"work\") org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T3                                         :work:\" 0 2 (org-heading t effort-minutes nil effort nil) 2 49 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime nil face default done-face org-agenda-done undone-face default mouse-face highlight org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" help-echo \"mouse-2 or RET jump to Org file \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32793/test.org\\\"\" org-marker #<marker (moves after insertion) at 25 in test.org<tmp-32793>> org-hd-marker #<marker (moves after insertion) at 25 in test.org<tmp-32793>> todo-state nil ts-date nil priority 1000 urgency 1000 type \"tagsmatch\") 158 215 (type \"tagsmatch\" urgency 1000 priority 1000 ts-date nil todo-state nil org-hd-marker #<marker (moves after insertion) at 25 in test.org<tmp-32793>> org-marker #<marker (moves after insertion) at 25 in test.org<tmp-32793>> help-echo \"mouse-2 or RET jump to Org file \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32793/test.org\\\"\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-not-done-regexp \"\\\\(TODO\\\\)\" mouse-face highlight undone-face default done-face org-agenda-done face nil effort-minutes nil effort nil dotime nil format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) extra \"\" time \"\" level \" \" txt #(\"T3                                         :work:\" 0 2 (org-heading t effort-minutes nil effort nil) 2 49 (org-heading t)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags (\"work\") org-category \"test\" org-heading t org-agenda-type tags org-last-args (nil \"work\") org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-series-cmd nil) 215 221 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags org-heading t org-category \"test\" tags (\"work\") org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T3                                         :work:\" 0 2 (org-heading t effort-minutes nil effort nil) 2 49 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime nil effort nil effort-minutes nil face (org-tag default) done-face org-agenda-done undone-face default mouse-face highlight org-not-done-regexp \"\\\\(TODO\\\\)\" org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" help-echo \"mouse-2 or RET jump to Org file \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/tmp-32793/test.org\\\"\" org-marker #<marker (moves after insertion) at 25 in test.org<tmp-32793>> org-hd-marker #<marker (moves after insertion) at 25 in test.org<tmp-32793>> todo-state nil ts-date nil priority 1000 urgency 1000 type \"tagsmatch\") 221 222 (org-series-cmd nil org-redo-cmd (org-tags-view 'nil (if current-prefix-arg nil \"work\")) org-last-args (nil \"work\") org-agenda-type tags))""#
    ]];
    crate::common::assert_oracle_parity_with_shared_tempdir_expect(
        r##"(let* ((file (expand-file-name "test.org" (getenv "NEOVM_ORACLE_TEST_TMPDIR")))
       (org-agenda-files (list file)))
  (with-temp-file file
    (insert "* T1 :work:\n* T2 :home:\n* T3 :work:"))
  (condition-case nil
      (org-tags-view nil "work")
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-search-view
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_search_view() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK #(\"Search words: keyword\nPress ‘[’, ‘]’ to add/sub word, ‘{’, ‘}’ to add/sub regexp, ‘C-u r’ for a fresh search\n  test:       T1 keyword\n  test:       T3 keyword\n\" 0 13 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search org-agenda-structural-header t face org-agenda-structure) 13 14 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search org-agenda-structural-header t) 14 21 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search org-agenda-structural-header t face org-agenda-structure-filter) 21 22 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search) 22 29 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 29 30 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search font-lock-face help-key-binding face org-agenda-structure-secondary) 30 31 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 31 33 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 33 34 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 34 35 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search font-lock-face help-key-binding face org-agenda-structure-secondary) 35 36 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 36 54 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 54 55 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 55 56 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search font-lock-face help-key-binding face org-agenda-structure-secondary) 56 57 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 57 59 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 59 60 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 60 61 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search font-lock-face help-key-binding face org-agenda-structure-secondary) 61 62 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 62 82 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 82 83 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 83 86 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search font-lock-face help-key-binding face org-agenda-structure-secondary) 86 87 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 87 88 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search font-lock-face help-key-binding face org-agenda-structure-secondary) 88 89 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 89 108 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search face org-agenda-structure-secondary) 108 109 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search) 109 123 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T1 keyword\" 0 10 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime t face nil done-face org-agenda-done org-not-done-regexp nil org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" mouse-face highlight help-echo \"mouse-2 or RET jump to location\" org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32794>> org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32794>> urgency 1000 priority 1000 type \"search\") 123 133 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search org-heading t org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T1 keyword\" 0 10 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime t face nil done-face org-agenda-done org-not-done-regexp nil org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" mouse-face highlight help-echo \"mouse-2 or RET jump to location\" org-marker #<marker (moves after insertion) at 1 in test.org<tmp-32794>> org-hd-marker #<marker (moves after insertion) at 1 in test.org<tmp-32794>> urgency 1000 priority 1000 type \"search\") 133 134 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search) 134 148 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T3 keyword\" 0 10 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime t face nil done-face org-agenda-done org-not-done-regexp nil org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" mouse-face highlight help-echo \"mouse-2 or RET jump to location\" org-marker #<marker (moves after insertion) at 25 in test.org<tmp-32794>> org-hd-marker #<marker (moves after insertion) at 25 in test.org<tmp-32794>> urgency 1000 priority 1000 type \"search\") 148 158 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search org-heading t org-category \"test\" tags nil org-priority-highest 65 org-priority-lowest 67 time-of-day nil duration nil breadcrumbs nil txt #(\"T3 keyword\" 0 10 (org-heading t)) level \" \" time \"\" extra \"\" format (((org-prefix-has-time nil) (org-prefix-has-tag nil) (org-prefix-category-length 12) (org-prefix-has-effort nil) (org-prefix-has-breadcrumbs nil)) (format \" %s %s\" (format \"%s\" (if (member category-icon '(\"\" nil)) \"\" (concat category-icon \"\" (get-text-property 0 'extra-space category-icon)))) (format \"%-12s\" (if (member category '(\"\" nil)) \"\" (concat category \":\" (get-text-property 0 'extra-space category)))))) dotime t face nil done-face org-agenda-done org-not-done-regexp nil org-todo-regexp \"\\\\(DONE\\\\|TODO\\\\)\" org-complex-heading-regexp \"^\\\\(\\\\*+\\\\)\\\\(?: +\\\\(DONE\\\\|TODO\\\\)\\\\)?\\\\(?: +\\\\(\\\\[#\\\\(?:[A-Z]\\\\|[0-9]\\\\|[1-5][0-9]\\\\|6[0-4]\\\\)\\\\]\\\\)\\\\)?\\\\(?: +\\\\(.*?\\\\)\\\\)??\\\\(?:[ \t]+\\\\(:\\\\([[:alnum:]_@#%:]+\\\\):\\\\)\\\\)?[ \t]*$\" mouse-face highlight help-echo \"mouse-2 or RET jump to location\" org-marker #<marker (moves after insertion) at 25 in test.org<tmp-32794>> org-hd-marker #<marker (moves after insertion) at 25 in test.org<tmp-32794>> urgency 1000 priority 1000 type \"search\") 158 159 (org-series-cmd nil org-redo-cmd (org-search-view nil (if current-prefix-arg nil \"keyword\")) org-last-args (nil \"keyword\" nil) org-agenda-type search))""#
    ]];
    crate::common::assert_oracle_parity_with_shared_tempdir_expect(
        r##"(let* ((file (expand-file-name "test.org" (getenv "NEOVM_ORACLE_TEST_TMPDIR")))
       (org-agenda-files (list file)))
  (with-temp-file file
    (insert "* T1 keyword\n* T2 other\n* T3 keyword"))
  (condition-case nil
      (org-search-view nil "keyword")
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_agenda_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK ((:all 0) (:todo 0) (:done 0))""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\n* DONE D1\n* TODO T2\n* DONE D2")
  (org-agenda-prepare-buffers (list (current-buffer)))
  (let ((r '()))
    (push (list :all (length (org-map-entries (lambda () t) nil 'file))) r)
    (push (list :todo (length (org-map-entries (lambda () t) "TODO" 'file))) r)
    (push (list :done (length (org-map-entries (lambda () t) "DONE" 'file))) r)
    (nreverse r)))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-get-todos
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_agenda_todos() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (\"T1\" \"D1\" \"T2\")""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\n* DONE D1\n* WAITING W1\n* TODO T2")
  (mapcar (lambda (x) (org-element-property :raw-value x))
          (org-element-map (org-element-parse-buffer) 'headline
            (lambda (h) (when (org-element-property :todo-keyword h) h)))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-get-deadlines
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_agenda_deadlines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1\nDEADLINE: <2026-01-15>\n* T2\nDEADLINE: <2026-01-20>\n* T3")
  (org-element-map (org-element-parse-buffer) 'planning
    (lambda (p) (when (org-element-property :deadline p)
                  (org-element-property :raw-value
                    (org-element-property :parent p))))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-get-scheduled
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_agenda_scheduled() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1\nSCHEDULED: <2026-01-15>\n* T2\nSCHEDULED: <2026-01-20>\n* T3")
  (org-element-map (org-element-parse-buffer) 'planning
    (lambda (p) (when (org-element-property :scheduled p)
                  (org-element-property :raw-value
                    (org-element-property :parent p))))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-get-timestamps
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_agenda_timestamps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK ((active 2026 15) (inactive 2026 20) (active-range 2026 25))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1\n<2026-01-15>\n* T2\n[2026-01-20]\n* T3\n<2026-01-25>--<2026-01-30>")
  (org-element-map (org-element-parse-buffer) 'timestamp
    (lambda (ts) (list (org-element-property :type ts)
                      (org-element-property :year-start ts)
                      (org-element-property :day-start ts)))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-get-blocks
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_agenda_blocks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK ((2026 15 2026 20))""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1\n<2026-01-15>--<2026-01-20>\n* T2\n[2026-01-25]--[2026-01-30]")
  (org-element-map (org-element-parse-buffer) 'timestamp
    (lambda (ts) (when (eq (org-element-property :type ts) 'active-range)
                   (list (org-element-property :year-start ts)
                         (org-element-property :day-start ts)
                         (org-element-property :year-end ts)
                         (org-element-property :day-end ts))))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-get-sexps
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_agenda_sexps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (\"%%(diary-anniversary 1 1 2000)\")""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "%%(diary-anniversary 1 1 2000)")
  (org-element-map (org-element-parse-buffer) 'diary-sexp
    (lambda (d) (org-element-property :value d))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-to-appt
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_appt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"No event to add\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1\nSCHEDULED: <2026-01-15 10:00>\n* T2\nDEADLINE: <2026-01-16 14:00>")
  (condition-case nil
      (org-agenda-to-appt t)
    (error nil)))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-set-restriction-lock
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_restriction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect =
        expect_test::expect![[r#""ERR (void-variable org-agenda-restrict-lock-current)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\n* H4")
  (goto-char (point-min))
  (search-forward "H2")
  (beginning-of-line)
  (condition-case nil
      (org-agenda-set-restriction-lock 'subtree)
    (error nil))
  (list org-agenda-restrict-lock-current))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-remove-restriction-lock
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_restriction_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect =
        expect_test::expect![[r#""ERR (void-variable org-agenda-restrict-lock-current)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (condition-case nil
      (org-agenda-set-restriction-lock 'subtree)
    (error nil))
  (org-agenda-remove-restriction-lock)
  (list org-agenda-restrict-lock-current))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-prepare-buffers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_prepare() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"* H\nBody\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (org-agenda-prepare-buffers (list (current-buffer)))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-format-item
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK #(\"T1\" 0 2 (dotime nil format nil extra \"\" time \"\" level todo txt #(\"T1\" 0 2 (org-heading t)) breadcrumbs nil duration nil time-of-day nil org-priority-lowest 67 org-priority-highest 65 tags nil org-category \"\" org-heading t))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1 :work:")
  (goto-char (point-min))
  (condition-case nil
      (org-agenda-format-item nil "T1" 'todo nil nil nil)
    (error nil)))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-finalize
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_finalize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"* TODO T1\n* DONE D1\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\n* DONE D1")
  (condition-case nil
      (org-agenda-finalize)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-mark-filtered-text
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_filter_mark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"* TODO T1\n* DONE D1\n* TODO T2\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\n* DONE D1\n* TODO T2")
  (condition-case nil
      (org-agenda-mark-filtered-text)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter-apply
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_filter_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"* TODO T1\n* DONE D1\n* TODO T2\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\n* DONE D1\n* TODO T2")
  (condition-case nil
      (org-agenda-filter-apply "+work")
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-redo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_redo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect =
        expect_test::expect![[r#""OK #(\"* TODO T1\n* DONE D1\" 0 19 (org-lprops nil))""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\n* DONE D1")
  (condition-case nil
      (org-agenda-redo)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-quit
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_quit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK \"Warning (files): Missing ‘lexical-binding’ cookie in \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/program-32800-100.el\\\".\nYou can add one with ‘M-x elisp-enable-lexical-binding RET’.\nSee ‘(elisp)Selecting Lisp Dialect’ and ‘(elisp)Converting to Lexical Binding’\nfor more information.\nNo event to add\nNo agenda restriction to remove.\nLocking agenda restriction to subtree\nAgenda restriction lock removed\nLocking agenda restriction to subtree\nAgenda restriction lock removed\nRebuilding agenda buffer...done\n\"""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (condition-case nil
      (org-agenda-quit)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-exit
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf25_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK \"Warning (files): Missing ‘lexical-binding’ cookie in \\\"/tmp/nix-shell.XcUf3d/neovm-inline-oracle-sf3ylzib/program-32800-100.el\\\".\nYou can add one with ‘M-x elisp-enable-lexical-binding RET’.\nSee ‘(elisp)Selecting Lisp Dialect’ and ‘(elisp)Converting to Lexical Binding’\nfor more information.\nNo event to add\nNo agenda restriction to remove.\nLocking agenda restriction to subtree\nAgenda restriction lock removed\nLocking agenda restriction to subtree\nAgenda restriction lock removed\nRebuilding agenda buffer...done\n\"""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (condition-case nil
      (org-agenda-exit)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}
