//! Oracle parity tests for GNU core undo API semantics.
//!
//! GNU implements `undo-boundary` in `src/undo.c`, `buffer-enable-undo` in
//! `src/buffer.c`, and `buffer-disable-undo`/`primitive-undo` in
//! `lisp/simple.el`.  These tests pin observable low-level behavior without
//! depending on interactive command-loop undo state.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_undo_boundary_idempotence_and_disabled_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (let ((results nil))
    (setq buffer-undo-list '(a))
    (undo-boundary)
    (push buffer-undo-list results)
    (undo-boundary)
    (push buffer-undo-list results)
    (setq buffer-undo-list t)
    (undo-boundary)
    (push buffer-undo-list results)
    (nreverse results)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_buffer_enable_disable_undo_current_and_named_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((buf (generate-new-buffer "neovm--undo-core")))
  (unwind-protect
      (list
       (with-current-buffer buf
         (buffer-disable-undo)
         (list buffer-undo-list
               (buffer-enable-undo)
               buffer-undo-list
               (buffer-disable-undo)
               buffer-undo-list))
       (list (buffer-enable-undo "neovm--undo-core")
             (with-current-buffer buf buffer-undo-list))
       (condition-case err
           (buffer-enable-undo "neovm--missing-undo-core")
         (error (list (car err) (cadr err)))))
    (kill-buffer buf)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_primitive_undo_manual_insert_and_delete_records() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abcd")
  (let ((buffer-undo-list nil))
    (list
     (primitive-undo 1 '((2 . 4) nil))
     (buffer-string)
     (point)
     buffer-undo-list
     (primitive-undo 1 '(("XY" . 2) nil))
     (buffer-string)
     (point)
     buffer-undo-list)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_let_bound_buffer_undo_list_on_modified_buffer_skips_first_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abcd")
  (let ((buffer-undo-list nil))
    (delete-region 2 4)
    (list (buffer-string)
          buffer-undo-list
          (buffer-modified-tick)
          (buffer-chars-modified-tick)
          (buffer-modified-p))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
