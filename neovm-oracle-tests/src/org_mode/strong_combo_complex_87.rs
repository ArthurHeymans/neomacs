use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};
#[test]
fn combo87_error_org_export_no_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ox)
 (with-temp-buffer (org-mode) (condition-case e (org-export-as 'nonexistent nil nil t)
  (error (list :e (car e))))))"##,
    );
}
#[test]
fn combo87_error_org_table_recalc_bad_formula() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
 (with-temp-buffer (org-mode) (insert "| a | b |\n| 1 | 2 |\n")
  (insert "#+TBLFM: $3=$1+$2\n") (goto-char (point-min))
   (condition-case e (progn (org-table-recalculate t) :recalc-ok) (error (list :e (car e))))))"##,
    );
}
#[test]
fn combo87_error_org_clone_subtree_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
 (with-temp-buffer (org-mode) (insert "* A\n") (goto-char (point-min))
 (condition-case nil (org-clone-subtree-with-time-shift 0) (error :bad-count))))"##,
    );
}
#[test]
fn combo87_error_org_insert_link_no_desc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ol)
 (with-temp-buffer (org-mode) (condition-case nil (org-insert-link nil "https://x.com" nil)
  (error :bad-args))))"##,
    );
}
#[test]
fn combo87_error_org_clock_goto_no_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org-clock)
 (with-temp-buffer (org-mode) (insert "* H\n") (goto-char (point-min))
 (condition-case e (org-clock-goto) (error (list :e (car e))))))"##,
    );
}
#[test]
fn combo87_error_org_move_subtree_no_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
 (with-temp-buffer (org-mode) (insert "* H\n") (goto-char (point-min))
 (condition-case nil (org-move-subtree-up) (error :cannot-move))))"##,
    );
}
#[test]
fn combo87_error_org_mark_subtree_no_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
 (with-temp-buffer (org-mode) (goto-char (point-min))
 (condition-case nil (org-mark-subtree) (error :no-subtree))))"##,
    );
}
#[test]
fn combo87_error_org_update_statistics_no_cookie() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
 (with-temp-buffer (org-mode) (insert "* H\n") (goto-char (point-min))
 (condition-case nil (org-update-statistics-cookies nil) (error :no-cookie))))"##,
    );
}
#[test]
fn combo87_error_org_sort_entries_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'org)
 (with-temp-buffer (org-mode) (condition-case nil (org-sort-entries nil ?a) (error :no-entries))))"##,
    );
}
#[test]
fn combo87_error_org_export_file_missing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ox)
 (with-temp-buffer (org-mode) (insert "#+INCLUDE: \"/nonexistent-661/file.org\"\n")
 (condition-case nil (org-export-as 'ascii nil nil t) (error :include-error))))"##,
    );
}
