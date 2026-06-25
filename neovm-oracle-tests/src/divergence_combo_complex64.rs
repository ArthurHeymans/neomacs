//! Complex combo batch 64 — process / pipe / sentinel / timer interplay
//! with coding systems, including multi-stage pipe filtering and timer
//! reentrancy under buffers with text properties.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx64_process_filter_chunked_utf8_decoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((buf (get-buffer-create " *neo-cx64-pf*"))
       (chunks nil))
  (let ((p (make-process :name "neo-cx64-pf"
                         :command '("sh" "-c" "printf 'hello \\xe4\\xb8\\x96\\xe7\\x95\\x8c'")
                         :buffer buf
                         :filter (lambda (proc data) (push data chunks))
                         :coding 'utf-8-unix)))
    (accept-process-output p 2)
    (sit-for 0.05)
    (let ((output (with-current-buffer buf (buffer-string))))
      (kill-buffer buf)
      (list output (length output) (length chunks)
            (apply #'+ (mapcar #'length chunks))))))
"##,
    );
}

#[test]
fn div_cx64_process_sentinel_exit_and_signal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((events nil))
  (let ((p1 (make-process :name "neo-cx64-exit"
                          :command '("sh" "-c" "exit 7")
                          :sentinel (lambda (proc ev) (push (cons :exit ev) events))))
        (p2 (make-process :name "neo-cx64-sig"
                          :command '("sh" "-c" "kill -TERM $$")
                          :sentinel (lambda (proc ev) (push (cons :sig ev) events)))))
    (accept-process-output p1 2)
    (accept-process-output p2 2)
    (sit-for 0.05)
    (list (nreverse events)
          (process-exit-status p1)
          (process-status p1)
          (process-exit-status p2)
          (process-status p2))))
"##,
    );
}

#[test]
fn div_cx64_make_pipe_pair_one_way_communication() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let* ((tmp (make-temp-name "/tmp/neo-cx64-pipe-"))
           (p (make-pipe-process :name "neo-cx64-pipe"
                                 :buffer (generate-new-buffer " *neo-cx64-pipe*")
                                 :coding 'utf-8-unix)))
      (process-send-string p "first\n")
      (process-send-string p "second\n")
      (process-send-eof p)
      (accept-process-output p 1)
      (sit-for 0.05)
      (let ((content (with-current-buffer (process-buffer p) (buffer-string))))
        (delete-process p)
        (kill-buffer (process-buffer p))
        content))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx64_timer_repeated_invocation_in_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fire-seq)
  (let ((timer (run-with-timer 0 0.001 (lambda () (push (length fire-seq) fire-seq)))))
    (sit-for 0.02)
    (cancel-timer timer)
    (let ((first (nreverse fire-seq)))
      (setq fire-seq nil)
      (let ((once (run-with-timer 0 nil (lambda () (push :once fire-seq)))))
        (sit-for 0.02)
        (list (length first) (car first) (car (last first))
              (nreverse fire-seq))))))
"##,
    );
}

#[test]
fn div_cx64_idle_timer_does_not_fire_during_active_processing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fired)
  (let ((idle (run-with-idle-timer 0.05 nil (lambda () (push :idle fired)))))
    (sit-for 0.01)            ; too short for idle to trigger
    (let ((before idle-fired)
          (first fired))
      (sit-for 0.1)
      (cancel-timer idle)
      (list first (nreverse fired)))))
"##,
    );
}

#[test]
fn div_cx64_process_environment_inheritance_and_override() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((before (getenv "NEO_CX64"))
       (env1 (let ((process-environment (cons "NEO_CX64=value1" process-environment)))
               (string-trim (shell-command-to-string "echo $NEO_CX64"))))
       (env2 (let ((process-environment (cons "NEO_CX64=value2" process-environment)))
               (string-trim (shell-command-to-string "printf %s $NEO_CX64")))))
  (list before env1 env2 (getenv "NEO_CX64")))
"##,
    );
}

#[test]
fn div_cx64_process_buffer_undo_recording() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((buf (get-buffer-create " *neo-cx64-pb*")))
  (with-current-buffer buf
    (buffer-enable-undo))
  (let ((p (make-process :name "neo-cx64-pb"
                         :command '("printf" "%s" "ABCDEF")
                         :buffer buf)))
    (accept-process-output p 1)
    (sit-for 0.05)
  (with-current-buffer buf
    (let ((content (buffer-string))
          (undo-list-len (if (boundp 'buffer-undo-list) (length buffer-undo-list) :none)))
      (undo)
      (let ((after-undo (buffer-string)))
        (prog1 (list content undo-list-len after-undo)
          (kill-buffer buf))))))
"##,
    );
}

#[test]
fn div_cx64_two_processes_interleaved_buffer_appends() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((buf (get-buffer-create " *neo-cx64-i*")))
  ;; Both processes append to the same buffer; the OS interleaving of their two
  ;; outputs is nondeterministic, as is the "Process ... finished" sentinel
  ;; noise.  Drain p1 fully (no-op its incidental sentinel) before starting and
  ;; draining p2, so the appends land in a fixed order on both engines while
  ;; still exercising two processes writing to one shared buffer.
  (let ((p1 (make-process :name "neo-cx64-i1" :command '("printf" "%s" "AAA") :buffer buf)))
    (set-process-sentinel p1 #'ignore)
    (while (process-live-p p1) (accept-process-output p1 1))
    (while (accept-process-output p1 0))
    (let ((p2 (make-process :name "neo-cx64-i2" :command '("printf" "%s" "BBB") :buffer buf)))
      (set-process-sentinel p2 #'ignore)
      (while (process-live-p p2) (accept-process-output p2 1))
      (while (accept-process-output p2 0))))
  (prog1 (with-current-buffer buf (buffer-string))
    (kill-buffer buf)))
"##,
    );
}

#[test]
fn div_cx64_set_process_coding_system_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((buf (get-buffer-create " *neo-cx64-cs*")))
  (let ((p (make-process :name "neo-cx64-cs"
                         :command '("sh" "-c" "printf '\\xe4\\xb8\\x96\\xe7\\x95\\x8c'")
                         :buffer buf)))
    (set-process-coding-system p 'utf-8-unix 'utf-8-unix)
    (accept-process-output p 1)
    (sit-for 0.05))
  (let ((decode-system (process-coding-system (get-process "neo-cx64-cs"))))
    (prog1 (list (with-current-buffer buf (buffer-string))
                 decode-system)
      (kill-buffer buf))))
"##,
    );
}

#[test]
fn div_cx64_timer_process_buffer_undo_textprop_narrow_env_exitcode_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((timer-fired nil))
  (run-with-timer 0 nil (lambda () (setq timer-fired :t)))
  (let ((env-val (let ((process-environment (cons "NEO_CX64=v" process-environment)))
                   (string-trim (shell-command-to-string "echo $NEO_CX64"))))
        (exit-code (let ((p (make-process :name "neo-cx64-ec"
                                           :command '("sh" "-c" "exit 9"))))
                     (accept-process-output p 2)
                     (process-exit-status p))))
    (sit-for 0.01)
    (let ((buf (get-buffer-create " *neo-cx64-mega*")))
      (with-current-buffer buf
        (buffer-enable-undo)
        (insert "ABCDEF 世界 café 0123")
        (put-text-property 1 5 'face 'bold)
        (put-text-property 8 12 'display "XX")
        (let ((m (set-marker (make-marker) 8))
              (ov (make-overlay 3 9)))
          (overlay-put ov 'face 'italic)
          (overlay-put ov 'evaporate t)
          (narrow-to-region 2 15)))
      (let ((p (make-process :name "neo-cx64-mega-p"
                             :command '("sh" "-c" "printf 'APPENDED'")
                             :buffer buf)))
        (accept-process-output p 1)
        (sit-for 0.05))
      (let ((content (with-current-buffer buf (buffer-string))))
        (with-current-buffer buf
          (widen)
          (let ((state (list timer-fired env-val exit-code
                             content (length content)
                             (marker-position (set-marker (make-marker) 8))
                             (length (overlays-in 1 20))
                             (text-properties-at 1))))
            (undo)
            (list state (buffer-string)
                  (text-properties-at 1)
                  (length (overlays-in 1 20))))))
      (kill-buffer buf))))
"##,
    );
}
