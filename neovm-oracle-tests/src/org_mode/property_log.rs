use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_tags_multivalue_property_delete_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task :old:\n")
    (insert ":PROPERTIES:\n:A: 1\n:B: two\n:END:\n")
    (goto-char (point-min))
    (org-toggle-tag "new" 'on)
    (org-toggle-tag "old" 'off)
    (org-entry-put nil "A" "updated")
    (org-entry-put-multivalued-property nil "Multi" "x" "y" "z")
    (org-entry-delete nil "B")
    (list (org-get-tags)
          (org-entry-properties nil 'standard)
          (buffer-substring-no-properties (point-min) (point-max)))))"#,
    );
}

#[test]
fn org_archive_tag_toggle_parse_roundtrip_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-archive)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Active\n** DONE Child\nBody\n** TODO Keep\n")
    (goto-char (point-min))
    (search-forward "Child")
    (beginning-of-line)
    (org-toggle-archive-tag)
    (let ((after-archive
           (buffer-substring-no-properties (point-min) (point-max))))
      (org-toggle-archive-tag)
      (list after-archive
            (buffer-substring-no-properties (point-min) (point-max))
            (org-element-map (org-element-parse-buffer) 'headline
              (lambda (headline)
                (list (org-element-property :raw-value headline)
                      (org-element-property :tags headline))))))))"#,
    );
}

#[test]
fn org_done_log_drawer_timestamp_normalized_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task\n")
    (goto-char (point-min))
    (let ((org-log-into-drawer t)
          (org-log-note-clock-out nil)
          (org-log-done 'time))
      (org-todo "DONE")
      (list (org-log-beginning t)
            (replace-regexp-in-string
             "CLOSED: \\[.*\\]"
             "CLOSED: [stamp]"
             (buffer-substring-no-properties (point-min) (point-max)))))))"#,
    );
}

#[test]
fn org_property_inheritance_allowed_cycle_delete_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-use-property-inheritance '("Owner" "Milestone"))
          (changes nil))
      (add-hook 'org-property-changed-functions
                (lambda (key value) (push (list key value) changes))
                nil t)
      (org-mode)
      (insert "#+PROPERTY: Status_ALL Todo Doing Done :ETC\n")
      (insert "* Project\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:Milestone: M1\n:END:\n")
      (insert "** Task\n")
      (insert ":PROPERTIES:\n:Status: Todo\n:Owner: Bea\n:Other: keep\n:END:\n")
      (goto-char (point-min))
      (search-forward "Task")
      (beginning-of-line)
      (let ((inherited (list (org-entry-get nil "Owner" 'inherit)
                             (org-entry-get nil "Milestone" 'inherit)
                             (org-entry-get-with-inheritance "Milestone")))
            (allowed (org-property-get-allowed-values nil "Status" 'table)))
        (search-forward ":Status:")
        (org-property-next-allowed-value)
        (org-property-next-allowed-value)
        (org-property-previous-allowed-value)
        (goto-char (point-min))
        (search-forward "Task")
        (beginning-of-line)
        (org-entry-add-to-multivalued-property nil "Multi" "x")
        (org-entry-add-to-multivalued-property nil "Multi" "y")
        (org-entry-remove-from-multivalued-property nil "Multi" "x")
        (org-entry-delete nil "Other")
        (list inherited
              allowed
              (org-entry-get nil "Status")
              (org-entry-get-multivalued-property nil "Multi")
              (org-entry-member-in-multivalued-property nil "Multi" "y")
              (nreverse changes)
              (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_property_values_global_delete_roundtrip_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+PROPERTY: Owner_ALL Ada Bea Cy\n")
    (insert "* A\n:PROPERTIES:\n:Owner: Ada\n:Effort: 0:30\n:END:\n")
    (insert "** A1\n:PROPERTIES:\n:Owner: Bea\n:Effort: 0:15\n:END:\n")
    (insert "* B\n:PROPERTIES:\n:Owner: Ada\n:Effort: 1:00\n:END:\n")
    (goto-char (point-min))
    (let ((owners-before (sort (copy-sequence (org-property-values "Owner"))
                               #'string<))
          (efforts-before (sort (copy-sequence (org-property-values "Effort"))
                                #'string<)))
      (org-delete-property-globally "Effort")
      (goto-char (point-min))
      (search-forward "A1")
      (beginning-of-line)
      (org-entry-put nil "Owner" "Cy")
      (list owners-before
            efforts-before
            (sort (copy-sequence (org-property-values "Owner")) #'string<)
            (org-property-values "Effort")
            (org-entry-properties nil 'standard)
            (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_property_set_delete_allowed_values_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+PROPERTY: Phase_ALL Plan Build Ship\n")
    (insert "* Project\n")
    (insert "** Task\n")
    (goto-char (point-min))
    (search-forward "Task")
    (beginning-of-line)
    (let ((org-last-set-property "Phase")
          (org-last-set-property-value "Build"))
      (org-set-property "Phase" "Plan")
      (org-set-property "Owner" "Ada")
      (let ((after-set (buffer-substring-no-properties
                        (point-min) (point-max)))
            (allowed (org-property-get-allowed-values nil "Phase" 'table)))
        (org-delete-property "Owner")
        (search-forward ":Phase:")
        (org-property-next-allowed-value)
        (org-property-next-allowed-value)
        (org-property-next-allowed-value)
        (list after-set
              allowed
              (org-entry-properties nil 'standard)
              org-last-set-property
              org-last-set-property-value
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_todo_done_note_log_drawer_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task\n")
    (goto-char (point-min))
    (let ((org-log-into-drawer "LOGBOOK")
          (org-log-done 'note)
          (org-log-note-how 'time)
          (org-log-note-clock-out nil)
          (org-log-note-headings
           '((done . "State %-12s from %-12S %t")
             (note . "Note taken on %t"))))
      (cl-letf (((symbol-function 'read-string)
                 (lambda (&rest _) "Finished carefully"))
                ((symbol-function 'read-char-exclusive)
                 (lambda (&rest _) ?\C-c)))
        (org-todo "DONE")
        (when (and (boundp 'org-log-note-marker)
                   org-log-note-marker)
          (with-current-buffer (marker-buffer org-log-note-marker)
            (goto-char org-log-note-marker)
            (insert "Finished carefully")
            (org-add-log-note))))
      (list (org-entry-get nil "CLOSED")
            (replace-regexp-in-string
             "\\[[0-9][^]\n]+\\]"
             "[stamp]"
             (buffer-substring-no-properties
              (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_log_repeat_reschedule_redeadline_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Repeater\n")
    (insert "SCHEDULED: <2026-05-27 Wed +1w>\n")
    (insert "DEADLINE: <2026-05-28 Thu +2w>\n")
    (goto-char (point-min))
    (let ((org-log-into-drawer t)
          (org-log-reschedule 'time)
          (org-log-redeadline 'time)
          (org-log-repeat 'time)
          (org-log-done nil))
      (org-schedule nil "2026-06-03")
      (org-deadline nil "2026-06-11")
      (org-todo "DONE")
      (list (org-entry-get nil "LAST_REPEAT")
            (replace-regexp-in-string
             "\\[[0-9][^]\n]+\\]"
             "[stamp]"
             (buffer-substring-no-properties
              (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_property_clock_drawer_fold_element_lifecycle_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (require 'org-element)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-use-property-inheritance '("Client" "Sprint"))
          (org-clock-into-drawer "LOGBOOK")
          (org-log-into-drawer "LOGBOOK")
          (org-clock-history-length 8)
          (org-clock-persist nil)
          (org-clock-out-remove-zero-time-clocks t))
      (org-mode)
      (insert "#+PROPERTY: Status_ALL Todo Doing Blocked Done\n")
      (insert "* Project :work:\n")
      (insert ":PROPERTIES:\n:Client: Acme\n:Sprint: S1\n:END:\n")
      (insert "** TODO Alpha :billable:\n")
      (insert ":PROPERTIES:\n:Status: Todo\n:Owner: Ada\n:END:\n")
      (insert "Alpha body\n")
      (insert "** TODO Beta :internal:\n")
      (insert ":PROPERTIES:\n:Owner: Bea\n:END:\n")
      (insert "Beta body\n")
      (insert "*** WAIT Beta child :blocked:\n")
      (insert ":PROPERTIES:\n:Status: Blocked\n:Owner: Cy\n:END:\n")
      (insert "Child body\n")
      (insert "* Tail\nTail body\n")
      (let ((snapshot
             (lambda (label)
               (list label
                     (mapcar
                      (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (search-forward needle)
                          (list needle
                                (line-number-at-pos)
                                (invisible-p (point))
                                (org-element-type
                                 (org-element-at-point)))))
                      '("Project" ":Client:" "Alpha" ":Status:" "Alpha body"
                        "CLOCK:" "Beta" "Beta child" "Child body" "Tail"))
                     (org-element-map (org-element-parse-buffer)
                         '(headline drawer property-drawer clock planning)
                       (lambda (el)
                         (list (org-element-type el)
                               (org-element-property :begin el)
                               (org-element-property :end el)
                               (org-element-property :raw-value el)
                               (org-element-property :todo-keyword el)
                               (org-element-property :tags el))))
                     (save-excursion
                       (goto-char (point-min))
                       (let (out)
                         (while (re-search-forward "^\\*+ " nil t)
                           (push (list (org-get-heading t t t t)
                                       (org-entry-get nil "Client" 'inherit)
                                       (org-entry-get nil "Sprint" 'inherit)
                                       (org-entry-get nil "Status")
                                       (org-entry-get-multivalued-property
                                        nil "Multi")
                                       (get-text-property
                                        (line-beginning-position)
                                        :probe-minutes))
                                 out))
                         (nreverse out)))
                     (buffer-substring-no-properties
                      (point-min) (point-max))))))
        (let (states)
          (push (funcall snapshot 'initial) states)
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-entry-put nil "Status" "Doing")
          (org-entry-add-to-multivalued-property nil "Multi" "review")
          (org-entry-add-to-multivalued-property nil "Multi" "api")
          (org-clock-in nil (encode-time 0 0 9 27 5 2026))
          (org-clock-out nil t (encode-time 0 45 10 27 5 2026))
          (push (funcall snapshot 'after-alpha-clock) states)
          (goto-char (point-min))
          (search-forward "Beta child")
          (beginning-of-line)
          (org-entry-put nil "Sprint" "S2")
          (org-entry-remove-from-multivalued-property nil "Multi" "api")
          (org-clock-in nil (encode-time 0 15 11 27 5 2026))
          (org-clock-out nil t (encode-time 0 0 12 27 5 2026))
          (push (funcall snapshot 'after-child-clock) states)
          (goto-char (point-min))
          (org-clock-sum "2026-05-27" "2026-05-28" nil :probe-minutes)
          (push (funcall snapshot 'after-clock-sum) states)
          (org-fold-hide-drawer-all)
          (push (funcall snapshot 'drawers-hidden) states)
          (goto-char (point-min))
          (search-forward "CLOCK:")
          (org-fold-show-context 'default)
          (push (funcall snapshot 'clock-context) states)
          (org-fold-show-all)
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-entry-delete nil "Owner")
          (org-property-next-allowed-value)
          (push (funcall snapshot 'after-property-cycle) states)
          (list (nreverse states)
                (sort (copy-sequence (org-property-values "Owner"))
                      #'string<)
                (org-clock-sum-current-item "2026-05-27")
                (mapcar (lambda (m)
                          (and (markerp m)
                               (marker-buffer m)
                               (with-current-buffer (marker-buffer m)
                                 (save-excursion
                                   (goto-char m)
                                   (org-get-heading t t t t)))))
                        org-clock-history)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_property_space_multivalue_cleanup_parse_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-use-property-inheritance t)
          (changes nil))
      (add-hook 'org-property-changed-functions
                (lambda (key value) (push (list key value) changes))
                nil t)
      (org-mode)
      (insert "* Parent\n")
      (insert ":PROPERTIES:\n:Owner: Ada Lovelace\n:Multi: old value\n:END:\n")
      (insert "** Child\n")
      (insert ":PROPERTIES:\n:Local: keep\n:END:\n")
      (goto-char (point-min))
      (search-forward "Parent")
      (beginning-of-line)
      (org-entry-put-multivalued-property
       nil "Multi" "alpha beta" "gamma" "delta value")
      (let ((parent-before (org-entry-properties nil 'standard))
            (protected (mapcar #'org-entry-protect-space
                               '("alpha beta" "gamma" "delta value")))
            (restored (mapcar #'org-entry-restore-space
                              '("alpha_beta" "gamma" "delta_value"))))
        (search-forward "Child")
        (beginning-of-line)
        (let ((inherited-before
               (list (org-entry-get nil "Owner" 'inherit)
                     (org-entry-get-multivalued-property nil "Multi")
                     (org-entry-get-with-inheritance "Multi"))))
          (goto-char (point-min))
          (search-forward "Parent")
          (beginning-of-line)
          (org-entry-remove-from-multivalued-property
           nil "Multi" "alpha beta")
          (org-entry-delete nil "Owner")
          (org-entry-delete nil "Multi")
          (let ((tree (org-element-parse-buffer)))
            (list parent-before
                  protected
                  restored
                  inherited-before
                  (nreverse changes)
                  (org-entry-properties nil 'standard)
                  (org-element-map tree 'node-property
                    (lambda (node)
                      (list (org-element-property :key node)
                            (org-element-property :value node))))
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_startup_log_options_todo_property_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-todo-keywords
           '((sequence "TODO(t)" "WAIT(w@/!)" "|" "DONE(d!)" "CANCELED(c@)")))
          (org-log-note-headings
           '((done . "DONE %-12s from %-12S %t")
             (state . "STATE %-12s from %-12S %t")
             (note . "NOTE %t")))
          (org-log-note-clock-out nil))
      (org-mode)
      (insert "#+STARTUP: logdrawer lognotedone lognoterepeat nologreschedule logredeadline nologstatesreversed\n")
      (insert "#+PROPERTY: Owner_ALL Ada Bea Cy\n")
      (insert "* TODO Parent :project:\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "** TODO Child\n")
      (insert "SCHEDULED: <2026-05-27 Wed +1w>\n")
      (insert "DEADLINE: <2026-05-28 Thu>\n")
      (goto-char (point-min))
      (org-set-regexps-and-options)
      (let ((startup-settings
             (list org-log-into-drawer
                   org-log-done
                   org-log-repeat
                   org-log-reschedule
                   org-log-redeadline
                   org-log-states-order-reversed)))
        (search-forward "Child")
        (beginning-of-line)
        (let ((inherited-owner (org-entry-get nil "Owner" 'inherit))
              (allowed-owner
               (org-property-get-allowed-values nil "Owner" 'table)))
          (org-set-property "Owner" "Bea")
          (org-schedule nil "2026-06-03")
          (org-deadline nil "2026-06-04")
          (org-todo "WAIT")
          (when (and (boundp 'org-log-note-marker)
                     org-log-note-marker
                     (marker-buffer org-log-note-marker))
            (with-current-buffer (marker-buffer org-log-note-marker)
              (goto-char org-log-note-marker)
              (insert "Waiting on review")
              (org-add-log-note)))
          (org-todo "DONE")
          (when (and (boundp 'org-log-note-marker)
                     org-log-note-marker
                     (marker-buffer org-log-note-marker))
            (with-current-buffer (marker-buffer org-log-note-marker)
              (goto-char org-log-note-marker)
              (insert "Finished after review")
              (org-add-log-note)))
          (list startup-settings
                inherited-owner
                allowed-owner
                (org-entry-properties nil 'standard)
                (org-log-beginning nil)
                (replace-regexp-in-string
                 "\\[[0-9][^]\n]+\\]"
                 "[stamp]"
                 (buffer-substring-no-properties
                  (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_property_inherit_literal_special_views_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-use-property-inheritance '("Owner" "NilLike" "Effort"))
          (org-property-format ":%s: %s")
          (changes nil))
      (add-hook 'org-property-changed-functions
                (lambda (key value) (push (list key value) changes))
                nil t)
      (org-mode)
      (insert "#+CATEGORY: DemoCat\n")
      (insert "#+PROPERTY: Owner_ALL Ada Bea Cy\n")
      (insert "#+COLUMNS: %25ITEM %TODO %PRIORITY %Owner %Effort{:}\n")
      (insert "* TODO Parent [#B]\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:NilLike: nil\n:Effort: 1:00\n:END:\n")
      (insert "** WAIT Child [#C]\n")
      (insert ":PROPERTIES:\n:Local: child-only\n:END:\n")
      (goto-char (point-min))
      (org-set-regexps-and-options)
      (search-forward "Child")
      (beginning-of-line)
      (let ((inherit-flags
             (mapcar #'org-property-inherit-p
                     '("Owner" "Local" "Effort" "CATEGORY" "NilLike")))
            (literal-values
             (list (org-entry-get nil "NilLike" 'inherit)
                   (org-entry-get nil "NilLike" 'inherit 'literal-nil)
                   (org-entry-get-with-inheritance "NilLike")
                   (org-entry-get-with-inheritance "NilLike" 'literal-nil)))
            (special-before
             (list (org-entry-get nil "TODO")
                   (org-entry-get nil "PRIORITY")
                   (org-entry-get nil "CATEGORY")
                   (org-entry-get nil "ITEM")
                   (org-property-or-variable-value "COLUMNS" 'inherit)))
            (props-standard-before (org-entry-properties nil 'standard))
            (props-special-before (org-entry-properties nil 'special))
            (props-all-before (org-entry-properties nil)))
        (org-entry-put nil "TODO" "DONE")
        (org-entry-put nil "PRIORITY" "A")
        (org-entry-put nil "Owner" "Bea")
        (org-entry-put nil "NilLike" nil)
        (org-entry-delete nil "Local")
        (list inherit-flags
              literal-values
              special-before
              props-standard-before
              props-special-before
              props-all-before
              (nreverse changes)
              (org-entry-properties nil 'standard)
              (org-entry-properties nil 'special)
              (org-entry-get nil "TODO")
              (org-entry-get nil "PRIORITY")
              (org-entry-get nil "NilLike" 'inherit 'literal-nil)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}
