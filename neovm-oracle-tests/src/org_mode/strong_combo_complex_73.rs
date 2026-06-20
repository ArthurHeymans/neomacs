//! Strong combo-complex-73/74 oracle tests — esoteric probes:
//! org-submit-bug-report, org-element with org-element-parse-
//! secondary-string nested, org-table with org-table-rotate-
//! recalculate-marks, org-agenda with filter-preset interactions,
//! org-babel with ob-java/ob-js/ob-julia availability,
//! org-export with org-export-data-for-backend, org-persist
//! with gc cycle, org-habit with org-habit-build-graph,
//! and org-compat with org-with-point-at boundary.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn combo73_submit_bug_report_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
  (list
   :submit-fbound (fboundp 'org-submit-bug-report)
   :version-fbound (fboundp 'org-version)
   ))"##,
    );
}

#[test]
fn combo73_element_parse_secondary_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  ;; parse-secondary-string with nested bold-inside-italic spec
  (let ((result (org-element-parse-secondary-string
                  "nested *bold /italic!/* end" '(bold italic))))
    (list
     :result-type (type-of result)
     :has-bold (org-element-map result 'bold #'identity)
     :has-italic (org-element-map result 'italic #'identity)
     )))"##,
    );
}

#[test]
fn combo73_table_rotate_recalc_marks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
  (list
   :rotate-fbound (fboundp 'org-table-rotate-recalculate-marks)
   ))"##,
    );
}

#[test]
fn combo73_agenda_filter_preset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-agenda)
  (list
   :filter-preset-bound (boundp 'org-agenda-filter-preset)
   :filter-fbound (fboundp 'org-agenda-filter-by-tag)
   :filter-category-fbound (fboundp 'org-agenda-filter-by-category)
   :filter-effort-fbound (fboundp 'org-agenda-filter-by-effort)
   :top-headline-fbound (fboundp 'org-agenda-filter-by-top-headline)
   ))"##,
    );
}

#[test]
fn combo73_babel_esoteric_langs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (list
   :ob-java (condition-case nil (require 'ob-java) (error (featurep 'ob-java)))
   :ob-js (condition-case nil (require 'ob-js) (error (featurep 'ob-js)))
   :ob-julia (condition-case nil (require 'ob-julia) (error (featurep 'ob-julia)))
   :ob-sed (condition-case nil (require 'ob-sed) (error (featurep 'ob-sed)))
   :ob-screen (condition-case nil (require 'ob-screen) (error (featurep 'ob-screen)))
   ))"##,
    );
}

#[test]
fn combo73_export_data_for_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode) (require 'ox) (require 'ox-ascii)
  (insert "*bold* /italic/.\n")
  (let* ((tree (org-element-parse-buffer))
         (info (org-export-get-environment))
         (para (car (org-element-map tree 'paragraph #'identity)))
         (r '()))
    (push (list :export-string-fbound (fboundp 'org-export-string-as)) r)
    ;; org-export-data-for-backend
    (condition-case nil
        (let ((out (when (fboundp 'org-export-data-with-backend)
                     (org-export-data-with-backend para info 'ascii))))
          (push (list :data-ok (and out (stringp out))) r))
      (error nil))
    (nreverse r)))"##,
    );
}

#[test]
fn combo73_persist_gc_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-persist)
  (list
   :gc-fbound (fboundp 'org-persist-gc)
   :read-fbound (fboundp 'org-persist-read)
   :write-fbound (fboundp 'org-persist-write)
   ))"##,
    );
}

#[test]
fn combo73_habit_build_graph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-habit)
  (list
   :build-graph-fbound (fboundp 'org-habit-build-graph)
   :parse-todo-fbound (fboundp 'org-habit-parse-todo)
   :is-habit-fbound (fboundp 'org-is-habit-p)
   ))"##,
    );
}

#[test]
fn combo73_compat_with_point_at() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-compat)
  (list
   :with-point-at-fbound (fboundp 'org-with-point-at)
   :with-silent-fbound (fboundp 'org-with-silent-modifications)
   :with-wide-buffer-fbound (fboundp 'org-with-wide-buffer)
   :format-time-fbound (fboundp 'org-format-time-string)
   ))"##,
    );
}

#[test]
fn combo73_org_agenda_time_of_day() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-agenda)
  (list
   :time-of-day-fbound (fboundp 'org-agenda-time-of-day-to-ampm)
   :format-item-fbound (fboundp 'org-agenda-format-item)
   :add-time-grid-fbound (fboundp 'org-agenda-add-time-grid-maybe)
   ))"##,
    );
}
