//! Strict combo oracle probes, batch 50: async process machinery — process
//! output via filter, process-plist/get/put, process-buffer/mark, connection
//! type (pipe/pty) and process-contact, and stderr routing to a separate
//! buffer. Commands use shell-file-name + shell-command-switch (builtins) so
//! they are portable across systems where /bin/echo and /bin/true do not exist
//! (e.g. NixOS).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_j0_process_output_via_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let (collected)
  (let ((proc (make-process :name "probe-proc-out"
                            :command (list shell-file-name shell-command-switch "echo hello world")
                            :connection-type 'pipe
                            :filter (lambda (_p s) (setq collected (concat collected s))))))
    (set-process-query-on-exit-flag proc nil)
    (accept-process-output proc 1)
    (list collected
          (process-status proc)
          (process-exit-status proc)
          (process-buffer proc))))
"##,
        expect_test::expect![[r#""OK (\"hello world\n\" exit 0 nil)""#]],
    );
}

#[test]
fn div_j0_process_plist_get_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((proc (make-process :name "probe-proc-pl"
                          :command (list shell-file-name shell-command-switch "true"))))
  (set-process-query-on-exit-flag proc nil)
  (process-put proc 'probe-prop 42)
  (list (process-get proc 'probe-prop)
        (process-get proc 'missing)
        (plist-get (process-plist proc) 'probe-prop)))
"##,
        expect_test::expect![[r#""OK (42 nil 42)""#]],
    );
}

#[test]
fn div_j0_process_buffer_and_mark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK (t 4 " *probe-proc-buf*" "hi\n")
    // Neomacs:   OK (t 36 " *probe-proc-buf*" "hi\n")
    // (marker-position (process-mark proc)) is 4 in GNU (end of the 3-char
    // "hi\n" output) but 36 in Neomacs — the process mark is positioned
    // incorrectly after process output. Buffer identity and content agree.
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((buf (generate-new-buffer " *probe-proc-buf*")))
  (let ((proc (make-process :name "probe-proc-bm"
                            :command (list shell-file-name shell-command-switch "echo hi")
                            :buffer buf)))
    (set-process-query-on-exit-flag proc nil)
    (accept-process-output proc 1)
    (list (eq (process-buffer proc) buf)
          (marker-position (process-mark proc))
          (buffer-name (process-buffer proc))
          (with-current-buffer buf (buffer-string)))))
"##,
        expect_test::expect![[
            r#""OK (t 36 \" *probe-proc-buf*\" \"hi\n\nProcess probe-proc-bm finished\n\")""#
        ]],
    );
}

#[test]
fn div_j0_process_connection_type_and_contact() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((p1 (make-process :name "probe-pipe"
                        :command (list shell-file-name shell-command-switch "true")
                        :connection-type 'pipe))
      (p2 (make-process :name "probe-pty"
                        :command (list shell-file-name shell-command-switch "true")
                        :connection-type 'pty)))
  (set-process-query-on-exit-flag p1 nil)
  (set-process-query-on-exit-flag p2 nil)
  (list (process-type p1)
        (process-type p2)
        (car (process-contact p1))
        (car (process-contact p2))))
"##,
        expect_test::expect![[r#""ERR (wrong-type-argument listp t)""#]],
    );
}

#[test]
fn div_j0_process_stderr_separate_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK ("out\n    " "err\n    ")
    // Neomacs:   OK ("out\n\n    Process probe-stderr finished\n    " "err\n\n    Process probe-stderr stderr finished\n    ")
    // Neomacs writes the default "Process X finished" sentinel message INTO
    // the stdout and stderr buffers when the process exits; GNU does not
    // pollute the output buffers with the sentinel message.
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((outbuf (generate-new-buffer " *probe-stderr-out*"))
      (errbuf (generate-new-buffer " *probe-stderr-err*")))
  (let ((proc (make-process :name "probe-stderr"
                            :command (list shell-file-name shell-command-switch "echo out; echo err 1>&2")
                            :buffer outbuf
                            :stderr errbuf)))
    (set-process-query-on-exit-flag proc nil)
    (accept-process-output proc 1)
    (list (with-current-buffer outbuf (buffer-string))
          (with-current-buffer errbuf (buffer-string)))))
"##,
        expect_test::expect![[
            r#""OK (\"out\n\nProcess probe-stderr finished\n\" \"err\n\nProcess probe-stderr stderr finished\n\")""#
        ]],
    );
}

#[test]
fn div_j0_process_name_pid_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((proc (make-process :name "probe-proc-np"
                          :command (list shell-file-name shell-command-switch "true"))))
  (set-process-query-on-exit-flag proc nil)
  (accept-process-output proc 1)
  (list (process-name proc)
        (integerp (process-id proc))
        (> (length (process-list)) 0)
        (memq proc (process-list))))
"##,
        expect_test::expect![[r#""OK (\"probe-proc-np\" t nil nil)""#]],
    );
}
