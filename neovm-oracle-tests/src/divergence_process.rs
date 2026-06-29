//! Process / subprocess divergence probes (calibration).
//!
//! Probes call-process (output capture, exit codes), shell-command-to-string,
//! make-process + accept-process-output, process-status/live-p/buffer/name/
//! mark, and process-list. Deterministic via simple echo/printf/true/false
//! subprocesses (both engines run them under --batch).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_proc_call_process_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (call-process "echo" nil t nil "hello")
  (buffer-string))
"##,
        expect_test::expect![[r#""OK \"hello\n\"""#]],
    );
}

#[test]
fn div_proc_call_process_printf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (call-process "printf" nil t nil "%s-%d" "abc" 42)
  (buffer-string))
"##,
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
}

#[test]
fn div_proc_call_process_exit_codes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (call-process "true")
      (call-process "false")
      (call-process "sh" nil nil nil "-c" "exit 7"))
"##,
        expect_test::expect![[r#""OK (0 1 7)""#]],
    );
}

#[test]
fn div_proc_shell_command_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (shell-command-to-string "echo hi")
      (shell-command-to-string "printf %s world"))
"##,
        expect_test::expect![[r#""OK (\"hi\n\" \"world\")""#]],
    );
}

#[test]
fn div_proc_make_process_accept_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (let ((p (make-process :name "neo-test" :buffer (current-buffer)
                         :command (list "echo" "output-line"))))
    (accept-process-output p 2)
    (buffer-string)))
"##,
        expect_test::expect![[r#""OK \"output-line\n\nProcess neo-test finished\n\"""#]],
    );
}

#[test]
fn div_proc_process_predicates_and_status() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((p (make-process :name "neo-stat" :command '("true"))))
  (accept-process-output p 2)
  (list (processp p)
        (process-name p)
        (memq (process-status p) '(run exit signal))
        (process-live-p p)))
"##,
        expect_test::expect![[r#""OK (t \"neo-stat\" (exit signal) nil)""#]],
    );
}

#[test]
fn div_proc_process_buffer_and_mark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (let ((p (make-process :name "neo-buf" :buffer (current-buffer) :command '("true"))))
    (accept-process-output p 2)
    (list (eq (process-buffer p) (current-buffer))
          (markerp (process-mark p))
          (buffer-name (process-buffer p)))))
"##,
        expect_test::expect![[r#""OK (t t \" *temp*\")""#]],
    );
}

#[test]
fn div_proc_process_command() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((p (make-process :name "neo-cmd" :command '("echo" "a" "b"))))
  (accept-process-output p 2)
  (process-command p))
"##,
        expect_test::expect![[r#""OK (\"echo\" \"a\" \"b\")""#]],
    );
}

#[test]
fn div_proc_process_list_membership() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let* ((p (make-process :name "neo-list" :command '("true")))
       (listed (memq p (process-list))))
  (accept-process-output p 2)
  (if listed t nil))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_proc_call_process_stderr_separate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (let ((stderr-buf (generate-new-buffer " *stderr*")))
    (call-process "sh" nil (list t stderr-buf) nil "-c" "echo out; echo err 1>&2")
    (let ((out (buffer-string)))
      (with-current-buffer stderr-buf
        (let ((err (buffer-string)))
          (kill-buffer stderr-buf)
          (list out err)))))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_proc_set_process_filter_captures() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let (collected)
  (let ((p (make-process :name "neo-filt" :command '("echo" "captured")
                         :connection-type 'pipe
                         :filter (lambda (proc str) (push str collected))
                         :buffer nil)))
    (accept-process-output p 2))
  collected)
"##,
        expect_test::expect![[r#""OK (\"captured\n\")""#]],
    );
}
