/// Batch 457: process/shell/file I/O deep edge probes.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx457_shell_command_sync() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-trim (shell-command-to-string "echo hello"))
      (string-trim (shell-command-to-string "printf 'wo rld'")))"##,
    );
}

#[test]
fn div_cx457_shell_command_to_string_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (list (shell-command-to-string "echo test")
        (shell-command-to-string "exit 0")))"##,
    );
}

#[test]
fn div_cx457_call_process_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(call-process "true" nil nil nil)"##);
}

#[test]
fn div_cx457_call_process_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (call-process "echo" nil t nil "hello")
  (string-trim-right (buffer-string)))"##,
    );
}

#[test]
fn div_cx457_start_process_sync() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (start-process "cx457" (current-buffer) "echo" "test")
  (accept-process-output (get-process "cx457") 2)
  (string-trim-right (buffer-string)))"##,
    );
}

#[test]
fn div_cx457_delete_directory_recursive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((d (make-temp-file "neo-cx457-dir-" t)))
  (with-temp-file (expand-file-name "sub" d) (insert "x"))
  (unwind-protect
      (progn (delete-directory d t) t)
    (ignore-errors (delete-directory d t))))"##,
    );
}

#[test]
fn div_cx457_copy_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((src (make-temp-file "neo-cx457-src-"))
      (dst (make-temp-file "neo-cx457-dst-")))
  (with-temp-file src (insert "content"))
  (unwind-protect
      (progn (copy-file src dst t)
             (with-temp-buffer (insert-file-contents dst) (buffer-string)))
    (delete-file src)
    (ignore-errors (delete-file dst))))"##,
    );
}

#[test]
fn div_cx457_rename_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((src (make-temp-file "neo-cx457-rn-")))
  (unwind-protect
      (progn (rename-file src "/tmp/neo-cx457-renamed" t)
             (file-exists-p "/tmp/neo-cx457-renamed"))
    (ignore-errors (delete-file "/tmp/neo-cx457-renamed"))))"##,
    );
}

#[test]
fn div_cx457_add_name_to_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "neo-cx457-an-")))
  (unwind-protect
      (progn (add-name-to-file f "/tmp/neo-cx457-hardlink" t)
             (file-exists-p "/tmp/neo-cx457-hardlink"))
    (delete-file f)
    (ignore-errors (delete-file "/tmp/neo-cx457-hardlink"))))"##,
    );
}

#[test]
fn div_cx457_make_directory_change_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((d (expand-file-name "neo-cx457-mkdir" temporary-file-directory)))
  (make-directory d t)
  (unwind-protect
      (file-directory-p d)
    (delete-directory d t)))"##,
    );
}

#[test]
fn div_cx457_insert_file_contents_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "neo-cx457-ifc-")))
  (with-temp-file f (insert "new data"))
  (unwind-protect
      (with-temp-buffer
        (insert "old content")
        (insert-file-contents f nil nil nil t)
        (buffer-string))
    (delete-file f)))"##,
    );
}

#[test]
fn div_cx457_write_region_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "neo-cx457-wr-")))
  (write-region "test data" nil f nil 0)
  (prog1 (with-temp-buffer
           (insert-file-contents f)
           (string-trim-right (buffer-string)))
    (delete-file f)))"##,
    );
}

#[test]
fn div_cx457_make_temp_file_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "neo-cx457-mtf-")))
  (unwind-protect
      (file-exists-p f)
    (delete-file f)))"##,
    );
}

#[test]
fn div_cx457_file_selinux_context() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "neo-cx457-selinux-")))
  (unwind-protect
      (condition-case e (file-selinux-context f) (error (car e)))
    (delete-file f)))"##,
    );
}

#[test]
fn div_cx457_path_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(list path-separator directory-sep-char)"##);
}
