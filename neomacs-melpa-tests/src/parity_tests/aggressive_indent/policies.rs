use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_comment_policy_defers_then_processes_the_same_real_edit() {
    let elisp_form = r####"(with-temp-buffer
                            (emacs-lisp-mode)
                            (setq
                             aggressive-indent-sit-for-time
                             60)
                            (aggressive-indent-mode
                             1)
                            (insert
                             "(defun recover ()\n"
                             "(message \"misindented\")\n"
                             "  ;; explain the recovery")
                            (timer-event-handler
                             aggressive-indent--idle-timer)
                            (let ((suppressed
                                   (list
                                    (buffer-string)
                                    (point)
                                    (and
                                     aggressive-indent--changed-list
                                     t)
                                    aggressive-indent--idle-timer
                                    (nth
                                     4
                                     (syntax-ppss)))))
                              (let ((aggressive-indent-comments-too
                                     t))
                                (insert
                                 " now")
                                (timer-event-handler
                                 aggressive-indent--idle-timer))
                              (list
                               suppressed
                               (buffer-string)
                               (point)
                               aggressive-indent--changed-list
                               aggressive-indent--idle-timer)))"####;
    let expect = expect![[
        r#"OK (("(defun recover ()\n(message \"misindented\")\n  ;; explain the recovery" 68 t nil t) "(defun recover ()\n  (message \"misindented\")\n  ;; explain the recovery now" 74 nil nil)"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}
