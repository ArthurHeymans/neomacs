use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_global_mode_edits_only_eligible_live_buffers_and_disables_cleanly() {
    let elisp_form = r####"(let ((aggressive-indent-excluded-modes
                                  '(c-mode))
                                 lisp-buffer
                                 c-buffer
                                 text-buffer
                                 readonly-buffer)
                             (setq
                              lisp-buffer
                              (generate-new-buffer
                               " *aggressive-indent-lisp*")
                              c-buffer
                              (generate-new-buffer
                               " *aggressive-indent-c*")
                              text-buffer
                              (generate-new-buffer
                               " *aggressive-indent-text*")
                              readonly-buffer
                              (generate-new-buffer
                               " *aggressive-indent-readonly*"))
                             (unwind-protect
                                 (progn
                                   (with-current-buffer
                                       lisp-buffer
                                     (emacs-lisp-mode)
                                     (setq
                                      aggressive-indent-sit-for-time
                                      60))
                                   (with-current-buffer
                                       c-buffer
                                     (c-mode))
                                   (with-current-buffer
                                       text-buffer
                                     (text-mode))
                                   (with-current-buffer
                                       readonly-buffer
                                     (emacs-lisp-mode)
                                     (setq
                                      buffer-read-only
                                      t))
                                   (global-aggressive-indent-mode
                                    1)
                                   (with-current-buffer
                                       lisp-buffer
                                     (insert
                                      "(defun deploy ()\n"
                                      "(when ready\n"
                                      "(message \"deploy\")))")
                                     (timer-event-handler
                                      aggressive-indent--idle-timer))
                                   (with-current-buffer
                                       c-buffer
                                     (insert
                                      "int main(void) {\nreturn 0;\n}\n"))
                                   (with-current-buffer
                                       text-buffer
                                     (insert
                                      "Operational prose remains outside the programming-mode workflow."))
                                   (let ((enabled
                                          (list
                                           global-aggressive-indent-mode
                                           (with-current-buffer
                                               lisp-buffer
                                             (list
                                              aggressive-indent-mode
                                              (buffer-string)
                                              (point)
                                              aggressive-indent--changed-list
                                              aggressive-indent--idle-timer))
                                           (with-current-buffer
                                               c-buffer
                                             (list
                                              aggressive-indent-mode
                                              (buffer-substring-no-properties
                                               (point-min)
                                               (point-max))))
                                           (with-current-buffer
                                               text-buffer
                                             (list
                                              aggressive-indent-mode
                                              (buffer-string)))
                                           (with-current-buffer
                                               readonly-buffer
                                             (list
                                              aggressive-indent-mode
                                              buffer-read-only)))))
                                     (global-aggressive-indent-mode
                                      -1)
                                     (list
                                      enabled
                                      global-aggressive-indent-mode
                                      (mapcar
                                       (lambda (buffer)
                                         (with-current-buffer
                                             buffer
                                           aggressive-indent-mode))
                                       (list
                                        lisp-buffer
                                        c-buffer
                                        text-buffer
                                        readonly-buffer)))))
                               (global-aggressive-indent-mode
                                -1)
                               (when
                                   (buffer-live-p
                                    readonly-buffer)
                                 (with-current-buffer
                                     readonly-buffer
                                   (setq
                                    buffer-read-only
                                    nil)))
                               (dolist (buffer
                                        (list
                                         lisp-buffer
                                         c-buffer
                                         text-buffer
                                         readonly-buffer))
                                 (when
                                     (buffer-live-p
                                      buffer)
                                   (with-current-buffer
                                       buffer
                                     (set-buffer-modified-p
                                      nil))
                                   (kill-buffer
                                    buffer)))))"####;
    let expect = expect![[
        r#"OK ((t (t "(defun deploy ()\n  (when ready\n    (message \"deploy\")))" 56 nil nil) (nil "int main(void) {\nreturn 0;\n}\n") (nil "Operational prose remains outside the programming-mode workflow.") (nil t)) nil (nil nil nil nil))"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}
