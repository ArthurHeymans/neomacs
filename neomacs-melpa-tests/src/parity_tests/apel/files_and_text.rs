use expect_test::expect;

use super::assert_apel_source_parity;

#[test]
fn filename_filter_pipeline_sanitizes_practical_cross_platform_document_names() {
    let elisp_form = r##"(let ((filename-limit-length 18)
                           (filename-filters
                            '(filename-special-filter
                              filename-eliminate-top-low-lines
                              filename-canonicalize-low-lines
                              filename-maybe-truncate-by-size
                              filename-eliminate-bottom-low-lines)))
                      (mapcar #'replace-as-filename
                              '("  Quarterly / report: 2026?.org  "
                                "__project___notes__"
                                "short-name.txt"
                                "012345678901234567_suffix ignored"
                                nil)))"##;
    let expect = expect![[
        r#"OK ("Quarterly_report_2026" "project_notes" "short-name.txt" "012345678901234567" nil)"#
    ]];
    assert_apel_source_parity("filename.el", elisp_form, expect);
}

#[test]
fn individual_filename_filters_and_poly_pipeline_cover_boundaries() {
    let elisp_form = r##"(let ((filename-limit-length 5))
                      (list (mapcar #'filename-control-p '(0 31 32 126 127 128))
                            (filename-special-filter "a b/c:d\t")
                            (filename-eliminate-top-low-lines "___alpha__")
                            (filename-canonicalize-low-lines "a___b____c")
                            (filename-maybe-truncate-by-size "12345_suffix")
                            (filename-maybe-truncate-by-size "1234_suffix")
                            (filename-eliminate-bottom-low-lines "alpha___")
                            (poly-funcall
                             (list #'filename-special-filter
                                   #'filename-canonicalize-low-lines)
                             "a / b")))"##;
    let expect = expect![[
        r#"OK ((t t nil nil t nil) "a_b_c_d_" "alpha__" "a_b_c" "12345" "1234_suffix" "alpha" "a_b")"#
    ]];
    assert_apel_source_parity("filename.el", elisp_form, expect);
}

#[test]
fn load_path_workflow_finds_relative_absolute_latest_and_duplicate_paths() {
    let elisp_form = r##"(let* ((root (expand-file-name "apel-paths" default-directory))
                           (older (expand-file-name "plugin-1" root))
                           (newer (expand-file-name "plugin-2" root))
                           (default-load-path (list root))
                           (load-path nil))
                      (make-directory older t)
                      (make-directory newer t)
                      (set-file-times older '(10000 0 0 0))
                      (set-file-times newer '(20000 0 0 0))
                      (add-path "plugin-1")
                      (add-path "plugin-1/")
                      (add-path newer 'append)
                      (let ((latest (get-latest-path "\\`plugin-[0-9]+\\'")))
                        (add-latest-path "\\`plugin-[0-9]+\\'")
                        (list (mapcar #'file-name-nondirectory
                                      (mapcar #'directory-file-name load-path))
                              (file-name-nondirectory
                               (directory-file-name latest))
                              (= (length load-path) 2))))"##;
    let expect = expect![[r#"OK (("plugin-1" "plugin-2") "plugin-2" t)"#]];
    assert_apel_source_parity("path-util.el", elisp_form, expect);
}

#[test]
fn installed_file_executable_and_module_detection_use_real_sandbox_files() {
    let elisp_form = r##"(let* ((root (expand-file-name "apel-installed" default-directory))
                           (library (expand-file-name "demo.el" root))
                           (program (expand-file-name "runner.tool" root)))
                      (make-directory root t)
                      (with-temp-file library (insert "(provide 'demo)"))
                      (with-temp-file program (insert "#!/bin/sh\nexit 0\n"))
                      (set-file-modes program #o755)
                      (list
                       (file-name-nondirectory
                        (file-installed-p "demo.el" (list root)))
                       (file-installed-p "missing.el" (list root))
                       (file-name-nondirectory
                        (exec-installed-p "runner" (list root) '(".tool" "")))
                       (exec-installed-p "missing" (list root) '(".tool"))
                       (file-name-nondirectory
                        (module-installed-p 'demo (list root)))
                       (progn (provide 'already-loaded)
                              (module-installed-p 'already-loaded (list root)))
                       (module-installed-p 'absent (list root))))"##;
    let expect = expect![[r#"OK ("demo.el" nil "runner.tool" nil "demo.el" t nil)"#]];
    assert_apel_source_parity("path-util.el", elisp_form, expect);
}

#[test]
fn visibility_workflow_hides_region_except_trailing_newline_and_restores_subset() {
    let elisp_form = r##"(with-temp-buffer
                     (insert "public\nsecret line\nvisible tail\n")
                     (let ((secret-start 8)
                           (secret-end 20))
                       (invisible-region secret-start secret-end)
                       (let ((hidden
                              (list (get-text-property secret-start 'invisible)
                                    (get-text-property (1- secret-end) 'invisible)
                                    (get-text-property secret-end 'invisible)
                                    (next-visible-point secret-start))))
                         (visible-region (+ secret-start 2) (- secret-end 2))
                         (list hidden
                               (buffer-substring-no-properties
                                secret-start secret-end)
                               (mapcar
                                (lambda (position)
                                  (get-text-property position 'invisible))
                                (number-sequence secret-start secret-end))
                               (enable-invisible)
                               (disable-invisible)
                               (end-of-invisible)))))"##;
    let expect = expect![[
        r#"OK ((t nil nil 20) "secret line\n" (t t nil nil nil nil nil nil nil nil t nil nil) nil nil nil)"#
    ]];
    assert_apel_source_parity("invisible.el", elisp_form, expect);
}

#[test]
fn destructive_association_removal_covers_equal_eq_cdr_and_improper_entries() {
    let elisp_form = r##"(let* ((equal-key (copy-sequence "key"))
                           (eq-value (list :shared))
                           (by-car (list (cons "key" 1) 'noise
                                         (cons equal-key 2) (cons "other" 3)))
                           (by-symbol (list (cons 'key 1) (cons 'other 2)
                                            (cons 'key 3)))
                           (by-cdr (list (cons 'a '(1 2)) 'noise
                                        (cons 'b (list 1 2)) (cons 'c 3)))
                           (by-eq-cdr (list (cons 'a eq-value)
                                           (cons 'b (list :shared))
                                           (cons 'c eq-value))))
                      (list (remassoc "key" by-car)
                            (remassq 'key by-symbol)
                            (remrassoc '(1 2) by-cdr)
                            (remrassq eq-value by-eq-cdr)
                            by-car by-symbol by-cdr by-eq-cdr))"##;
    let expect = expect![[
        r#"OK (#1=(("other" . 3)) #2=((other . 2)) #3=((c . 3)) #4=((b :shared)) (("key" . 1) noise ("key" . 2) . #1#) ((key . 1) . #2#) ((a 1 2) noise (b 1 2) . #3#) ((a :shared) . #4#))"#
    ]];
    assert_apel_source_parity("poe.el", elisp_form, expect);
}

#[test]
fn character_event_and_string_compatibility_helpers_round_trip_real_text() {
    let elisp_form = r##"(list
                      (char-list-to-string '(65 955 20013))
                      (string-to-char-list "Aλ中")
                      (string-to-int-list "Aλ中")
                      (mapcar (lambda (character)
                                (list character
                                      (char-int character)
                                      (int-char (char-int character))
                                      (char-length character)
                                      (char-octet character)
                                      (char-or-char-int-p character)))
                              '(65 955 20013))
                      (character-to-event ?x)
                      (event-to-character (character-to-event ?x))
                      (char-category ?A))"##;
    let expect = expect![[
        r#"OK ("Aλ中" (65 955 20013) (65 955 20013) ((65 65 65 1 65 t) (955 955 955 1 3 t) (20013 20013 20013 1 78 t)) 120 120 ".Lalr")"#
    ]];
    assert_apel_source_parity("emu.el", elisp_form, expect);
}

#[test]
fn richtext_parser_handles_escaped_lt_comments_and_balanced_annotations() {
    let elisp_form = r##"(with-temp-buffer
                     (insert "before <lt>tag<bold>strong</bold> "
                             "<comment>hidden</comment> after")
                     (goto-char (point-min))
                     (let (annotations)
                       (while-let ((annotation (richtext-next-annotation)))
                         (push annotation annotations))
                       (list (buffer-substring-no-properties
                              (point-min) (point-max))
                             (nreverse annotations)
                             (point))))"##;
    let expect = expect![[
        r#"OK ("before <tag<bold>strong</bold> <comment>hidden</comment> after" ((12 18 "bold" t) (24 31 "bold" nil) (32 41 "comment" t) (47 58 "comment" nil)) 58)"#
    ]];
    assert_apel_source_parity("richtext.el", elisp_form, expect);
}

#[test]
fn richtext_decode_turns_header_markup_and_hard_newlines_into_buffer_semantics() {
    let elisp_form = r##"(with-temp-buffer
                     (let ((fill-column 72)
                           (enriched-verbose nil))
                       (insert "Content-Type: text/richtext\n"
                               "Text-Width: 72\n\n"
                               "<bold>Hello</bold><nl>\n"
                               "world & <lt>literal")
                       (richtext-decode (point-min) (point-max))
                       (let ((content
                              (buffer-substring-no-properties
                               (point-min) (point-max)))
                             (face
                              (get-text-property (point-min) 'face))
                             (world-properties
                              (text-properties-at
                               (string-match "world" (buffer-string))))
                             (hard-boundary
                              (next-single-property-change
                               (point-min) 'hard nil (point-max))))
                         (list
                          (if (equal content "Hello\nworld & <literal")
                              'content-exact
                            content)
                          (if (equal face '(bold)) 'bold-exact face)
                          (if (equal world-properties
                                     '(front-sticky nil hard t))
                              'hard-newline-exact
                            world-properties)
                          hard-boundary))))"##;
    let expect = expect!["OK (content-exact bold-exact hard-newline-exact 6)"];
    assert_apel_source_parity("richtext.el", elisp_form, expect);
}
