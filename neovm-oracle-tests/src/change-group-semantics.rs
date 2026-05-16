//! Oracle parity tests for GNU change-group and silent-modification semantics.
//!
//! These target `atomic-change-group`, `prepare-change-group`, and
//! `with-silent-modifications`.  They also pin `combine-after-change-calls`,
//! whose public macro lives in `lisp/subr.el` but whose coalescing behavior is
//! implemented by GNU `src/insdel.c` on top of buffer change state.  These
//! tests compare the observable Elisp contract rather than approximating it.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_atomic_change_group_success_keeps_changes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "base")
  (let ((result (atomic-change-group
                  (goto-char (point-max))
                  (insert "-ok")
                  (buffer-string))))
    (list result
          (buffer-string)
          (buffer-modified-p)
          (eq buffer-undo-list t)
          (consp buffer-undo-list))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_atomic_change_group_error_rolls_back_changes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "base")
  (let ((before-undo buffer-undo-list))
    (list
     (condition-case err
         (atomic-change-group
           (goto-char (point-max))
           (insert "-bad")
           (error "stop"))
       (error (list (car err) (cadr err))))
     (buffer-string)
     (equal before-undo buffer-undo-list)
     (eq buffer-undo-list t)
     (consp buffer-undo-list))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_manual_change_group_cancel_and_accept() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abc")
  (let ((cancel-handle (prepare-change-group)))
    (activate-change-group cancel-handle)
    (goto-char (point-max))
    (insert "-cancel")
    (cancel-change-group cancel-handle)
    (let ((after-cancel (buffer-string)))
      (let ((accept-handle (prepare-change-group)))
        (activate-change-group accept-handle)
        (goto-char (point-max))
        (insert "-accept")
        (accept-change-group accept-handle)
        (list after-cancel
              (buffer-string)
              (buffer-modified-p)
              (eq buffer-undo-list t)
              (consp buffer-undo-list))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_with_silent_modifications_restores_modified_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abc")
  (set-buffer-modified-p nil)
  (let ((before-hooks nil)
        (after-hooks nil))
    (add-hook 'before-change-functions
              (lambda (beg end)
                (push (list beg end) before-hooks))
              nil t)
    (add-hook 'after-change-functions
              (lambda (beg end len)
                (push (list beg end len) after-hooks))
              nil t)
    (with-silent-modifications
      (goto-char (point-max))
      (insert "X"))
    (list (buffer-string)
          (buffer-modified-p)
          before-hooks
          after-hooks
          (eq buffer-undo-list t)
          (consp buffer-undo-list))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_combine_after_change_calls_coalesces_without_before_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abcdef")
  (let ((after-log nil))
    (add-hook 'after-change-functions
              (lambda (beg end len)
                (push (list beg end len (buffer-string)) after-log))
              nil t)
    (combine-after-change-calls
      (goto-char 2)
      (insert "Y")
      (goto-char (point-max))
      (insert "Z"))
    (list (buffer-string)
          (nreverse after-log))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_combine_after_change_calls_disabled_by_before_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abcdef")
  (let ((before-log nil)
        (after-log nil))
    (add-hook 'before-change-functions
              (lambda (beg end)
                (push (list beg end (buffer-string)) before-log))
              nil t)
    (add-hook 'after-change-functions
              (lambda (beg end len)
                (push (list beg end len (buffer-string)) after-log))
              nil t)
    (combine-after-change-calls
      (goto-char 2)
      (insert "Y")
      (goto-char (point-max))
      (insert "Z"))
    (list (buffer-string)
          (nreverse before-log)
          (nreverse after-log))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_combine_after_change_calls_flushes_during_unwind() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abc")
  (let ((after-log nil))
    (add-hook 'after-change-functions
              (lambda (beg end len)
                (push (list beg end len (buffer-string)) after-log))
              nil t)
    (list
     (condition-case err
         (combine-after-change-calls
           (goto-char (point-max))
           (insert "X")
           (error "stop"))
       (error (list (car err) (cadr err))))
     (buffer-string)
     (nreverse after-log))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_nested_combine_after_change_calls_defers_until_outer_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abcdef")
  (let ((after-log nil)
        (inside-log nil))
    (add-hook 'after-change-functions
              (lambda (beg end len)
                (push (list beg end len (buffer-string)) after-log))
              nil t)
    (list
     (combine-after-change-calls
       (goto-char 2)
       (insert "X")
       (setq inside-log (list :after-first after-log))
       (let ((inner
              (combine-after-change-calls
                (goto-char (point-max))
                (insert "Y")
                (setq inside-log
                      (cons (list :inside-inner after-log) inside-log))
                :inner-value)))
         (setq inside-log
               (cons (list :after-inner inner after-log) inside-log)))
       :outer-value)
     (buffer-string)
     (nreverse inside-log)
     (nreverse after-log))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
