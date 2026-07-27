use expect_test::expect;

use super::assert_archive_region_parity;

#[test]
fn archive_region_open_archive_file_passes_exact_destination_to_custom_function_and_returns_result()
{
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "open-custom.org"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix))
                calls)
         (with-temp-file
             archive
           (insert
            "archived content"))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (list
                (archive-region-open-archive-file
                 (lambda (path)
                   (push path calls)
                   :opened))
                (nreverse
                 calls)
                (file-exists-p
                 archive)
                (buffer-string)
                buffer-file-name))
           (delete-file archive)))"##;
    let expect = expect![[
        r#"OK (:opened ("[ORACLE-SANDBOX]/open-custom.org_archive") t "" "[ORACLE-SANDBOX]/open-custom.org")"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_open_archive_file_default_visits_real_file_content_and_mode() {
    let elisp_form = r##"(save-window-excursion
         (let* ((source
                 (archive-region-test-path
                  "open-default.txt"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix)))
           (with-temp-file
               archive
             (insert
              "first archived block\n"
              "second archived block\n"))
           (unwind-protect
               (with-temp-buffer
                 (setq-local
                  buffer-file-name
                  source)
                 (let ((source-buffer
                        (current-buffer))
                       (result
                        (archive-region-open-archive-file)))
                   (list
                    result
                    (eq
                     source-buffer
                     (current-buffer))
                    (file-name-nondirectory
                     buffer-file-name)
                    major-mode
                    (buffer-string)
                    (buffer-modified-p))))
             (archive-region-test-kill-file-buffers)
             (delete-file archive))))"##;
    let expect = expect![[
        r#"OK ((:buffer nil) nil "open-default.txt_archive" fundamental-mode "first archived block\nsecond archived block\n" nil)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_open_archive_file_missing_destination_errors_before_invoking_function() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "missing-open.txt"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix))
                calls)
         (when
             (file-exists-p
              archive)
           (delete-file archive))
         (with-temp-buffer
           (setq-local
            buffer-file-name
            source)
           (list
            (condition-case error-data
                (list
                 :ok
                 (archive-region-open-archive-file
                  (lambda (path)
                    (push path calls)
                    :unexpected)))
              (error
               (list
                :error
                (car error-data)
                (cdr error-data))))
            calls
            (file-exists-p
             archive)
            (buffer-string))))"##;
    let expect = expect![[r#"OK ((:error error ("Archive file does not exist.")) nil nil "")"#]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_open_other_window_delegates_with_exact_find_file_other_window_symbol() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'archive-region-open-archive-file)
               (lambda (&optional function)
                 (push function calls)
                 :other-window-opened)))
           (list
            (archive-region-open-archive-file-other-window)
            calls
            (commandp
             'archive-region-open-archive-file-other-window)
            (interactive-form
             'archive-region-open-archive-file-other-window))))"##;
    let expect = expect!["OK (:other-window-opened (find-file-other-window) t (interactive nil))"];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_open_archive_file_honors_custom_regex_like_unicode_suffix() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "suffix.md"))
                (archive-region-filename-suffix
                 ".[保管]+")
                (archive
                 (concat
                  source
                  archive-region-filename-suffix))
                received)
         (with-temp-file
             archive
           (insert
            "custom suffix payload"))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (cl-letf
                   (((symbol-function
                      'find-file)
                     (lambda (path)
                       (setq received path)
                       :custom-open)))
                 (list
                  (archive-region-open-archive-file)
                  received
                  (archive-region-current-archive-file)
                  (archive-region-test-read-file
                   received))))
           (delete-file archive)))"##;
    let expect = expect![[
        r#"OK (:custom-open "[ORACLE-SANDBOX]/suffix.md.[保管]+" "[ORACLE-SANDBOX]/suffix.md.[保管]+" "custom suffix payload")"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}
