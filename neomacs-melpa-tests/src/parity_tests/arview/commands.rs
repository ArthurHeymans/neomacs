use expect_test::expect;

use super::assert_arview_parity;

#[test]
fn arview_process_prefix_arg_nil_and_unrecognized_values_do_not_prompt() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'read-directory-name)
                     (lambda (&rest arguments)
                       (push
                        (cons 'directory
                              arguments)
                        calls)
                       "/unexpected/"))
                    ((symbol-function
                      'read-string)
                     (lambda (&rest arguments)
                       (push
                        (cons 'string
                              arguments)
                        calls)
                       "unexpected")))
                 (list
                  (mapcar
                   (lambda (argument)
                     (list
                      argument
                      (arview-process-prefix-arg
                       argument)))
                   '(nil 1 4 16
                     (1)
                     (64)
                     wrong))
                  calls)))"##;
    let expect =
        expect!["OK (((nil nil) (1 nil) (4 nil) (16 nil) ((1) nil) ((64) nil) (wrong nil)) nil)"];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_process_prefix_arg_single_prefix_reads_exact_existing_directory() {
    let elisp_form = r##"(let* ((temporary-file-directory
                     (arview-test-path
                      "default-temp/"))
                    (chosen
                     (arview-test-path
                      "chosen-temp/"))
                    calls)
               (make-directory
                temporary-file-directory t)
               (make-directory chosen t)
               (cl-letf
                   (((symbol-function
                      'read-directory-name)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       chosen))
                    ((symbol-function
                      'read-string)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       "unexpected")))
                 (list
                  (arview-process-prefix-arg
                   '(4))
                  (nreverse calls)
                  temporary-file-directory)))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-SANDBOX]/chosen-temp/") (("Temporary directory: " "[ORACLE-SANDBOX]/default-temp/" nil t)) "[ORACLE-SANDBOX]/default-temp/")"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_process_prefix_arg_double_prefix_reads_directory_and_prefixes_extra_arguments() {
    let elisp_form = r##"(let* ((temporary-file-directory
                     (arview-test-path
                      "default-temp/"))
                    (chosen
                     (arview-test-path
                      "chosen-temp/"))
                    calls)
               (make-directory
                temporary-file-directory t)
               (make-directory chosen t)
               (cl-letf
                   (((symbol-function
                      'read-directory-name)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'directory
                         arguments)
                        calls)
                       chosen))
                    ((symbol-function
                      'read-string)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'string
                         arguments)
                        calls)
                       "--strip-components=1")))
                 (list
                  (arview-process-prefix-arg
                   '(16))
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-SANDBOX]/chosen-temp/" " --strip-components=1") ((directory "Temporary directory: " "[ORACLE-SANDBOX]/default-temp/" nil t) (string "Additional arguments: ")))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_process_prefix_arg_directory_signal_prevents_second_prompt() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'read-directory-name)
                     (lambda (&rest arguments)
                       (push
                        (cons 'directory
                              arguments)
                        calls)
                       (error
                        "cancelled directory")))
                    ((symbol-function
                      'read-string)
                     (lambda (&rest arguments)
                       (push
                        (cons 'string
                              arguments)
                        calls)
                       "unexpected")))
                 (list
                  (condition-case error-data
                      (list
                       :ok
                       (arview-process-prefix-arg
                        '(16)))
                    (error
                     (list
                      :error
                      (car error-data)
                      (cdr error-data))))
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((:error error ("cancelled directory")) ((directory "Temporary directory: " "[ORACLE-TMPDIR]/" nil t)))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_command_forwards_filename_and_processed_prefix_arguments_exactly() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'arview-process-prefix-arg)
                     (lambda (argument)
                       (push
                        (list
                         :prefix argument)
                        calls)
                       (pcase argument
                         ('nil nil)
                         ('(4)
                          '("/custom/temp/"))
                         ('(16)
                          '("/custom/temp/"
                            " --verbose")))))
                    ((symbol-function
                      'arview-view)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         :view arguments)
                        calls)
                       :viewed)))
                 (list
                  (arview nil
                          "/work/plain.tar")
                  (arview '(4)
                          "/work/custom.zip")
                  (arview '(16)
                          "/work/verbose.7z")
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (:viewed :viewed :viewed ((:prefix nil) (:view "/work/plain.tar") (:prefix (4)) (:view "/work/custom.zip" "/custom/temp/") (:prefix (16)) (:view "/work/verbose.7z" "/custom/temp/" " --verbose")))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_dired_forwards_real_file_under_point_and_processed_prefix() {
    let elisp_form = r##"(let* ((directory
                     (arview-test-path
                      "dired-command/"))
                    (first
                     (expand-file-name
                      "alpha.tar"
                      directory))
                    (second
                     (expand-file-name
                      "space archive.zip"
                      directory))
                    calls
                    buffer)
               (make-directory directory t)
               (arview-test-write-file
                first "first")
               (arview-test-write-file
                second "second")
               (unwind-protect
                   (progn
                     (setq buffer
                           (dired-noselect
                            directory))
                     (with-current-buffer buffer
                       (dired-goto-file second)
                       (cl-letf
                           (((symbol-function
                              'arview-process-prefix-arg)
                             (lambda (argument)
                               (push
                                (list
                                 :prefix argument)
                                calls)
                               '("/chosen/"
                                 " --extra")))
                            ((symbol-function
                              'arview-view)
                             (lambda (&rest arguments)
                               (push
                                (cons
                                 :view arguments)
                                calls)
                               :opened)))
                         (list
                          (arview-dired '(16))
                          major-mode
                          (file-name-nondirectory
                           (dired-get-filename))
                          (nreverse calls)))))
                 (when
                     (buffer-live-p buffer)
                   (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (:opened dired-mode "space archive.zip" ((:prefix (16)) (:view "[ORACLE-SANDBOX]/dired-command/space archive.zip" "/chosen/" " --extra")))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_dired_outside_dired_mode_is_silent_and_does_not_resolve_filename() {
    let elisp_form = r##"(let (calls)
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function
                        'dired-get-filename)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           :filename arguments)
                          calls)
                         "/unexpected"))
                      ((symbol-function
                        'arview-process-prefix-arg)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           :prefix arguments)
                          calls)
                         nil))
                      ((symbol-function
                        'arview-view)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           :view arguments)
                          calls)
                         :unexpected)))
                   (list
                    major-mode
                    (arview-dired '(4))
                    calls))))"##;
    let expect = expect!["OK (fundamental-mode nil nil)"];
    assert_arview_parity(elisp_form, expect);
}
