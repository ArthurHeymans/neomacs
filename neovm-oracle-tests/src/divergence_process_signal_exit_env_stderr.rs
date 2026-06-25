//! Confirmed divergences in process/subprocess handling, kept as
//! `#[ignore]`d oracle-parity tests that document the exact mismatch
//! (see each ignore reason). They fail today and guard the fix.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
#[ignore = "DIVERGENCE: signal-process rejects symbolic POSIX signal names (SIGKILL/SIGTERM/...) with \"Undefined signal name\"; GNU resolves the symbol/string to the numeric signal and delivers it."]
fn divergence_signal_process_symbolic_sigkill() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((msg nil) (proc (start-process "neo-ip-xxx" nil "sleep" "30")))
  (set-process-query-on-exit-flag proc nil)
  (set-process-sentinel proc (lambda (_p e) (setq msg e)))
  (signal-process proc 'SIGKILL)
  (while (process-live-p proc) (accept-process-output proc 0.1))
  (while (null msg) (accept-process-output proc 0.05))
  (list (process-status proc) (process-exit-status proc) (string-match "killed" msg)))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: signal-process rejects symbolic POSIX signal names (SIGKILL/SIGTERM/...) with \"Undefined signal name\"; GNU resolves the symbol/string to the numeric signal and delivers it."]
fn divergence_signal_process_symbolic_sigterm() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((proc (start-process "neo-ses-xxx" nil "sleep" "30")))
  (set-process-query-on-exit-flag proc nil)
  (signal-process proc 'SIGTERM)
  (while (process-live-p proc) (accept-process-output proc 0.1))
  (list (process-status proc) (process-exit-status proc)))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: process-exit-status collapses every child exit code >1 to 1 (e.g. exit 42 => GNU 42, neomacs 1); only 0 and 1 round-trip. Regressed by the \"signal exit status\" change."]
fn divergence_process_exit_status_nonzero_collapses() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((proc (start-process "neo-es-xxx" nil "sh" "-c" "exit 42")))
  (set-process-query-on-exit-flag proc nil)
  (while (process-live-p proc) (accept-process-output proc 1))
  (process-exit-status proc))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: setenv/process-environment changes are not exported to child processes; the child sees an empty value while GNU passes the new binding."]
fn divergence_setenv_not_exported_to_subprocess() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((process-environment (copy-sequence process-environment)))
  (setenv "NEO_TEST_VAR_XYZ" "value42")
  (list (getenv "NEO_TEST_VAR_XYZ")
        (shell-command-to-string "printf %s \"$NEO_TEST_VAR_XYZ\"")))"##,
    );
}

#[test]
fn divergence_make_process_stderr_buffer_ignored() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((obuf (generate-new-buffer " neo-o2-xxx")) (ebuf (generate-new-buffer " neo-e2-xxx")))
  (let ((p (make-process :name "neo-se2-xxx"
            :command '("sh" "-c" "echo OUT; echo ERR 1>&2")
            :buffer obuf :stderr ebuf :noquery t)))
    (while (process-live-p p) (accept-process-output p 0.1))
    (sit-for 0.2)
    (list (with-current-buffer obuf (buffer-string))
          (with-current-buffer ebuf (buffer-string)))))"##,
    );
}
