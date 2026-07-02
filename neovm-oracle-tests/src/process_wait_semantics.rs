//! Oracle parity tests for process wait/readiness semantics.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn compat_accept_process_output_drains_exited_process_io_matches_gnu_emacs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect =
        expect_test::expect![[r#""OK (exit ((filter \"payload\") (sentinel \"finished\n\")))""#]];
    crate::common::assert_oracle_parity_expect(
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
        expect,
    );
}

#[test]
fn read_process_output_max_limits_filter_chunks_and_snapshots_at_creation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (5 5 5 1)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"
(let ((chunks nil)
      (p nil))
  (unwind-protect
      (progn
        (let ((read-process-output-max 5)
              (process-connection-type nil))
          (setq p
                (make-process
                 :name "readmax-oracle"
                 :buffer nil
                 :connection-type 'pipe
                 :command (list "/bin/sh" "-c" "printf 0123456789abcdef")
                 :filter (lambda (_proc string)
                           (push (length string) chunks)))))
        (setq read-process-output-max 1000)
        (while (process-live-p p)
          (accept-process-output p 1))
        (while (accept-process-output p 0))
        (nreverse chunks))
    (when p
      (ignore-errors
        (delete-process p)))))
"#,
        expect,
    );
}

#[test]
fn read_process_output_carries_split_decode_sequences_between_chunks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""OK (((1 (233)) (1 (88))) ((1 (4194243))) ((1 (10)) (1 (88))))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"
(let ((process-connection-type nil))
  (let ((probe
         (lambda (script coding)
           (let ((chunks nil)
                 (p nil))
             (unwind-protect
                 (progn
                   (let ((read-process-output-max 1))
                     (setq p
                           (make-process
                            :name "readmax-utf8-oracle"
                            :buffer nil
                            :connection-type 'pipe
                            :coding coding
                            :command (list "/bin/sh" "-c" script)
                            :filter (lambda (_proc string)
                                      (push (list (length string)
                                                  (string-to-list string))
                                            chunks)))))
                   (while (process-live-p p)
                     (accept-process-output p 1))
                   (while (accept-process-output p 0))
                   (nreverse chunks))
               (when p
                 (ignore-errors
                   (delete-process p))))))))
    (list (funcall probe "printf '\\303\\251X'" 'utf-8-unix)
          (funcall probe "printf '\\303'" 'utf-8-unix)
          (funcall probe "printf '\\r\\nX'" 'utf-8-dos))))
"#,
        expect,
    );
}
