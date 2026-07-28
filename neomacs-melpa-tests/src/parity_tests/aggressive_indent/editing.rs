use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_public_defun_command_preserves_cursor_and_supports_complete_undo() {
    let elisp_form = r####"(with-temp-buffer
                            (emacs-lisp-mode)
                            (buffer-enable-undo)
                            (insert
                             "(defun publish (value)\n"
                             "  (let ((payload (list :value value)))\n"
                             "(message \"%S\" payload)))")
                            (goto-char
                             (point-min))
                            (search-forward
                             "message")
                            (let ((original
                                   (buffer-string))
                                  (original-point
                                   (point)))
                              (setq
                               buffer-undo-list
                               nil)
                              (undo-boundary)
                              (aggressive-indent-indent-defun)
                              (undo-boundary)
                              (let ((indented
                                     (buffer-string))
                                    (indented-point
                                     (point)))
                                (setq
                                 buffer-undo-list
                                 (primitive-undo
                                  2
                                  buffer-undo-list))
                                (list
                                 original
                                 original-point
                                 indented
                                 indented-point
                                 (buffer-string)
                                 (point)
                                 buffer-undo-list))))"####;
    let expect = expect![[
        r#"OK ("(defun publish (value)\n  (let ((payload (list :value value)))\n(message \"%S\" payload)))" 71 "(defun publish (value)\n  (let ((payload (list :value value)))\n    (message \"%S\" payload)))" 75 "(defun publish (value)\n  (let ((payload (list :value value)))\n(message \"%S\" payload)))" 71 nil)"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}
