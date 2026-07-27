use expect_test::expect;

use super::assert_archive_region_parity;

#[test]
fn archive_region_moves_real_commented_lisp_region_to_archive_with_header_and_uncommented_payload()
{
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "basic.el"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix))
                result)
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (emacs-lisp-mode)
               (insert
                "(keep-before)\n"
                ";; (archived-one)\n"
                ";; (archived-two)\n"
                "(keep-after)\n")
               (with-temp-file
                   source
                 (insert
                  "disk-original\n"))
               (goto-char
                (point-min))
               (forward-line 1)
               (let ((start
                      (point)))
                 (forward-line 2)
                 (let ((end
                        (point)))
                   (cl-letf
                       (((symbol-function
                          'format-time-string)
                         (lambda (&rest _)
                           "[2024/03/04]")))
                     (setq result
                           (archive-region
                            start
                            end)))))
               (list
                result
                (buffer-string)
                (point)
                (file-exists-p
                 archive)
                (archive-region-test-read-file
                 archive)
                (archive-region-test-read-file
                 source)))
           (when
               (file-exists-p
                archive)
             (delete-file archive))
           (when
               (file-exists-p
                source)
             (delete-file source))))"##;
    let expect = expect![[
        r#"OK (nil #("(keep-before)\n(keep-after)\n" 0 14 (fontified nil) 14 27 (fontified nil)) 15 t ";; [2024/03/04]\n;; (archive-region-pos \"(keep-before)\")\n(archived-one)\n(archived-two)\n\n" "disk-original\n")"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_first_line_selection_records_nil_navigation_context() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "first-line.txt"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix)))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (setq-local
                comment-start
                "#")
               (setq-local
                comment-end
                "")
               (insert
                "archive first\n"
                "keep second\n")
               (cl-letf
                   (((symbol-function
                      'format-time-string)
                     (lambda (&rest _)
                       "DATE")))
                 (archive-region
                  (point-min)
                  (progn
                    (goto-char
                     (point-min))
                    (forward-line 1)
                    (point))))
               (let ((archived
                      (archive-region-test-read-file
                       archive)))
                 (list
                  (buffer-string)
                  archived
                  (string-match-p
                   "(archive-region-pos nil)"
                   archived))))
           (when
               (file-exists-p
                archive)
             (delete-file archive))))"##;
    let expect = expect![[
        r##"OK ("keep second\n" "# DATE\n# (archive-region-pos nil)\narchive first\n\n" 9)"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_selection_after_blank_lines_links_to_previous_nonempty_source_line() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "context.txt"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix)))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (setq-local
                comment-start
                "#")
               (setq-local
                comment-end
                "")
               (insert
                "context heading\n"
                "\n"
                "\n"
                "# archived payload\n"
                "keep tail\n")
               (goto-char
                (point-min))
               (forward-line 3)
               (let ((start
                      (point)))
                 (forward-line 1)
                 (cl-letf
                     (((symbol-function
                        'format-time-string)
                       (lambda (&rest _)
                         "DATE")))
                   (archive-region
                    start
                    (point))))
               (list
                (buffer-string)
                (archive-region-test-read-file
                 archive)))
           (when
               (file-exists-p
                archive)
             (delete-file archive))))"##;
    let expect = expect![[
        r##"OK ("context heading\n\n\nkeep tail\n" "# DATE\n# (archive-region-pos \"context heading\")\narchived payload\n\n")"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_two_sequential_moves_append_in_invocation_order_without_overwriting() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "append-order.txt"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix))
                (dates
                 '("DATE-1"
                   "DATE-2")))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (setq-local
                comment-start
                "#")
               (setq-local
                comment-end
                "")
               (insert
                "keep-a\n"
                "# first archived\n"
                "keep-b\n"
                "# second archived\n"
                "keep-c\n")
               (cl-letf
                   (((symbol-function
                      'format-time-string)
                     (lambda (&rest _)
                       (pop dates))))
                 (goto-char
                  (point-min))
                 (forward-line 1)
                 (let ((start
                        (point)))
                   (forward-line 1)
                   (archive-region
                    start
                    (point)))
                 (goto-char
                  (point-min))
                 (forward-line 2)
                 (let ((start
                        (point)))
                   (forward-line 1)
                   (archive-region
                    start
                    (point))))
               (let ((archived
                      (archive-region-test-read-file
                       archive)))
                 (list
                  (buffer-string)
                  archived
                  (string-match
                   "DATE-1"
                   archived)
                  (string-match
                   "DATE-2"
                   archived)
                  dates)))
           (when
               (file-exists-p
                archive)
             (delete-file archive))))"##;
    let expect = expect![[
        r##"OK ("keep-a\nkeep-b\nkeep-c\n" "# DATE-1\n# (archive-region-pos \"keep-a\")\nfirst archived\n\n# DATE-2\n# (archive-region-pos \"keep-b\")\nsecond archived\n\n" 2 59 nil)"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_custom_suffix_and_date_format_drive_real_destination_and_header() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "custom.md"))
                (archive-region-filename-suffix
                 ".history")
                (archive-region-date-format
                 "%Y-%m-%dT%H:%M")
                (archive
                 (concat
                  source
                  archive-region-filename-suffix))
                captured-format)
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (setq-local
                comment-start
                "<!--")
               (setq-local
                comment-end
                "-->")
               (insert
                "custom archive payload")
               (cl-letf
                   (((symbol-function
                      'format-time-string)
                     (lambda (format-string &rest _)
                       (setq captured-format
                             format-string)
                       "2025-08-09T10:11")))
                 (archive-region
                  (point-min)
                  (point-max)))
               (list
                captured-format
                (buffer-string)
                (file-name-nondirectory
                 archive)
                (archive-region-test-read-file
                 archive)
                (file-exists-p
                 (concat
                  source
                  "_archive"))))
           (when
               (file-exists-p
                archive)
             (delete-file archive))))"##;
    let expect = expect![[
        r#"OK ("%Y-%m-%dT%H:%M" "" "custom.md.history" "<!-- 2025-08-09T10:11 -->\n<!-- (archive-region-pos nil) -->\ncustom archive payload\n" nil)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_reversed_bounds_expose_exact_failure_and_leave_source_and_files_unchanged() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "reversed.txt"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix)))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (insert
                "alpha\nbeta\ngamma\n")
               (let ((before
                      (buffer-string)))
                 (goto-char
                  (point-min))
                 (forward-line 2)
                 (let ((later
                        (point)))
                   (goto-char
                    (point-min))
                   (forward-line 1)
                   (let ((earlier
                          (point)))
                     (list
                      (condition-case error-data
                          (list
                           :ok
                           (archive-region
                            later
                            earlier))
                        (error
                         (list
                          :error
                          (car error-data)
                          (cdr error-data))))
                      before
                      (buffer-string)
                      (file-exists-p
                       archive))))))
           (when
               (file-exists-p
                archive)
             (delete-file archive))))"##;
    let expect = expect![[
        r#"OK ((:error end-of-file ("Error reading from stdin")) "alpha\nbeta\ngamma\n" "alpha\nbeta\ngamma\n" nil)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_partial_line_selection_archives_exact_characters_and_preserves_surrounding_text()
{
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "partial.txt"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix)))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (setq-local
                comment-start
                "#")
               (setq-local
                comment-end
                "")
               (insert
                "prefix ARCHIVE-ME suffix")
               (goto-char
                (point-min))
               (search-forward
                "ARCHIVE-ME")
               (let ((end
                      (point))
                     (start
                      (match-beginning 0)))
                 (cl-letf
                     (((symbol-function
                        'format-time-string)
                       (lambda (&rest _)
                         "DATE")))
                   (archive-region
                    start
                    end)))
               (list
                (buffer-string)
                (archive-region-test-read-file
                 archive)))
           (when
               (file-exists-p
                archive)
             (delete-file archive))))"##;
    let expect =
        expect![[r##"OK ("prefix  suffix" "# DATE\n# (archive-region-pos nil)\nARCHIVE-ME\n")"##]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_non_file_buffer_errors_before_changing_text_or_creating_destination() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "# archive me\n"
          "keep me\n")
         (setq-local
          comment-start
          "#")
         (setq-local
          comment-end
          "")
         (let ((before
                (buffer-string)))
           (list
            (condition-case error-data
                (list
                 :ok
                 (archive-region
                  (point-min)
                  (progn
                    (goto-char
                     (point-min))
                    (forward-line 1)
                    (point))))
              (error
               (list
                :error
                (car error-data)
                (cdr error-data))))
            before
            (buffer-string)
            buffer-file-name
            (point-min)
            (point-max))))"##;
    let expect = expect![[
        r##"OK ((:error error ("Need filename")) "# archive me\nkeep me\n" "# archive me\nkeep me\n" nil 1 22)"##
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_missing_parent_failure_exposes_pre_append_buffer_mutation_and_no_file() {
    let elisp_form = r##"(let* ((parent
                 (archive-region-test-path
                  "missing-parent"))
                (source
                 (expand-file-name
                  "source.el"
                  parent))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix)))
         (when
             (file-directory-p
              parent)
           (delete-directory
            parent
            t))
         (with-temp-buffer
           (setq-local
            buffer-file-name
            source)
           (emacs-lisp-mode)
           (insert
            ";; (archive-me)\n"
            "(keep)\n")
           (let ((before
                  (buffer-substring-no-properties
                   (point-min)
                   (point-max))))
             (cl-letf
                 (((symbol-function
                    'format-time-string)
                   (lambda (&rest _)
                     "DATE")))
               (list
                (condition-case error-data
                    (list
                     :ok
                     (archive-region
                      (point-min)
                      (progn
                        (goto-char
                         (point-min))
                        (forward-line 1)
                        (point))))
                  (error
                   (list
                    :error
                    (car error-data)
                    (cdr error-data))))
                before
                (buffer-substring-no-properties
                 (point-min)
                 (point-max))
                (file-exists-p
                 archive)
                (file-directory-p
                 parent)
                (point-min)
                (point-max))))))"##;
    let expect = expect![[
        r#"OK ((:error file-missing ("Opening output file" "No such file or directory" "[ORACLE-SANDBOX]/missing-parent/source.el_archive")) ";; (archive-me)\n(keep)\n" ";; DATE\n;; (archive-region-pos nil)\n(archive-me)\n\n(keep)\n" nil nil 1 58)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_appends_after_existing_bytes_and_adds_exact_single_trailing_newline() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "existing.txt"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix)))
         (with-temp-file
             archive
           (set-buffer-multibyte
            nil)
           (insert
            "EXISTING\n"))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (setq-local
                comment-start
                "#")
               (setq-local
                comment-end
                "")
               (insert
                "λ payload")
               (cl-letf
                   (((symbol-function
                      'format-time-string)
                     (lambda (&rest _)
                       "DATE")))
                 (archive-region
                  (point-min)
                  (point-max)))
               (let ((content
                      (archive-region-test-read-file
                       archive)))
                 (list
                  (buffer-string)
                  content
                  (string-suffix-p
                   "\n"
                   content)
                  (string-suffix-p
                   "\n\n"
                   content)
                  (string-bytes
                   content))))
           (when
               (file-exists-p
                archive)
             (delete-file archive))))"##;
    let expect = expect![[
        r#"OK ("" "EXISTING\n# DATE\n# (archive-region-pos nil)\n\316\273 payload\n" t nil 56)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}
