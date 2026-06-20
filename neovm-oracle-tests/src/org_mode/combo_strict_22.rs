//! Combo-strict-22 — esoteric corner probes: org-babel-ob-org
//! recursive, org-capture-dynamic, org-datetree-insert-line,
//! org-element-cache-after-change, org-footnote-new, org-habit-
//! toggle, org-id-locations-load/save, org-list-repair.
use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};
#[test]
fn strict_babel_ob_org_recursive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (condition-case nil (require 'ob-org) (error nil)) (list
 :ob-org-loaded (featurep 'ob-org) :org-exec-fbound (fboundp 'org-babel-execute:org)))"##,
    );
}
#[test]
fn strict_babel_ob_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (condition-case nil (require 'ob-eval) (error nil)) (list
 :ob-eval-loaded (featurep 'ob-eval) :eval-fbound (fboundp 'org-babel-eval)))"##,
    );
}
#[test]
fn strict_capture_dynamic_insertion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-capture) (list
 :dynamic-fbound (fboundp 'org-capture-inside-embedded-elisp-p)
 :expand-fbound (fboundp 'org-capture-expand-embedded-elisp)
 :fill-fbound (fboundp 'org-capture-fill-template)
 :template-fbound (boundp 'org-capture-templates)))"##,
    );
}
#[test]
fn strict_datetree_insert_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode) (require 'org-datetree)
 (let ((r '())) (goto-char (point-min))
  (condition-case nil (let ((pos (org-datetree-find-date-create (org-today))))
    (push (list :created (and pos (numberp pos))) r)
    (push (list :heads (length (org-element-map (org-element-parse-buffer) 'headline #'identity))) r))
   (error (push :error r))) (nreverse r)))"##,
    );
}
#[test]
fn strict_element_cache_after_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-element) (list
 :after-change-fbound (fboundp 'org-element--cache-after-change)
 :before-change-fbound (fboundp 'org-element--cache-before-change)
 :cache-sync-fbound (fboundp 'org-element--cache-sync)))"##,
    );
}
#[test]
fn strict_footnote_new() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org) (list
 :footnote-new-fbound (fboundp 'org-footnote-new)
 :insert-fbound (fboundp 'org-insert-footnote-reference-numeric-definition)))"##,
    );
}
#[test]
fn strict_habit_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-habit) (list
 :toggle-fbound (fboundp 'org-habit-toggle-display)
 :insert-fbound (fboundp 'org-habit-insert-consistency-graphs)
 :build-graph-fbound (fboundp 'org-habit-build-graph)))"##,
    );
}
#[test]
fn strict_id_locations_load_save() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-id) (list
 :locations-load-fbound (fboundp 'org-id-locations-load)
 :locations-save-fbound (fboundp 'org-id-locations-save)
 :locations-file-bound (boundp 'org-id-locations-file)
 :add-fbound (fboundp 'org-id-add-location)))"##,
    );
}
#[test]
fn strict_list_repair() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-list) (list
 :repair-fbound (fboundp 'org-list-repair)
 :repair-bullet-fbound (fboundp 'org-list-bullet-string)
 :indent-item-fbound (fboundp 'org-list-indent-item-generic)))"##,
    );
}
#[test]
fn strict_mobile_suom_agenda() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-mobile) (list
 :sumo-fbound (fboundp 'org-mobile-create-sumo-agenda)
 :push-fbound (fboundp 'org-mobile-push)
 :pull-fbound (fboundp 'org-mobile-pull)))"##,
    );
}
