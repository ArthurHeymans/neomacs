use expect_test::expect;

use super::assert_arview_parity;

#[test]
fn arview_process_file_local_builds_exact_shell_command_and_process_call() {
    let elisp_form = r##"(with-temp-buffer
               (let ((default-directory
                       (arview-test-path
                        "process-working/"))
                     (shell-file-name
                      "/fixture/shell")
                     call)
                 (make-directory
                  default-directory t)
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
                         23)))
                   (list
                    (arview-process-file
                     "tar"
                     "-xf --warning=no-all"
                     "../archive space.tar"
                     (current-buffer))
                    call))))"##;
    let expect = expect![[
        r#"OK (23 ("/fixture/shell" nil t nil ("-c" "tar -xf --warning=no-all [ORACLE-SANDBOX]/archive\\ space.tar") "[ORACLE-SANDBOX]/process-working/"))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_process_file_local_quotes_spaces_quotes_and_shell_metacharacters_in_filename() {
    let elisp_form = r##"(let ((shell-file-name
                    "/fixture/shell")
                   calls)
               (cl-letf
                   (((symbol-function
                      'process-file)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       0)))
                 (mapcar
                  (lambda (filename)
                    (list
                     filename
                     (arview-process-file
                      "extractor"
                      "--mode exact"
                      filename
                      nil)
                     (car calls)))
                  '("/work/plain.tar"
                    "/work/space name.tar"
                    "/work/quote'file.tar"
                    "/work/dollar$semi;pipe|.tar"
                    "/work/資料 λ.tar"))))"##;
    let expect = expect![[
        r#"OK (("/work/plain.tar" 0 ("/fixture/shell" nil nil nil "-c" "extractor --mode exact /work/plain.tar")) ("/work/space name.tar" 0 ("/fixture/shell" nil nil nil "-c" "extractor --mode exact /work/space\\ name.tar")) ("/work/quote'file.tar" 0 ("/fixture/shell" nil nil nil "-c" "extractor --mode exact /work/quote\\'file.tar")) ("/work/dollar$semi;pipe|.tar" 0 ("/fixture/shell" nil nil nil "-c" "extractor --mode exact /work/dollar\\$semi\\;pipe\\|.tar")) ("/work/資料 λ.tar" 0 ("/fixture/shell" nil nil nil "-c" "extractor --mode exact /work/資料\\ λ.tar")))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_process_file_real_printf_writes_expanded_archive_path_to_log() {
    let elisp_form = r##"(let* ((directory
                    (arview-test-path
                     "real-process/"))
                   (file
                    (expand-file-name
                     "archive space.tar"
                     directory))
                   (log
                    (get-buffer-create
                     " *arview-real-process-log*")))
               (make-directory directory t)
               (with-current-buffer log
                 (erase-buffer))
               (let ((default-directory
                       directory))
                 (list
                  (arview-process-file
                   "printf"
                   "%s"
                   "archive space.tar"
                   log)
                  (with-current-buffer log
                    (buffer-string))
                  (expand-file-name file)
                  default-directory)))"##;
    let expect = expect![[
        r#"OK (0 "[ORACLE-SANDBOX]/real-process/archive space.tar" "[ORACLE-SANDBOX]/real-process/archive space.tar" "[ORACLE-SANDBOX]/real-process/")"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_process_file_propagates_real_nonzero_exit_status_and_output() {
    let elisp_form = r##"(let* ((file
                    (arview-test-path
                     "unused.tar"))
                   (log
                    (get-buffer-create
                     " *arview-failing-process-log*")))
               (with-current-buffer log
                 (erase-buffer))
               (list
                (arview-process-file
                 "sh"
                 "-c 'printf failure-message; exit 7' --"
                 file
                 log)
                (with-current-buffer log
                  (buffer-string))
                (buffer-live-p log)))"##;
    let expect = expect![[r#"OK (7 "failure-message" t)"#]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_process_file_remote_builds_configured_shell_and_localname_command() {
    let elisp_form = r##"(let (call)
               (cl-letf
                   (((symbol-function
                      'tramp-get-method-parameter)
                     (lambda (method parameter)
                       (push
                        (list
                         :parameter method
                         parameter)
                        call)
                       (pcase parameter
                         ('tramp-remote-shell
                          "/fixture/remote-shell")
                         ('tramp-remote-shell-args
                          '("-lc" "fixture"))
                         (_
                          (error
                           "unexpected parameter")))))
                    ((symbol-function
                      'process-file)
                     (lambda
                       (program infile destination
                                display &rest arguments)
                       (push
                        (list
                         :process
                         program
                         infile
                         destination
                         display
                         arguments)
                        call)
                       9)))
                 (list
                  (arview-process-file
                   "tar"
                   "-xf"
                   "/ssh:host:/remote/path/archive space.tar"
                   'fixture-log)
                  (nreverse call))))"##;
    let expect = expect![[
        r#"OK (9 ((:parameter "ssh" tramp-remote-shell) (:parameter "ssh" tramp-remote-shell-args) (:process "/fixture/remote-shell" nil fixture-log nil ("-lc" "fixture" "tar -xf /remote/path/archive\\ space.tar"))))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_process_file_does_not_mutate_shell_quote_configuration() {
    let elisp_form = r##"(let ((shell-file-name-quote-list
                    '(42 43))
                   (comint-file-name-quote-list
                    '(99))
                   inside)
               (cl-letf
                   (((symbol-function
                      'process-file)
                     (lambda (&rest _)
                       (setq inside
                             (list
                              shell-file-name-quote-list
                              comint-file-name-quote-list))
                       0)))
                 (list
                  (arview-process-file
                   "extract"
                   "--flag"
                   "/work/file.tar"
                   nil)
                  inside
                  shell-file-name-quote-list
                  comint-file-name-quote-list)))"##;
    let expect = expect!["OK (0 (#1=(42 43) #1#) #1# (99))"];
    assert_arview_parity(elisp_form, expect);
}
