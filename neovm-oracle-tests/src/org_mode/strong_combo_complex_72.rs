//! Strong combo-complex-72 — absolute final probes: org-babel
//! with ob-table, org-element with org-element-put-property
//! stress, org-timer with org-timer-item-insert, org-export
//! with async (if bound), org-agenda with org-agenda-get-blocks,
//! org-entities with org-entities-restricted, org-macro with
//! org-macro--collect-macros, and org-publish with current file.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn combo72_babel_ob_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (condition-case nil (require 'ob-table) (error nil))
  (list
   :ob-table-loaded (featurep 'ob-table)
   :sbe-fbound (fboundp 'org-sbe)
   ))"##,
    );
}

#[test]
fn combo72_element_put_property_stress() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-element)
  (insert "* H\nBody.\n")
  (let ((r '()))
    (let* ((tree (org-element-parse-buffer))
           (hl (car (org-element-map tree 'headline #'identity))))
      ;; put many properties
      (org-element-put-property hl :custom-prop-1 "value1")
      (org-element-put-property hl :custom-prop-2 42)
      (org-element-put-property hl :custom-prop-3 '(a b c))
      (push (list :prop1 (org-element-property :custom-prop-1 hl)) r)
      (push (list :prop2 (org-element-property :custom-prop-2 hl)) r)
      (push (list :prop3 (org-element-property :custom-prop-3 hl)) r)
      ;; original properties still there
      (push (list :level (org-element-property :level hl)) r)
      (push (list :raw (substring-no-properties (org-element-property :raw-value hl))) r))
    (nreverse r)))"##,
    );
}

#[test]
fn combo72_timer_item_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-timer)
  (let ((r '()))
    (push (list :item-insert-fbound (fboundp 'org-timer-item)) r)
    (condition-case nil
        (progn (org-timer-item 1)
               (push (list :inserted (buffer-string)) r))
      (error (push (list :insert-error t) r)))
    (nreverse r)))"##,
    );
}

#[test]
fn combo72_export_async() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (list
   :async-export-fbound (boundp 'org-export-async-init-file)
   :in-background-fbound (fboundp 'org-export-in-background)
   ))"##,
    );
}

#[test]
fn combo72_agenda_get_blocks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-agenda)
  (list
   :get-blocks-fbound (fboundp 'org-agenda-get-blocks)
   :get-day-entries-fbound (fboundp 'org-agenda-get-day-entries)
   :get-timeline-fbound (fboundp 'org-agenda-get-timeline)
   ))"##,
    );
}

#[test]
fn combo72_entities_restricted() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-entities)
  (list
   :restricted-fbound (boundp 'org-entities-restricted)
   :restricted (when (boundp 'org-entities-restricted) org-entities-restricted)
   :user-fbound (boundp 'org-entities-user)
   :ascii-fbound (boundp 'org-entities-ascii-explanatory)
   ))"##,
    );
}

#[test]
fn combo72_macro_collect_macros() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: a alice\n#+MACRO: b bob\n")
  (let ((r '()))
    ;; org-macro--collect-macros
    (push (list :collect-fbound (fboundp 'org-macro--collect-macros)) r)
    (when (fboundp 'org-macro--collect-macros)
      (let ((macros (org-macro--collect-macros)))
        (push (list :collected-count (length macros)) r)))
    ;; org-macro-templates
    (push (list :templates-bound (boundp 'org-macro-templates)) r)
    (nreverse r)))"##,
    );
}

#[test]
fn combo72_publish_current_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox-publish)
  (list
   :current-file-fbound (fboundp 'org-publish-current-file)
   :current-project-fbound (fboundp 'org-publish-current-project)
   :publish-fbound (fboundp 'org-publish)
   :all-fbound (fboundp 'org-publish-all)
   ))"##,
    );
}

#[test]
fn combo72_org_log_into_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (list
   :log-into-drawer-fbound (boundp 'org-log-into-drawer)
   :log-repeat-fbound (boundp 'org-log-repeat)
   :log-state-notes-insert-fbound (boundp 'org-log-state-notes-insert-after-drawers)
   :log-done-fbound (boundp 'org-log-done)
   ))"##,
    );
}

#[test]
fn combo72_org_export_snippet_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "@@html:<b>bold</b>@@ and @@latex:\\textbf{bold}@@\n")
  (let ((r '()))
    (let* ((tree (org-element-parse-buffer))
           (snippets (org-element-map tree 'export-snippet #'identity)))
      (push (list :snippet-count (length snippets)) r)
      (push (list :snippet-backends
                  (mapcar (lambda (s) (org-element-property :back-end s)) snippets)) r))
    (nreverse r)))"##,
    );
}
