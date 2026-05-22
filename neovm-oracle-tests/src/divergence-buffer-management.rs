//! Divergence tests: buffer naming, get-buffer, buried buffers, buffer-list.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_buffer_list_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((bufs (buffer-list)))
  (list (> (length bufs) 0)
        (bufferp (car bufs))
        (eq (car bufs) (current-buffer))))"#,
    );
}

#[test]
fn divergence_get_buffer_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((buf (get-buffer-create " *test-gbc*")))
  (list (bufferp buf)
        (buffer-name buf)
        (buffer-live-p buf)
        (bury-buffer buf)
        (eq buf (get-buffer " *test-gbc*"))
        (kill-buffer buf)
        (buffer-live-p buf)))"#,
    );
}

#[test]
fn divergence_generate_new_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((buf1 (generate-new-buffer " *test-gnb*"))
        (buf2 (generate-new-buffer " *test-gnb*")))
  (list (buffer-name buf1)
        (buffer-name buf2)
        (not (eq buf1 buf2))
        (kill-buffer buf1)
        (kill-buffer buf2)))"#,
    );
}

#[test]
fn divergence_buffer_name_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((buf (get-buffer-create "test-name-edge")))
  (list (buffer-name buf)
        (string= (buffer-name buf) "test-name-edge")
        (get-buffer "test-name-edge")
        (eq (get-buffer "test-name-edge") buf)
        (rename-buffer "renamed-edge")
        (buffer-name buf)
        (kill-buffer buf)))"#,
    );
}

#[test]
fn divergence_buffer_file_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (null (buffer-file-name))
  (null (buffer-file-name (current-buffer)))
  (fboundp 'set-visited-file-name)
  (fboundp 'write-file))"#,
    );
}

#[test]
fn divergence_buffer_modified() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (booleanp (buffer-modified-p))
  (buffer-modified-p)
  (set-buffer-modified-p t)
  (buffer-modified-p)
  (set-buffer-modified-p nil)
  (buffer-modified-p))"#,
    );
}

#[test]
fn divergence_buffer_size_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello")
  (list (buffer-size)
        (= (buffer-size) 5)
        (point-max)
        (= (point-max) 6)
        (buffer-chars-modified-tick)))"#,
    );
}

#[test]
fn divergence_buffer_multibyte_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (buffer-live-p (current-buffer))
  (multibyte-string-p (buffer-string))
  (enable-multibyte-characters)
  (bufferp (current-buffer)))"#,
    );
}

#[test]
fn divergence_with_current_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((buf (generate-new-buffer " *wcb*")))
  (unwind-protect
      (progn
        (with-current-buffer buf
          (insert "in-other"))
        (list (buffer-string)
              (with-current-buffer buf (buffer-string))
              (eq (current-buffer) buf)))
    (kill-buffer buf)))"#,
    );
}

#[test]
fn divergence_buffer_swap_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((buf (generate-new-buffer " *swap*")))
  (unwind-protect
      (progn
        (insert "original")
        (with-current-buffer buf (insert "swapped"))
        (list (buffer-string)
              (with-current-buffer buf (buffer-string))))
    (kill-buffer buf)))"#,
    );
}
