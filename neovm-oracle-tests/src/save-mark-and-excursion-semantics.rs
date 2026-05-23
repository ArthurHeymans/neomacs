//! Oracle parity tests for GNU `save-mark-and-excursion` semantics.
//!
//! GNU implements this macro in `lisp/simple.el`: it saves a copy of the mark
//! marker and `mark-active`, runs the body under `save-excursion`, then restores
//! both mark state and point/buffer state while firing activate/deactivate mark
//! hooks according to the restored transition.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_save_mark_and_excursion_restores_point_mark_and_active_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abcdef")
  (goto-char 2)
  (set-marker (mark-marker) 5)
  (setq mark-active nil)
  (let ((before (list (point) (mark t) mark-active))
        during)
    (setq during
          (save-mark-and-excursion
            (goto-char 4)
            (set-marker (mark-marker) 3)
            (setq mark-active t)
            (list (point) (mark t) mark-active)))
    (list before during (point) (mark t) mark-active)))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_save_mark_and_excursion_restores_marker_through_buffer_edits() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "0123456789")
  (goto-char 6)
  (set-marker (mark-marker) 8)
  (setq mark-active t)
  (let ((before (list (point) (mark t) mark-active))
        during)
    (setq during
          (save-mark-and-excursion
            (goto-char 3)
            (insert "XXX")
            (set-marker (mark-marker) 4)
            (setq mark-active nil)
            (list (point) (mark t) mark-active (buffer-string))))
    (list before during (point) (mark t) mark-active (buffer-string))))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_save_mark_and_excursion_mark_hook_transitions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((activate-log nil)
      (deactivate-log nil))
  (let ((activate-mark-hook
         (list (lambda () (push (list 'activate (mark t) mark-active) activate-log))))
        (deactivate-mark-hook
         (list (lambda () (push (list 'deactivate (mark t) mark-active) deactivate-log)))))
    (list
     (with-temp-buffer
       (insert "abcdef")
       (goto-char 2)
       (set-marker (mark-marker) 5)
       (setq mark-active nil)
       (save-mark-and-excursion
         (set-marker (mark-marker) 4)
         (setq mark-active t))
       (list (point) (mark t) mark-active
             (nreverse activate-log)
             (nreverse deactivate-log)))
     (progn
       (setq activate-log nil
             deactivate-log nil)
       (with-temp-buffer
         (insert "abcdef")
         (goto-char 2)
         (set-marker (mark-marker) 5)
         (setq mark-active t)
         (save-mark-and-excursion
           (set-marker (mark-marker) 4)
           (setq mark-active nil))
         (list (point) (mark t) mark-active
               (nreverse activate-log)
               (nreverse deactivate-log)))))))
"#;

    assert_oracle_parity(form);
}
