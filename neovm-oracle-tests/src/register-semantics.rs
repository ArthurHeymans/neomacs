//! Oracle parity tests for GNU `register.el` register semantics.
//!
//! These tests cover programmatic register storage, text collection,
//! insertion, numeric updates, marker swap-out, and descriptions.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_register_set_get_numbers_and_text_collection() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'register)
  (let ((register-alist nil)
        (register-separator nil))
    (with-temp-buffer
      (insert "alpha beta gamma")
      (set-register ?s "::")
      (setq register-separator ?s)
      (copy-to-register ?a 1 6 nil nil)
      (append-to-register ?a 7 11 nil)
      (prepend-to-register ?a 12 17 nil)
      (number-to-register 10 ?n)
      (increment-register 5 ?n)
      (list
       (get-register ?a)
       (get-register ?n)
       register-alist
       deactivate-mark))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_register_copy_delete_region_and_number_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'register)
  (let ((register-alist nil))
    (with-temp-buffer
      (insert "  -42 tail\nremove keep")
      (goto-char (point-min))
      (number-to-register nil ?n)
      (let ((point-after-number (point)))
        (search-forward "remove")
        (copy-to-register ?d (match-beginning 0) (match-end 0) t nil)
        (list
         (get-register ?n)
         point-after-number
         (get-register ?d)
         (buffer-string)
         deactivate-mark)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_register_insert_string_number_marker_and_rectangle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'register)
  (let ((register-alist nil))
    (list
     (with-temp-buffer
       (set-register ?s "TEXT")
       (insert "ab")
       (goto-char 2)
       (insert-register ?s nil)
       (list (buffer-string) (point) (mark)))
     (with-temp-buffer
       (set-register ?n 123)
       (insert-register ?n t)
       (list (buffer-string) (point) (mark)))
     (with-temp-buffer
       (let ((m (copy-marker 1)))
         (set-register ?m m)
         (insert-register ?m t)
         (list (buffer-string) (point) (mark))))
     (with-temp-buffer
       (insert "aa\nbb\n")
       (goto-char (point-min))
       (move-to-column 1)
       (set-register ?r '("X" "YZ"))
       (insert-register ?r t)
       (list (buffer-string) (point) (mark))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_register_point_jump_swap_out_and_descriptions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'register)
  (let ((register-alist nil))
    (list
     (with-temp-buffer
       (insert "one\ntwo\nthree")
       (goto-char (point-min))
       (forward-line 1)
       (point-to-register ?p)
       (goto-char (point-max))
       (jump-to-register ?p)
       (list (point) (marker-position (get-register ?p)) (buffer-name (marker-buffer (get-register ?p)))))
     (let ((buf (generate-new-buffer " *register-oracle-file*")))
       (unwind-protect
           (with-current-buffer buf
             (setq buffer-file-name "/tmp/neomacs-register-oracle.txt")
             (insert "file-backed buffer")
             (goto-char 7)
             (point-to-register ?f)
             (kill-buffer buf)
             (get-register ?f))
         (when (buffer-live-p buf)
           (kill-buffer buf))))
     (progn
       (set-register ?s "hello\nworld")
       (set-register ?n 42)
       (set-register ?r '("aa" "bb"))
       (list
        (register-describe-oneline ?s)
        (register-describe-oneline ?n)
        (register-describe-oneline ?r))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
