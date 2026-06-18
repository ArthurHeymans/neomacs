//! Process control / signal parity: kill/interrupt/signal-process,
//! process-running-child-p, list-system-processes, process-attributes,
//! plus the signal-0 no-op divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn proc_kill_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((proc (start-process "neo-kp-xxx" nil "sleep" "30")))
  (set-process-query-on-exit-flag proc nil)
  (kill-process proc)
  (while (process-live-p proc) (accept-process-output proc 0.1))
  (list (process-status proc) (memq (process-exit-status proc) '(9 15))))"##,
    );
}

#[test]
fn proc_signal_process_numeric() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((proc (start-process "neo-sn9-xxx" nil "sleep" "30")))
  (set-process-query-on-exit-flag proc nil)
  (signal-process proc 9)
  (while (process-live-p proc) (accept-process-output proc 0.1))
  (list (process-status proc) (process-exit-status proc)))"##,
    );
}

#[test]
fn proc_running_child_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((proc (start-process "neo-rc-xxx" nil "sleep" "30")))
  (set-process-query-on-exit-flag proc nil)
  (prog1 (condition-case e (progn (process-running-child-p proc) 'ok) (error (car e)))
    (delete-process proc)))"##,
    );
}

#[test]
fn proc_list_system_processes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((ps (list-system-processes)))
  (list (listp ps) (> (length ps) 0) (cl-every #'integerp ps)))"##,
    );
}

#[test]
fn proc_attributes_self() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((attrs (process-attributes (emacs-pid))))
  (list (listp attrs) (stringp (cdr (assq 'comm attrs))) (integerp (cdr (assq 'ppid attrs)))))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: (signal-process p 0) is a POSIX no-op existence check, but neomacs terminates the process (status 'signal, no longer live); GNU leaves it running."]
fn divergence_signal_process_signal0_noop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((proc (start-process "neo-s0-xxx" nil "sleep" "30")))
  (set-process-query-on-exit-flag proc nil)
  (sit-for 0.1)
  (let ((before (process-live-p proc))
        (ret (signal-process proc 0))
        (after (process-live-p proc))
        (status (process-status proc)))
    (delete-process proc)
    (list 'before before 'ret ret 'after after 'status status)))"##,
    );
}
