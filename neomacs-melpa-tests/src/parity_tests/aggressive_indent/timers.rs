use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_while_no_input_returns_body_value_or_pending_input_marker() {
    let elisp_form = r##"(let (events)
         (list
          (cl-letf (((symbol-function 'input-pending-p)
                     (lambda ()
                       (push 'checked-normal events)
                       nil)))
            (aggressive-indent--while-no-input
              (push 'body-ran events)
              '(completed value)))
          (cl-letf (((symbol-function 'input-pending-p)
                     (lambda ()
                       (push 'checked-pending events)
                       'pending)))
            (aggressive-indent--while-no-input
              (push 'body-must-not-run events)
              'unexpected))
          (nreverse events)
          quit-flag))"##;
    let expect =
        expect!["OK ((completed value) pending (checked-normal body-ran checked-pending) nil)"];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_cancel_timer_only_cancels_timer_objects_and_clears_them() {
    let elisp_form = r##"(let (events)
         (list
          (with-temp-buffer
            (setq aggressive-indent--idle-timer
                  (timer-create))
            (cl-letf (((symbol-function 'cancel-timer)
                       (lambda (timer)
                         (push
                          (list 'cancelled
                                (timerp timer))
                          events)
                         'done)))
              (list
               (aggressive-indent--maybe-cancel-timer)
               aggressive-indent--idle-timer)))
          (with-temp-buffer
            (setq aggressive-indent--idle-timer
                  'not-a-timer)
            (cl-letf (((symbol-function 'cancel-timer)
                       (lambda (&rest arguments)
                         (push
                          (cons 'unexpected arguments)
                          events))))
              (list
               (aggressive-indent--maybe-cancel-timer)
               aggressive-indent--idle-timer)))
          (nreverse events)))"##;
    let expect = expect!["OK ((nil nil) (nil not-a-timer) ((cancelled t)))"];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_timer_callback_processes_live_buffer_then_cancels_in_its_context() {
    let elisp_form = r##"(let ((target
                (generate-new-buffer
                 " *aggressive-live-timer*"))
               events)
         (unwind-protect
             (progn
               (with-current-buffer target
                 (setq aggressive-indent-mode t
                       aggressive-indent--changed-list
                       '((2 8))
                       aggressive-indent--idle-timer
                       (timer-create)))
               (cl-letf (((symbol-function
                           'aggressive-indent--process-changed-list-and-indent)
                          (lambda ()
                            (push
                             (list
                              'processed
                              (buffer-name)
                              aggressive-indent--changed-list)
                             events)
                            (setq
                             aggressive-indent--changed-list
                             nil)))
                         ((symbol-function 'cancel-timer)
                          (lambda (timer)
                            (push
                             (list
                              'cancelled
                              (buffer-name)
                              (timerp timer))
                             events)))
                         ((symbol-function 'input-pending-p)
                          (lambda () nil)))
                 (list
                  (aggressive-indent--indent-if-changed
                   target)
                  (with-current-buffer target
                    (list
                     aggressive-indent--changed-list
                     aggressive-indent--idle-timer))
                  (nreverse events))))
           (when (buffer-live-p target)
             (kill-buffer target))))"##;
    let expect = expect![[
        r#"OK (nil (nil nil) ((processed " *aggressive-live-timer*" ((2 8))) (cancelled " *aggressive-live-timer*" t)))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_timer_callback_skips_disabled_or_unchanged_live_buffers() {
    let elisp_form = r##"(let ((target
                (generate-new-buffer
                 " *aggressive-skip-timer*"))
               events)
         (unwind-protect
             (cl-letf (((symbol-function
                         'aggressive-indent--process-changed-list-and-indent)
                        (lambda ()
                          (push 'unexpected-process events)))
                       ((symbol-function
                         'aggressive-indent--maybe-cancel-timer)
                        (lambda ()
                          (push 'unexpected-cancel events))))
               (with-current-buffer target
                 (setq aggressive-indent-mode nil
                       aggressive-indent--changed-list
                       '((1 2))))
               (let ((disabled
                      (aggressive-indent--indent-if-changed
                       target)))
                 (with-current-buffer target
                   (setq aggressive-indent-mode t
                         aggressive-indent--changed-list
                         nil))
                 (list
                  disabled
                  (aggressive-indent--indent-if-changed
                   target)
                  (nreverse events))))
           (when (buffer-live-p target)
             (kill-buffer target))))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_scheduled_callback_runs_real_post_edit_indentation_on_demand() {
    let elisp_form = r##"(let (scheduled)
         (cl-letf (((symbol-function 'run-with-idle-timer)
                    (lambda (&rest arguments)
                      (setq scheduled arguments)
                      'deterministic-timer)))
           (with-temp-buffer
             (emacs-lisp-mode)
             (aggressive-indent-mode 1)
             (insert
              "(defun timer-workflow ()\n"
              "(let ((value 3))\n"
              "(message \"%s\" value)))\n")
             (let ((before (buffer-string))
                   (changes
                    (copy-tree
                     aggressive-indent--changed-list)))
               (apply
                (nth 2 scheduled)
                (nthcdr 3 scheduled))
               (list
                before
                (buffer-string)
                changes
                aggressive-indent--changed-list
                (list
                 (nth 0 scheduled)
                 (nth 1 scheduled)
                 (eq
                  (nth 2 scheduled)
                  #'aggressive-indent--indent-if-changed)
                 (bufferp (nth 3 scheduled))))))))"##;
    let expect = expect![[
        r#"OK ("(defun timer-workflow ()\n(let ((value 3))\n(message \"%s\" value)))\n" "(defun timer-workflow ()\n  (let ((value 3))\n    (message \"%s\" value)))\n" ((43 66) (26 43) (1 26)) nil (0.05 t t t))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}
