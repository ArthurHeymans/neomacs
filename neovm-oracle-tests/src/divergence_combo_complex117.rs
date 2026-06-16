//! Complex combo batch 117 — `process` filters with chunk boundaries,
//! coding systems on partial UTF-8 chunks, sentinel ordering, and
//! stdin/stdout round-trips.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx117_process_filter_chunked_data_capture() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((collected nil)
       (buf (get-buffer-create " *neo-cx117-pf*"))
       (p (make-process :name "neo-cx117-pf"
                        :command '("sh" "-c" "for i in 1 2 3; do printf 'line%d\\n' $i; done")
                        :buffer buf
                        :filter (lambda (proc data) (push data collected)))))
  (accept-process-output p 2)
  (sit-for 0.05)
  (let ((all-data (apply #'concat (nreverse collected))))
    (kill-buffer buf)
    (list (length collected)
          (length all-data)
          (string-trim all-data))))
"##,
    );
}

#[test]
fn div_cx117_process_filter_with_coding_chunk_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((collected nil)
       (buf (get-buffer-create " *neo-cx117-cs*"))
       (p (make-process :name "neo-cx117-cs"
                        :command '("sh" "-c" "printf 'café \\xe4\\xb8\\x96\\xe7\\x95\\x8c'")
                        :buffer buf
                        :coding 'utf-8-unix
                        :filter (lambda (proc data) (push data collected))))
  (accept-process-output p 2)
  (sit-for 0.05)
  (let ((content (with-current-buffer buf (buffer-string))))
    (kill-buffer buf)
    (list content (length content))))
"##,
    );
}

#[test]
fn div_cx117_process_sentinel_runs_on_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((events nil))
  (let ((p (make-process :name "neo-cx117-sent"
                         :command '("sh" "-c" "exit 7")
                         :sentinel (lambda (proc ev) (push ev events)))))
    (accept-process-output p 2)
    (sit-for 0.05)
    (list (nreverse events)
          (process-exit-status p)
          (process-status p))))
"##,
    );
}

#[test]
fn div_cx117_process_send_string_to_stdin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((buf (get-buffer-create " *neo-cx117-stdin*"))
       (p (make-process :name "neo-cx117-stdin"
                        :command '("cat")
                        :buffer buf
                        :connection-type 'pipe)))
  (process-send-string p "alpha beta gamma\n")
  (process-send-eof p)
  (accept-process-output p 2)
  (sit-for 0.05)
  (let ((content (with-current-buffer buf (buffer-string))))
    (kill-buffer buf)
    content))
"##,
    );
}

#[test]
fn div_cx117_process_connection_type_pipe_vs_pty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let* ((buf-pipe (get-buffer-create " *neo-cx117-pipe*"))
           (p-pipe (make-process :name "neo-cx117-pipe"
                                 :command '("echo" "via-pipe")
                                 :buffer buf-pipe
                                 :connection-type 'pipe)))
      (accept-process-output p-pipe 2)
      (let ((pipe-content (with-current-buffer buf-pipe (buffer-string))))
        (kill-buffer buf-pipe)
        pipe-content))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx117_make_process_with_environment_override() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let* ((buf (get-buffer-create " *neo-cx117-env*"))
           (p (make-process :name "neo-cx117-env"
                            :command '("sh" "-c" "echo $NEO_CX117")
                            :buffer buf
                            :environment (cons "NEO_CX117=hello-env" process-environment))))
      (accept-process-output p 2)
      (sit-for 0.05)
      (let ((content (string-trim (with-current-buffer buf (buffer-string)))))
        (kill-buffer buf)
        content))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx117_process_filter_no_buffer_no_filter_appends_nowhere() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((collected nil))
  (let ((p (make-process :name "neo-cx117-nobuf"
                         :command '("sh" "-c" "printf 'no buffer here'")
                         :buffer nil
                         :filter (lambda (proc data) (push data collected)))))
    (accept-process-output p 2)
    (sit-for 0.05)
    (apply #'concat (nreverse collected))))
"##,
    );
}

#[test]
fn div_cx117_process_stderr_capture() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let* ((stderr-buf (get-buffer-create " *neo-cx117-err*"))
           (p (make-process :name "neo-cx117-err"
                            :command '("sh" "-c" "echo 'to stderr' >&2; echo 'to stdout'")
                            :stderr stderr-buf)))
      (accept-process-output p 2)
      (sit-for 0.05)
      (let ((stderr-content (string-trim (with-current-buffer stderr-buf (buffer-string)))))
        (kill-buffer stderr-buf)
        stderr-content))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx117_process_query_before_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((buf (get-buffer-create " *neo-cx117-q*"))
       (p (make-process :name "neo-cx117-q"
                        :command '("sh" "-c" "echo start; sleep 0.1; echo end")
                        :buffer buf)))
  (list (process-live-p p)
        (process-list)
        (memq p (process-list))
        (process-name p)
        (process-command p)
        (accept-process-output p 1)
        (process-exit-status p)))
"##,
    );
}

#[test]
fn div_cx117_make_process_file_name_temp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let* ((tmp (make-temp-name "/tmp/neo-cx117-tmp-"))
           (p (make-process :name "neo-cx117-tmpfile"
                            :command '("sh" "-c" (concat "echo $$$$ > " tmp))
                            )))
      (accept-process-output p 1)
      (sit-for 0.05)
      (let ((created (file-exists-p tmp))
            (content (when (file-exists-p tmp)
                       (string-trim (with-temp-buffer
                                      (insert-file-contents tmp)
                                      (buffer-string))))))
        (when (file-exists-p tmp) (delete-file tmp))
        (list created content)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx117_process_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((buf (get-buffer-create " *neo-cx117-mega*")))
  (with-current-buffer buf
    (buffer-enable-undo)
    (insert "Initial buffer content")
    (put-text-property 1 7 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)))
  (let ((p (make-process :name "neo-cx117-mega-p"
                         :command '("sh" "-c" "printf 'SUB-OUT'")
                         :buffer buf)))
    (accept-process-output p 1)
    (sit-for 0.05))
  (let ((content (with-current-buffer buf (buffer-string))))
    (with-current-buffer buf
      (widen)
      (let ((state (list content (length content)
                         (length (overlays-in 1 20))
                         (text-properties-at 1))))
        (undo)
        (kill-buffer buf)
        (list state (buffer-string))))))
"##,
    );
}
