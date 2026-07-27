use expect_test::expect;

use super::assert_arview_parity;

#[test]
fn arview_kill_buffer_hook_deletes_real_dired_directory_for_local_archive_state() {
    let elisp_form = r##"(let ((directory
                    (file-name-as-directory
                     (arview-test-path
                      "cleanup-local"))))
               (make-directory directory t)
               (arview-test-write-file
                (expand-file-name
                 "nested/payload.txt"
                 directory)
                "payload")
               (with-temp-buffer
                 (setq default-directory
                       directory)
                 (setq major-mode
                       'dired-mode)
                 (setq arview-buffer-p t)
                 (let ((result
                        (arview-kill-buffer-hook)))
                   (setq arview-buffer-p nil)
                   (list
                    result
                    (file-exists-p directory)
                    default-directory
                    major-mode))))"##;
    let expect = expect![[r#"OK (nil nil "[ORACLE-SANDBOX]/cleanup-local/" dired-mode)"#]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_kill_buffer_hook_deletes_directory_and_copied_archive_for_string_state() {
    let elisp_form = r##"(let ((directory
                    (file-name-as-directory
                     (arview-test-path
                      "cleanup-remote")))
                   (copy
                    (arview-test-path
                     "copied remote archive.tar")))
               (make-directory directory t)
               (arview-test-write-file
                (expand-file-name
                 "payload.txt"
                 directory)
                "payload")
               (arview-test-write-file
                copy "archive")
               (with-temp-buffer
                 (setq default-directory
                       directory)
                 (setq major-mode
                       'dired-mode)
                 (setq arview-buffer-p
                       copy)
                 (let ((result
                        (arview-kill-buffer-hook)))
                   (setq arview-buffer-p nil)
                   (list
                    result
                    (file-exists-p directory)
                    (file-exists-p copy)
                    default-directory))))"##;
    let expect = expect![[r#"OK (nil nil nil "[ORACLE-SANDBOX]/cleanup-remote/")"#]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_kill_buffer_hook_ignores_non_dired_buffers_even_with_archive_state() {
    let elisp_form = r##"(let ((directory
                    (file-name-as-directory
                     (arview-test-path
                      "cleanup-fundamental")))
                   (copy
                    (arview-test-path
                     "cleanup-copy.tar")))
               (make-directory directory t)
               (arview-test-write-file
                copy "archive")
               (unwind-protect
                   (with-temp-buffer
                     (setq default-directory
                           directory)
                     (setq major-mode
                           'fundamental-mode)
                     (setq arview-buffer-p
                           copy)
                     (list
                      (arview-kill-buffer-hook)
                      (file-exists-p directory)
                      (file-exists-p copy)
                      arview-buffer-p))
                 (when
                     (file-exists-p directory)
                   (delete-directory
                    directory t))
                 (when
                     (file-exists-p copy)
                   (delete-file copy))))"##;
    let expect = expect![[r#"OK (nil t t "[ORACLE-SANDBOX]/cleanup-copy.tar")"#]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_kill_buffer_hook_ignores_dired_buffers_without_archive_state() {
    let elisp_form = r##"(let ((directory
                    (file-name-as-directory
                     (arview-test-path
                      "cleanup-unmarked"))))
               (make-directory directory t)
               (unwind-protect
                   (with-temp-buffer
                     (setq default-directory
                           directory)
                     (setq major-mode
                           'dired-mode)
                     (setq arview-buffer-p nil)
                     (list
                      (arview-kill-buffer-hook)
                      (file-exists-p directory)
                      arview-buffer-p))
                 (when
                     (file-exists-p directory)
                   (delete-directory
                    directory t))))"##;
    let expect = expect!["OK (nil t nil)"];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_kill_buffer_hook_directory_failure_prevents_copied_archive_deletion() {
    let elisp_form = r##"(let ((directory
                    (file-name-as-directory
                     (arview-test-path
                      "undeletable-directory")))
                   (copy
                    (arview-test-path
                     "preserved-copy.tar"))
                   calls)
               (make-directory directory t)
               (arview-test-write-file
                copy "archive")
               (unwind-protect
                   (with-temp-buffer
                     (setq default-directory
                           directory)
                     (setq major-mode
                           'dired-mode)
                     (setq arview-buffer-p
                           copy)
                     (cl-letf
                         (((symbol-function
                            'delete-directory)
                           (lambda (&rest arguments)
                             (push
                              (cons
                               :directory arguments)
                              calls)
                             (error
                              "refused directory removal")))
                          ((symbol-function
                            'delete-file)
                           (lambda (&rest arguments)
                             (push
                              (cons
                               :file arguments)
                              calls)
                             :unexpected)))
                       (let ((result
                              (condition-case error-data
                                  (list
                                   :ok
                                   (arview-kill-buffer-hook))
                                (error
                                 (list
                                  :error
                                  (car error-data)
                                  (cdr error-data))))))
                         (setq arview-buffer-p nil)
                         (list
                          result
                          (nreverse calls)
                          (file-exists-p directory)
                          (file-exists-p copy)))))
                 (when
                     (file-exists-p directory)
                   (delete-directory
                    directory t))
                 (when
                     (file-exists-p copy)
                   (delete-file copy))))"##;
    let expect = expect![[
        r#"OK ((:error error ("refused directory removal")) ((:directory "[ORACLE-SANDBOX]/undeletable-directory/" t)) t t)"#
    ]];
    assert_arview_parity(elisp_form, expect);
}
