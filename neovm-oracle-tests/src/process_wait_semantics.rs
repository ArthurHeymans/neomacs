//! Oracle parity tests for process wait/readiness semantics.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn compat_accept_process_output_drains_exited_process_io_matches_gnu_emacs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"
(let ((events nil)
      (p nil))
  (unwind-protect
      (progn
        (setq p
              (make-process
               :name "compat-drain-exited-process"
               :buffer nil
               :command (list "/bin/sh" "-c" "printf payload")
               :filter (lambda (_proc string)
                         (push (list 'filter string) events))
               :sentinel (lambda (_proc string)
                           (push (list 'sentinel string) events))))
        (let ((deadline (+ (float-time) 2.0)))
          (while (and (< (float-time) deadline)
                      (or (not (assq 'filter events))
                          (not (assq 'sentinel events))))
            (accept-process-output p 0.05)))
        (list (process-status p)
              (nreverse events)))
    (when p
      (ignore-errors (delete-process p)))))
"#,
    );
}
