//! Strict combo oracle probes, batch 97: list navigation — forward-list,
//! backward-list, up-list, down-list across nested parentheses.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_r1_list_navigation_forward_backward_up_down() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a (b (c d)) e)")
  (goto-char 1)
  (down-list)
  (let ((d1 (point)))
    (forward-list)
    (let ((f1 (point)))
      (down-list)
      (let ((d2 (point)))
        (up-list 2)
        (list d1 f1 d2 (point))))))
"##,
    );
}

#[test]
fn div_r1_backward_list_and_up_list_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "xxx (a b) yyy (c d) zzz")
  (goto-char 25)
  (backward-list)
  (let ((b1 (point)))
    (condition-case err (up-list -1) (scan-error (list 'err)))
    (list b1 (point)
          (condition-case err (up-list 99) (scan-error (car err))))))
"##,
    );
}
