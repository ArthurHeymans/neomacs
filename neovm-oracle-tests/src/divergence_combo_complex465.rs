/// Batch 465: process-lines, make-temp-file deep, backup ops, signal-process.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx465_process_lines_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (process-lines "echo" "hello")
  (error (car e)))"##,
    );
}

#[test]
fn div_cx465_process_lines_ignore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (process-lines-ignore-status "sh" "-c" "exit 0")
  (error (car e)))"##,
    );
}

#[test]
fn div_cx465_signal_process_self() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (signal-process (emacs-pid) 0)
  (error (car e)))"##,
    );
}

#[test]
fn div_cx465_backup_file_ops_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (backup-file-name-p "/tmp/test.el~")
      (backup-file-name-p "/tmp/test.el")
      (file-name-sans-versions "/tmp/test.el~")
      (file-name-sans-versions "/tmp/test.el.~1~")))"##,
    );
}

#[test]
fn div_cx465_file_name_split_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (file-name-split "/a/b/c.d")
      (file-name-split "a/b/c")
      (file-name-split "/"))
"##,
    );
}

#[test]
fn div_cx465_file_name_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (file-name-concat "/tmp" "sub" "file.txt")
      (file-name-concat "/tmp" "file.txt"))"##,
    );
}

#[test]
fn div_cx465_make_temp_file_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "neo-cx465-mtf-" nil ".txt" "content")))
  (unwind-protect
      (list (file-exists-p f)
            (string-suffix-p ".txt" f)
            (with-temp-buffer (insert-file-contents f) (buffer-string)))
    (delete-file f)))"##,
    );
}

#[test]
fn div_cx465_file_writable_directory() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (file-writable-p "/tmp")
      (file-writable-p "/nonexistent-dir-cx465"))"##,
    );
}

#[test]
fn div_cx465_directory_files_no_dots() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((d (make-temp-file "neo-cx465-dir-" t)))
  (with-temp-file (expand-file-name ".hidden" d) (insert "x"))
  (unwind-protect
      (length (directory-files d nil "^[^.]"))
    (delete-directory d t)))"##,
    );
}

#[test]
fn div_cx465_format_time_string_fmt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((t1 (encode-time 0 30 14 16 6 2024 nil)))
  (list (format-time-string "%Y-%m-%d %H:%M:%S" t1)
        (format-time-string "%A, %d %B %Y" t1)
        (format-time-string "%s" t1)))"##,
    );
}

#[test]
fn div_cx465_decode_time_extended() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((dt (decode-time (encode-time 0 30 14 16 6 2024 nil))))
  (list (decoded-time-year dt) (decoded-time-month dt)
        (decoded-time-day dt) (decoded-time-hour dt)
        (decoded-time-minute dt) (decoded-time-second dt)))"##,
    );
}

#[test]
fn div_cx465_time_elapse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((t1 (current-time)))
  (cadr (time-elapse t1)))"##,
    );
}

#[test]
fn div_cx465_process_file_dest() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (process-file "echo" nil '(t nil) nil "hello world")
  (string-trim-right (buffer-string)))"##,
    );
}

#[test]
fn div_cx465_call_process_to_string_dest() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (call-process "echo" nil '(t nil) nil "call-process-test")
  (string-trim-right (buffer-string)))"##,
    );
}

#[test]
fn div_cx465_rename_buffer_no_unique() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((b (get-buffer-create " *cx465-rn*")))
  (rename-buffer " *cx465-rn-new*" t)
  (buffer-name))"##,
    );
}
