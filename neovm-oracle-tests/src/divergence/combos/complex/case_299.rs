//! Complex combo batch 299 — `process` deep: `set-process-window-size`,
//! `process-thread`, `process-running-p`, `network-interface-info`,
//! `serial-process-configure`, `make-serial-process` availability.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx299_process_window_size_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(list (fboundp 'set-process-window-size)
      (fboundp 'window-size)
      (boundp 'process-adaptive-read-buffering))
"##,
        expect,
    )
}

#[test]
fn div_cx299_process_thread_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t (:thread nil))""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let ((p (make-process :name "neo-cx299-thread" :command '("echo" "test"))))
      (list (fboundp 'process-thread)
            (when (fboundp 'process-thread)
              (process-thread p))))
  (error (list :errored (car e))))
"##,
        expect,
    )
}

#[test]
fn div_cx299_network_interface_info_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t t :wrong-type)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let ((ifaces (network-interface-list)))
      (list
       (or (null ifaces) (consp ifaces))
       ;; Entries are (NAME . ADDRESS); the info query takes the NAME
       ;; string. Project to shape booleans -- the live interface set,
       ;; ordering, and addresses are machine state and must not leak
       ;; into the expect.
       (and ifaces
            (let ((info (network-interface-info (caar ifaces))))
              (or (null info) (consp info))))
       ;; GNU signals wrong-type-argument for a non-string; lock the
       ;; signal SYMBOL only (its data embeds the volatile entry).
       (and ifaces
            (condition-case err
                (progn (network-interface-info (car ifaces)) :no-error)
              (wrong-type-argument :wrong-type)
              (error (list :other (car err)))))))
  (error (list :errored (car e))))
"##,
        expect,
    )
}

#[test]
fn div_cx299_serial_process_configure_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(list (fboundp 'make-serial-process)
      (fboundp 'serial-process-configure))
"##,
        expect,
    )
}

#[test]
fn div_cx299_process_adaptive_read_buffering() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(list (boundp 'process-adaptive-read-buffering)
      (boundp 'read-process-output-max))
"##,
        expect,
    )
}

#[test]
fn div_cx299_process_environment_override_with_multiple_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"Process neo-cx299-env finished\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let* ((buf (get-buffer-create " *neo-cx299-env*"))
           (p (make-process :name "neo-cx299-env"
                            :command '("sh" "-c" "echo $NEO_A $NEO_B $NEO_C")
                            :buffer buf
                            :environment (append '("NEO_A=alpha" "NEO_B=beta" "NEO_C=gamma")
                                                 process-environment))))
      (accept-process-output p 2)
      (sit-for 0.05)
      (let ((content (string-trim (with-current-buffer buf (buffer-string)))))
        (kill-buffer buf)
        content))
  (error (list :errored (car e))))
"##,
        expect,
    )
}

#[test]
fn div_cx299_make_network_process_with_filter_and_sentinel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (:errored file-error)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let ((filter-data nil)
          (sentinel-events nil))
      (let ((p (make-network-process :name "neo-cx299-net"
                                      :host "127.0.0.1"
                                      :service 80
                                      :family 'ipv4
                                      :filter (lambda (proc data) (push data filter-data))
                                      :sentinel (lambda (proc ev) (push ev sentinel-events)))))
        (prog1 (list (processp p)
                     (process-name p)
                     (process-contact p :local))
          (delete-process p))))
  (error (list :errored (car e))))
"##,
        expect,
    )
}

#[test]
fn div_cx299_process_output_with_coding_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (\"世界\nProcess neo-cx299-cs finished\n\" 33)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(let* ((buf (get-buffer-create " *neo-cx299-cs*"))
       (p (make-process :name "neo-cx299-cs"
                        :command '("sh" "-c" "printf '\\xe4\\xb8\\x96\\xe7\\x95\\x8c'")
                        :buffer buf)))
  (set-process-coding-system p 'utf-8-unix 'utf-8-unix)
  (accept-process-output p 1)
  (sit-for 0.05)
  (let ((content (with-current-buffer buf (buffer-string))))
    (kill-buffer buf)
    (list content (length content))))
"##,
        expect,
    )
}

#[test]
fn div_cx299_process_kill_after_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect =
        expect_test::expect![[r#""OK ((run open listen connect stop) nil t nil signal)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((p (make-process :name "neo-cx299-kill"
                        :command '("sh" "-c" "sleep 5"))))
  (list (process-live-p p)
        (delete-process p)
        (sit-for 0.05)
        (process-live-p p)
        (process-status p)))
"##,
        expect,
    )
}

#[test]
fn div_cx299_process_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (wrong-number-of-arguments widen 1)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(let* ((buf (get-buffer-create " *neo-cx299-mega*")))
  (with-current-buffer buf
    (buffer-enable-undo)
    (insert "Process network mega test buffer content")
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)))
  (let ((p (make-process :name "neo-cx299-mega-p"
                         :command '("sh" "-c" "printf 'PROCMEGA'")
                         :buffer buf)))
    (process-put p 'neo-cx299-tag :mega)
    (set-process-coding-system p 'utf-8-unix 'utf-8-unix)
    (accept-process-output p 1)
    (sit-for 0.05))
  (let ((content (with-current-buffer buf (buffer-string))))
    (with-current-buffer buf
      (widen())
      (let ((state (list content (length content)
                         (length (overlays-in 1 20))
                         (text-properties-at 1))))
        (undo)
        (kill-buffer buf)
        (list state (buffer-string)))))))
"##,
        expect,
    )
}
