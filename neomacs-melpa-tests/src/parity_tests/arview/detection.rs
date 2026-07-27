use expect_test::expect;

use super::assert_arview_parity;

#[test]
fn arview_file_extension_classifies_all_supported_tar_spellings_and_other_types() {
    let elisp_form = r##"(mapcar
               (lambda (filename)
                 (list
                  filename
                  (arview-file-extension
                   filename)))
               '("release.tar"
                 "release.TAR"
                 "release.tgz"
                 "release.TGZ"
                 "release.tar.gz"
                 "release.tar.bz"
                 "release.tar.bz2"
                 "release.tar.xz"
                 "release.tar.xz2"
                 "release.zip"
                 "release.ZIP"
                 "release.7z"
                 "release.rar"
                 "release.custom"
                 ".hidden.zip"
                 "name.with.many.parts.zip"))"##;
    let expect = expect![[
        r#"OK (("release.tar" tar) ("release.TAR" tar) ("release.tgz" tar) ("release.TGZ" tar) ("release.tar.gz" tar) ("release.tar.bz" tar) ("release.tar.bz2" tar) ("release.tar.xz" tar) ("release.tar.xz2" tar) ("release.zip" zip) ("release.ZIP" zip) ("release.7z" 7z) ("release.rar" rar) ("release.custom" custom) (".hidden.zip" zip) ("name.with.many.parts.zip" zip))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_file_extension_exposes_exact_missing_and_empty_extension_failures() {
    let elisp_form = r##"(mapcar
               (lambda (filename)
                 (condition-case error-data
                     (list
                      filename
                      :ok
                      (arview-file-extension
                       filename))
                   (error
                    (list
                     filename
                     :error
                     (car error-data)
                     (cdr error-data)))))
               '("README"
                 ".gitignore"
                 "trailing."
                 ""
                 "/workspace/directory/"))"##;
    let expect = expect![[
        r#"OK (("README" :error wrong-type-argument (char-or-string-p nil)) (".gitignore" :error wrong-type-argument (char-or-string-p nil)) ("trailing." :ok ##) ("" :error wrong-type-argument (char-or-string-p nil)) ("/workspace/directory/" :error wrong-type-argument (char-or-string-p nil)))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_file_archive_detects_real_tar_content_even_with_unrelated_extension() {
    let elisp_form = r##"(let* ((archive
                    (arview-test-create-tar
                     "content.blob"))
                   (detected
                    (arview-file-archive
                     archive)))
               (list
                detected
                (arview-file-extension
                 archive)
                (file-exists-p archive)
                (file-attribute-size
                 (file-attributes
                  archive))))"##;
    let expect = expect!["OK (tar blob t 10240)"];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_file_archive_passes_basename_after_double_dash_from_archive_directory() {
    let elisp_form = r##"(let* ((directory
                    (arview-test-path
                     "magic"))
                   (filename
                    (expand-file-name
                     "-fixture.data"
                     directory))
                   call)
               (make-directory directory t)
               (arview-test-write-file
                filename "payload")
               (cl-letf
                   (((symbol-function
                      'process-file)
                     (lambda
                       (program infile destination
                                display &rest arguments)
                       (setq call
                             (list
                              program
                              infile
                              (eq destination
                                  (current-buffer))
                              display
                              arguments
                              default-directory))
                       (insert
                        "-fixture.data: Zip archive data")
                       0)))
                 (list
                  (arview-file-archive
                   filename)
                  call
                  default-directory)))"##;
    let expect = expect![[
        r#"OK (zip ("file" nil nil t ("--" "-fixture.data") "[ORACLE-SANDBOX]/magic/") "[ORACLE-SANDBOX]/")"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_file_archive_honors_custom_ordered_case_sensitive_magic_patterns() {
    let elisp_form = r##"(let* ((filename
                    (arview-test-path
                     "custom.magic"))
                   (outputs
                    '("custom.magic: ALPHA payload"
                      "custom.magic: alpha payload"
                      "custom.magic: beta payload"))
                   (arview-file-alist
                    '((first . ".*: ALPHA")
                      (second . ".*: alpha")
                      (third . ".*: beta")))
                   results)
               (arview-test-write-file
                filename "payload")
               (dolist (output outputs)
                 (cl-letf
                     (((symbol-function
                        'process-file)
                       (lambda (&rest _)
                         (insert output)
                         0)))
                   (push
                    (arview-file-archive
                     filename)
                    results)))
               (nreverse results))"##;
    let expect = expect!["OK (first second third)"];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_file_archive_returns_nil_for_unmatched_or_empty_file_output() {
    let elisp_form = r##"(let ((filename
                    (arview-test-path
                     "plain.txt")))
               (arview-test-write-file
                filename "plain")
               (mapcar
                (lambda (output)
                  (cl-letf
                      (((symbol-function
                         'process-file)
                        (lambda (&rest _)
                          (insert output)
                          1)))
                    (list
                     output
                     (arview-file-archive
                      filename))))
                '("plain.txt: ASCII text"
                  ""
                  "plain.txt: Rar archive data"
                  "plain.txt: zip archive data")))"##;
    let expect = expect![[
        r#"OK (("plain.txt: ASCII text" nil) ("" nil) ("plain.txt: Rar archive data" nil) ("plain.txt: zip archive data" nil))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_archive_type_stops_after_first_non_nil_detector() {
    let elisp_form = r##"(let ((arview-archive-type-functions
                    '(arview-test-detector-a
                      arview-test-detector-b
                      arview-test-detector-c))
                   calls)
               (cl-letf
                   (((symbol-function
                      'arview-test-detector-a)
                     (lambda (filename)
                       (push
                        (list 'a filename)
                        calls)
                       nil))
                    ((symbol-function
                      'arview-test-detector-b)
                     (lambda (filename)
                       (push
                        (list 'b filename)
                        calls)
                       'zip))
                    ((symbol-function
                      'arview-test-detector-c)
                     (lambda (filename)
                       (push
                        (list 'c filename)
                        calls)
                       'unexpected)))
                 (list
                  (arview-archive-type
                   "/work/archive.bin")
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (zip ((a "/work/archive.bin") (b "/work/archive.bin")))"#]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_archive_type_returns_nil_after_all_detectors_and_propagates_signals() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'arview-test-signaling-detector)
                 (lambda (filename)
                   (error
                    "detector failed for %s"
                    filename))))
               (mapcar
                (lambda (functions)
                  (let ((arview-archive-type-functions
                         functions))
                    (condition-case error-data
                        (list
                         functions
                         :ok
                         (arview-archive-type
                          "fixture.bin"))
                      (error
                       (list
                        functions
                        :error
                        (car error-data)
                        (cdr error-data))))))
                '(nil
                  (ignore ignore)
                  (ignore
                   arview-test-signaling-detector)
                  (arview-test-missing-detector))))"##;
    let expect = expect![[
        r#"OK ((nil :ok nil) ((ignore ignore) :ok nil) ((ignore arview-test-signaling-detector) :error error ("detector failed for fixture.bin")) ((arview-test-missing-detector) :error void-function (arview-test-missing-detector)))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_default_detection_prefers_real_file_magic_before_misleading_extension() {
    let elisp_form = r##"(let* ((archive
                    (arview-test-create-tar
                     "misleading.zip"))
                   (default-order
                    (arview-archive-type
                     archive))
                   (extension-first
                    (let ((arview-archive-type-functions
                           '(arview-file-extension
                             arview-file-archive)))
                      (arview-archive-type
                       archive))))
               (list
                arview-archive-type-functions
                default-order
                extension-first))"##;
    let expect = expect!["OK ((arview-file-archive arview-file-extension) tar zip)"];
    assert_arview_parity(elisp_form, expect);
}
