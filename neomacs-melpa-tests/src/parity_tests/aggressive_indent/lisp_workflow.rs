use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_real_idle_dispatch_repairs_a_complete_lisp_edit_and_preserves_point() {
    let elisp_form = r####"(with-temp-buffer
                            (emacs-lisp-mode)
                            (setq
                             aggressive-indent-sit-for-time
                             60)
                            (aggressive-indent-mode
                             1)
                            (insert
                             "(defun reconcile (items)\n"
                             "(mapcar\n"
                             "(lambda (item)\n"
                             "(when item\n"
                             "(message \"%s\" item)))\n"
                             "items))")
                            (let ((before
                                   (list
                                    (buffer-string)
                                    (point)
                                    (and
                                     aggressive-indent--changed-list
                                     t)
                                    (timerp
                                     aggressive-indent--idle-timer)
                                    aggressive-indent-mode)))
                              (timer-event-handler
                               aggressive-indent--idle-timer)
                              (list
                               before
                               (buffer-string)
                               (point)
                               aggressive-indent--changed-list
                               aggressive-indent--idle-timer
                               aggressive-indent-mode)))"####;
    let expect = expect![[
        r#"OK (("(defun reconcile (items)\n(mapcar\n(lambda (item)\n(when item\n(message \"%s\" item)))\nitems))" 89 t t t) "(defun reconcile (items)\n  (mapcar\n   (lambda (item)\n     (when item\n       (message \"%s\" item)))\n   items))" 109 nil nil t)"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}
