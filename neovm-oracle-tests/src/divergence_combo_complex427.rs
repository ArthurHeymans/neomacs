//! Complex combo batch 427 — 18 probes into process/OS interface stubs
//! and edge cases: process-datagram-address, process-tty-name, process-mark,
//! process-filter/sentinel deep, set-process-plist, make-pipe-process,
//! open-network-stream-nowait, list-processes, process-list, process-live-p
//! on dead process, signal-process, get-internal-run-time, current-undo-list,
//! buffer-undo-list deep, with-undo-amalgamate, and window-prev-buffers.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// process-datagram-address / process-tty-name: process connection details.
#[test]
fn div_cx427_process_connection_details() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx427-pd"
                          :command '("echo" "test")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (prog1 (list (condition-case e (process-tty-name proc) (error (car e)))
               (condition-case e (process-datagram-address proc) (error (car e))))
    (delete-process proc)))
"##,
    );
}

/// process-mark / process-filter / process-sentinel query.
#[test]
fn div_cx427_process_mark_filter_sentinel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx427-pm"
                          :command '("echo" "test")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (prog1 (list (markerp (process-mark proc))
               (functionp (process-filter proc))
               (functionp (process-sentinel proc)))
    (delete-process proc)))
"##,
    );
}

/// set-process-plist / process-plist deep.
#[test]
fn div_cx427_set_process_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx427-pp"
                          :command '("echo" "done")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (set-process-plist proc '(key1 val1 key2 val2))
  (prog1 (process-plist proc)
    (delete-process proc)))
"##,
    );
}

/// make-pipe-process: creating a pipe (may be stubbed).
#[test]
fn div_cx427_make_pipe_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (make-pipe-process :name "neo-cx427-pipe")
  (error (car e)))
"##,
    );
}

/// process-list: enumerating all (non-deleted) processes.
#[test]
fn div_cx427_process_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx427-pl"
                          :command '("echo" "test")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (let ((in-list (memq proc (process-list))))
    (delete-process proc)
    in-list)
"##,
    );
}

/// process-live-p on dead process / after delete-process.
#[test]
fn div_cx427_process_live_dead() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx427-dp"
                          :command '("echo" "done")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (let ((before (process-live-p proc)))
    (delete-process proc)
    (list before (process-live-p proc))))
"##,
    );
}

/// signal-process: sending signals (may be stubbed).
#[test]
fn div_cx427_signal_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (signal-process (emacs-pid) 0)
  (error (car e)))
"##,
    );
}

/// get-internal-run-time: internal timing (basic availability check).
#[test]
fn div_cx427_get_internal_run_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((t1 (get-internal-run-time)))
  (and (listp t1) (= (length t1) 4)))
"##,
    );
}

/// current-undo-list / buffer-undo-list deeper.
#[test]
fn div_cx427_current_undo_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "abc")
  (delete-region 2 3)
  (list (consp buffer-undo-list)
        (listp buffer-undo-list)))
"##,
    );
}

/// with-undo-amalgamate / undo-boundary amalgamation.
#[test]
fn div_cx427_with_undo_amalgamate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "hello")
  (undo-boundary)
  (with-undo-amalgamate
    (insert " world")
    (insert "!"))
  (list (length (delq nil buffer-undo-list))
        (> (length (delq nil buffer-undo-list)) 0)))
"##,
    );
}

/// window-prev-buffers / window-next-buffers.
#[test]
fn div_cx427_window_prev_next_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "testing")
  (let ((w (selected-window)))
    (set-window-buffer w (current-buffer))
    (list (listp (window-prev-buffers w))
          (listp (window-next-buffers w)))))
"##,
    );
}

/// buffer-match-p / match-buffer: buffer matching predicates.
#[test]
fn div_cx427_buffer_match_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (buffer-match-p "\\*.*\\*" (current-buffer)) (error (car e)))
      (condition-case e (buffer-match-p "nonexistent-pattern" (current-buffer)) (error (car e))))
"##,
    );
}

/// unicode-canonicalize / unicode-decompose: Unicode normalization.
#[test]
fn div_cx427_unicode_normalize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (ucs-normalize-NFD-string "café")
      (ucs-normalize-NFC-string "cafe\u0301")
      (ucs-normalize-NFKD-string "①"))
"##,
    );
}

/// set-process-coding-system with nil parameters.
#[test]
fn div_cx427_set_process_coding_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx427-scn"
                          :command '("echo" "test")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (set-process-coding-system proc nil nil)
  (prog1 (process-coding-system proc)
    (delete-process proc)))
"##,
    );
}

/// process-connection-type as pty vs pipe.
#[test]
fn div_cx427_process_connection_pty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx427-pct"
                          :command '("echo" "test")
                          :connection-type 'pty :buffer nil)))
  (accept-process-output proc 2)
  (prog1 (process-status proc)
    (delete-process proc)))
"##,
    );
}
