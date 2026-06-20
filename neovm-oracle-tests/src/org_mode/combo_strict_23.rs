//! combo_strict_23.rs — babel ob-ditaa/ledger/matlab/ocaml/perl/
//! processing/ruby/scheme/sqlite, clock cancel/goto/in-last/
//! modify-effort, capture goto-target/kill, agenda deadline/
//! schedule/date-later/earlier, table current-dline/column/move.
use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};
#[test]
fn strict_babel_more_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (list
 :ob-perl (condition-case nil (require 'ob-perl) (error (featurep 'ob-perl)))
 :ob-ruby (condition-case nil (require 'ob-ruby) (error (featurep 'ob-ruby)))
 :ob-scheme (condition-case nil (require 'ob-scheme) (error (featurep 'ob-scheme)))
 :ob-ocaml (condition-case nil (require 'ob-ocaml) (error (featurep 'ob-ocaml)))
 :ob-processing (condition-case nil (require 'ob-processing) (error (featurep 'ob-processing)))
 :ob-ledger (condition-case nil (require 'ob-ledger) (error (featurep 'ob-ledger)))
 :ob-matlab (condition-case nil (require 'ob-matlab) (error (featurep 'ob-matlab)))
 :ob-eshell (condition-case nil (require 'ob-eshell) (error (featurep 'ob-eshell)))
 ))"##,
    );
}
#[test]
fn strict_clock_cancel_goto_in_last() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-clock) (list
 :cancel-fbound (fboundp 'org-clock-cancel) :goto-fbound (fboundp 'org-clock-goto)
 :in-last-fbound (fboundp 'org-clock-in-last) :modify-effort-fbound (fboundp 'org-clock-modify-effort-estimate)
 ))"##,
    );
}
#[test]
fn strict_capture_goto_target_kill() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-capture) (list
 :goto-target-fbound (fboundp 'org-capture-goto-target) :kill-fbound (fboundp 'org-capture-kill)
 :finalize-fbound (fboundp 'org-capture-finalize)))"##,
    );
}
#[test]
fn strict_agenda_date_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-agenda) (list
 :date-later-fbound (fboundp 'org-agenda-date-later) :date-earlier-fbound (fboundp 'org-agenda-date-earlier)
 :schedule-fbound (fboundp 'org-agenda-schedule) :deadline-fbound (fboundp 'org-agenda-deadline)
 :do-date-later (fboundp 'org-agenda-do-date-later)))"##,
    );
}
#[test]
fn strict_table_current_dline_column_move() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org) (list
 :current-dline-fbound (fboundp 'org-table-current-dline) :current-column-fbound (fboundp 'org-table-current-column)
 :move-column-fbound (fboundp 'org-table-move-column) :goto-column-fbound (fboundp 'org-table-goto-column)))"##,
    );
}
#[test]
fn strict_element_create_planning_clock_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-element) (list
 :planning (org-element-type (org-element-create 'planning nil))
 :clock (org-element-type (org-element-create 'clock '(:status closed :value "[2024-01-01 Mon 10:00]--[2024-01-01 Mon 11:00]")))
 :drawer (org-element-type (org-element-create 'drawer '(:drawer-name "NOTES")))
 :node-property (org-element-type (org-element-create 'node-property '(:key "ID" :value "abc")))
 ))"##,
    );
}
#[test]
fn strict_export_footnote_first_ref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ox) (list
 :footnote-first-ref-fbound (fboundp 'org-export-footnote-first-reference-p)
 :footnote-marker-fbound (fboundp 'org-export-get-footnote-number)))"##,
    );
}
#[test]
fn strict_fold_core_style() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (condition-case nil (require 'org-fold-core) (error nil)) (list
 :style-fbound (boundp 'org-fold-core-style) :style (when (boundp 'org-fold-core-style) org-fold-core-style)
 :ellipsis-fbound (boundp 'org-fold-core-ellipsis)))"##,
    );
}
#[test]
fn strict_babel_ref_resolve_2level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode) (require 'ob-emacs-lisp) (require 'ob-ref)
 (let ((org-confirm-babel-evaluate nil))
  (insert "#+name: step1\n| 7 |\n| 8 |\n\n")
  (insert "#+begin_src emacs-lisp :results value :var d=step1(+)\n(apply #'+ (mapcar #'car d))\n#+end_src\n")
  (let ((r '())) (goto-char (point-min)) (search-forward "#+begin_src")
   (push (org-babel-execute-src-block) r) (nreverse r))))"##,
    );
}
#[test]
fn strict_org_emphasis_with_prefix_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "some text to mark")
 (let ((r '())) (goto-char (point-min)) (search-forward "text")
  (set-mark (match-beginning 0)) (search-forward "to") (backward-char 1)
  (condition-case nil (org-emphasize ?*) (error nil))
  (push (list :after (buffer-string)) r)
  (push (list :bold-count (length (org-element-map (org-element-parse-buffer) 'bold #'identity))) r)
  (nreverse r)))"##,
    );
}
