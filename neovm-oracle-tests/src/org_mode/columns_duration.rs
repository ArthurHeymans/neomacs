use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_columns_compute_summaries_and_update_properties_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-colview)
  (with-temp-buffer
    (org-mode)
    (insert "#+COLUMNS: %24ITEM %Effort{:} %Points{+;%.1f} %Done{X/} %Check{X%}\n")
    (insert "* Project\n")
    (insert ":PROPERTIES:\n:Effort: 0:00\n:Points: 0\n:Done: [ ]\n:Check: [ ]\n:END:\n")
    (insert "** TODO Alpha\n")
    (insert ":PROPERTIES:\n:Effort: 1:15\n:Points: 2.5\n:Done: [X]\n:Check: [X]\n:END:\n")
    (insert "** TODO Beta\n")
    (insert ":PROPERTIES:\n:Effort: 0:45\n:Points: 3.0\n:Done: [ ]\n:Check: [ ]\n:END:\n")
    (goto-char (point-min))
    (search-forward "Project")
    (beginning-of-line)
    (org-columns nil)
    (org-columns-quit)
    (list
     (org-entry-get nil "Effort")
     (org-entry-get nil "Points")
     (org-entry-get nil "Done")
     (org-entry-get nil "Check")
     (mapcar (lambda (spec)
               (cons (car spec)
                     (get-text-property (point) 'org-summaries)))
             org-columns-current-fmt-compiled)
     (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn org_columns_capture_view_filter_skip_indent_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-colview)
  (with-temp-buffer
    (org-mode)
    (insert "* Root :keep:\n")
    (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
    (insert "** TODO Visible :work:\n")
    (insert ":PROPERTIES:\n:Effort: 0:30\n:Owner: Bea\n:END:\n")
    (insert "*** TODO Too deep :work:\n")
    (insert ":PROPERTIES:\n:Effort: 0:10\n:END:\n")
    (insert "** TODO Empty :work:\n")
    (insert "** TODO Hidden :skip:work:\n")
    (insert ":PROPERTIES:\n:Effort: 0:20\n:END:\n")
    (goto-char (point-min))
    (org-columns--capture-view
     2 "+work" t '("skip")
     "%20ITEM(Task) %TODO(State) %Effort{:} %Owner"
     nil)))"##,
    );
}

#[test]
fn org_duration_custom_units_columns_time_summary_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-duration)
  (require 'org-colview)
  (with-temp-buffer
    (let ((org-duration-units '(("min" . 1) ("h" . 60) ("d" . 480)
                                ("sprint" . 1200)))
          (org-duration-format '(("d" . nil) ("h" . t) ("min" . t))))
      (org-duration-set-regexps)
      (org-mode)
      (insert "#+COLUMNS: %18ITEM %Effort{:} %Age{@mean}\n")
      (insert "* Project\n")
      (insert "** A\n:PROPERTIES:\n:Effort: 1d 2h 30min\n:Age: 2d\n:END:\n")
      (insert "** B\n:PROPERTIES:\n:Effort: 0d 1h 45min\n:Age: 4h\n:END:\n")
      (goto-char (point-min))
      (search-forward "Project")
      (beginning-of-line)
      (let ((org-columns--time
             (float-time (encode-time 0 0 12 27 5 2026))))
        (org-columns nil)
        (org-columns-quit))
      (list
       (mapcar #'org-duration-to-minutes
               '("1d 2h 30min" "1sprint 2h" "0d 1h 45min"))
       (mapcar #'org-duration-from-minutes '(630 1320 105))
       (org-entry-get nil "Effort")
       (org-entry-get nil "Age")
       (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}
