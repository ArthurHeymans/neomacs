use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};
#[test]
fn combo96_org_priorities_beyond_ABC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
 (with-temp-buffer (org-mode) (insert "* [#A] H\n") (goto-char (point-min))
  (let* ((t (org-element-parse-buffer)) (h (car (org-element-map t 'headline #'identity))))
  (list :priority (org-element-property :priority h) :value (org-priority-to-value (org-element-property :priority h))))))"##,
    );
}
#[test]
fn combo96_org_table_higher_order_formula() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "| a | b | c | d |\n| 1 | 2 | 3 |   |\n| 4 | 5 | 6 |   |\n|   |   |   |   |\n")
 (insert "#+TBLFM: $4=$1+$2+$3::@>$4=vsum(@2..@-1)\n")
 (let ((r '())) (goto-char (point-min)) (org-table-recalculate t) (org-table-align) (goto-char (point-min))
  (forward-line 1) (push (list :row1-d (org-table-get "d" nil)) r) (forward-line)
  (push (list :row2-d (org-table-get "d" nil)) r) (nreverse r)))"##,
    );
}
#[test]
fn combo96_org_agenda_time_grid_settings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-agenda) (list
 :time-grid-fbound (boundp 'org-agenda-time-grid) :time-leading-fbound (boundp 'org-agenda-time-leading-zero)
 :weekend-fbound (boundp 'org-agenda-weekend-days)))"##,
    );
}
#[test]
fn combo96_org_duration_custom_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-duration) (list
 :hmm (let ((org-duration-format 'h:mm)) (org-duration-from-minutes 90))
 :default (org-duration-from-minutes 60)
 :roundtrip (let ((m (org-duration-to-minutes "1:15"))) (org-duration-from-minutes m))))"##,
    );
}
#[test]
fn combo96_org_mode_hooks_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org) (list
 :mode-load-hook (boundp 'org-mode-load-hook) :mode-hook (boundp 'org-mode-hook)
 :before-save-hook (boundp 'org-mode-hook)))"##,
    );
}
#[test]
fn combo96_org_local_variables_iteration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "# -*- mode: org; org-todo-keywords: '((sequence \"TODO\" \"|\" \"DONE\")) -*-\n* TODO X\n")
 (goto-char (point-min)) (condition-case nil (hack-local-variables) (error nil))
 (push (list :todo-kw (when (boundp 'org-todo-keywords) org-todo-keywords)) nil) (list :ok t))"##,
    );
}
#[test]
fn combo96_org_entities_all_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-entities) (list
 :entities-count (length org-entities) :first-name (car (car org-entities)) :last-name (car (car (last org-entities)))))"##,
    );
}
#[test]
fn combo96_org_image_resolutions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org) (list
 :image-fbound (fboundp 'org--create-inline-image) :display-attr-fbound (fboundp 'org-display-inline-images)))"##,
    );
}
#[test]
fn combo96_agenda_sequence_and_org_mode_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode) (require 'org-agenda)
 (insert "* TODO X\n* DONE Y\n") (let ((r '()))
  (push (list :todos (org-map-entries (lambda () (org-get-heading t t t t)) "TODO=\"TODO\"")) r)
  (push (list :dones (org-map-entries (lambda () (org-get-heading t t t t)) "TODO=\"DONE\"")) r)
  (nreverse r)))"##,
    );
}
#[test]
fn combo96_org_shift_tab_global_cycle_complete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "* A\n** B\nBody.\n** C\nBody.\n* D\nBody.\n") (let ((r '()))
  (goto-char (point-min)) (org-shifttab 3) (push (list :overview-invis (get-char-property (point) 'invisible)) r)
  (org-shifttab 3) (push (list :contents-invis (get-char-property (point) 'invisible)) r)
  (org-shifttab 3) (push (list :all-invis (get-char-property (point) 'invisible)) r)
  (nreverse r)))"##,
    );
}
