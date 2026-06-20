use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};
#[test]
fn combo103_org_entry_get_with_inherit_selective() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "* Root\n:PROPERTIES:\n:A: 1\n:B: 2\n:END:\n** Leaf\n")
 (let ((r '())) (goto-char (point-max)) (let ((org-use-property-inheritance '("A")))
  (push (list :A-inherit (org-entry-get (point) "A" t)) r)
  (push (list :B-no-inherit (org-entry-get (point) "B" t)) r)) (nreverse r)))"##,
    );
}
#[test]
fn combo103_org_columns_current_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "| a | b | c |\n| 1 | 2 | 3 |\n") (goto-char (point-min)) (forward-line 1) (forward-char 2)
 (list :col-name (org-table-current-column) :dline (org-table-current-dline)))"##,
    );
}
#[test]
fn combo103_org_babel_parse_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ob-core) (list
 :inline-fbound (fboundp 'org-babel-read) :parse-fbound (fboundp 'org-babel-parse-inline-src-block-match)))"##,
    );
}
#[test]
fn combo103_org_lint_specific_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode) (require 'org-lint)
 (insert "* H\n:PROPERTIES:\n:CUSTOM_ID: dupe\n:END:\n** H2\n:PROPERTIES:\n:CUSTOM_ID: dupe\n:END:\n")
 (let ((r '())) (condition-case nil (let ((reports (org-lint)))
  (push (list :report-count (length reports)) r)) (error (push :lint-err r))) (nreverse r)))"##,
    );
}
#[test]
fn combo103_org_timestamp_hms_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
 (list (org-timestamp-format (org-timestamp-from-string "<2024-01-01 Mon 09:05:03>") "%H:%M:%S")
  (org-timestamp-format (org-timestamp-from-string "<2024-01-01 Mon 09:05>") "%I:%M %p")
  (org-timestamp-format (org-timestamp-from-string "<2024-01-01 Mon>") "%Y-%m-%d")))"##,
    );
}
#[test]
fn combo103_org_parse_dates_in_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "* H\nDEADLINE: <2024-12-31 Tue>\nSCHEDULED: <2024-01-01 Mon +1w>\n")
 (let* ((t (org-element-parse-buffer)) (pl (car (org-element-map t 'planning #'identity))) (r '()))
  (when pl (push (list :deadline-ok (and pl (org-element-property :deadline pl) t)) r)
   (push (list :scheduled-ok (and pl (org-element-property :scheduled pl) t)) r)) (nreverse r)))"##,
    );
}
#[test]
fn combo103_org_babel_execute_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org) (list
 :macro-fbound (fboundp 'org-babel-macro) :macro-loaded t))"##,
    );
}
#[test]
fn combo103_org_table_wrap_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "a,b,c\n1,2,3\n4,5,6\n") (goto-char (point-min)) (org-table-convert-region (point-min) (point-max) '(4))
 (goto-char (point-min)) (condition-case nil (org-table-align) (error :align-err))
 (list :to-lisp (org-table-to-lisp)))"##,
    );
}
#[test]
fn combo103_org_footnote_move_to_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode)
 (insert "Text[fn:1]\n[fn:1] Def.\n") (let ((r '())) (goto-char (point-min)) (search-forward "[fn:1]") (backward-char 2)
  (push (list :on-ref (org-element-type (org-element-context))) r) (nreverse r)))"##,
    );
}
#[test]
fn combo103_org_search_view_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer (org-mode) (require 'org-agenda)
 (insert "* Apple\n** Banana\n* Cherry\n** Date\n") (let ((r '()))
  (org-occur "a") (push (list :matched (org-element-map (org-element-parse-buffer nil t) 'headline
   (lambda (h) (substring-no-properties (org-element-property :raw-value h))))) r)
  (org-remove-occur-highlights) (nreverse r)))"##,
    );
}
