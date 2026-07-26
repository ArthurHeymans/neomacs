use expect_test::expect;

use super::assert_with_editor_parity;

#[test]
fn with_editor_sleeping_filter_opens_relative_file_at_line_and_column() {
    let elisp_form = r##"(let* ((root (make-temp-file
                           "with-editor-protocol-" t))
                    (file (expand-file-name "message.txt" root))
                    visited
                    buffer)
               (unwind-protect
                   (progn
                     (with-temp-file file
                       (insert "zero\none abc\ntwo\n"))
                     (let ((default-directory
                            (file-name-as-directory root))
                           (with-editor-server-window-alist
                            `(("\\`"
                               . ,(lambda (selected)
                                    (setq visited selected))))))
                       (with-editor-sleeping-editor-filter
                        nil
                        (format
                         "WITH-EDITOR: 4312 OPEN +2:4%cmessage.txt%c IN %s\n"
                         31 31 root)))
                     (setq buffer (find-buffer-visiting file))
                     (with-current-buffer buffer
                       (list
                        (eq visited buffer)
                        with-editor-mode
                        with-editor--pid
                        (line-number-at-pos)
                        (current-column)
                        (buffer-string))))
                 (when (buffer-live-p buffer)
                   (with-current-buffer buffer
                     (setq-local kill-buffer-query-functions nil)
                     (set-buffer-modified-p nil)
                     (kill-buffer buffer)))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK (t t "4312" 2 4 "zero\none abc\ntwo\n")"#]];

    assert_with_editor_parity(elisp_form, expect);
}

#[test]
fn with_editor_sleeping_filter_supports_absolute_file_without_position() {
    let elisp_form = r##"(let* ((file (make-temp-file
                           "with-editor-absolute-"
                           nil ".txt" "payload"))
                    visited
                    buffer)
               (unwind-protect
                   (progn
                     (let ((with-editor-server-window-alist
                            `(("\\`"
                               . ,(lambda (selected)
                                    (setq visited selected))))))
                       (with-editor-sleeping-editor-filter
                        nil
                        (format
                         "WITH-EDITOR: 99 OPEN %s%c IN /\n"
                         file 31)))
                     (setq buffer (find-buffer-visiting file))
                     (with-current-buffer buffer
                       (list
                        (eq visited buffer)
                        with-editor-mode
                        with-editor--pid
                        (point)
                        (buffer-string))))
                 (when (buffer-live-p buffer)
                   (with-current-buffer buffer
                     (setq-local kill-buffer-query-functions nil)
                     (set-buffer-modified-p nil)
                     (kill-buffer buffer)))
                 (delete-file file)))"##;
    let expect = expect![[r#"OK (t t "99" 1 "payload")"#]];

    assert_with_editor_parity(elisp_form, expect);
}

#[test]
fn with_editor_sleeping_filter_returns_non_protocol_output_unchanged() {
    let elisp_form = r##"(list
               (with-editor-sleeping-editor-filter
                nil "ordinary output\n")
               (with-editor-output-filter
                "partial ordinary output"))"##;
    let expect = expect![[r#"OK ("ordinary output\n" "partial ordinary output")"#]];

    assert_with_editor_parity(elisp_form, expect);
}
