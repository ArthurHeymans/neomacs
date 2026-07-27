use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn ahg_command_name_completion_parses_mercurial_debugcomplete_and_falls_back() {
    let elisp_form = r##"(let (calls outcome)
                      (cl-letf
                          (((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              (pcase (car arguments)
                                ("st"
                                 (insert "status\nstrip\n")
                                 0)
                                ("empty" 0)
                                (_ 1)))))
                        (setq outcome
                              (list
                               (ahg-complete-command-name "st")
                               (ahg-complete-command-name "empty")
                               (ahg-complete-command-name "unknown")))
                        (list outcome (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((("status " "strip ") ("empty ") ("unknown ")) (("debugcomplete" ("st")) ("debugcomplete" ("empty")) ("debugcomplete" ("unknown"))))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_command_completion_combines_command_options_help_and_real_filename_candidates() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (src (expand-file-name "src" root)))
                      (make-directory src t)
                      (with-temp-file (expand-file-name "alpha.el" src)
                        (insert "alpha"))
                      (with-temp-file (expand-file-name "alpine.txt" src)
                        (insert "alpine"))
                      (make-directory (expand-file-name "assets" src) t)
                      (cl-letf
                          (((symbol-function 'ahg-complete-command-name)
                            (lambda (command)
                              (list (concat command "-one ")
                                    (concat command "-two ")))))
                        (let ((default-directory root))
                          (list
                           (ahg-complete-command "st")
                           (ahg-complete-command "help lo")
                           (ahg-complete-command "status --rev")
                           (ahg-complete-command "status src/al")))))"##;
    let expect = expect![[
        r#"OK (("st-one " "st-two ") ("help lo-one " "help lo-two ") ("status --rev") ("status src/alpha.el" "status src/alpine.txt"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_arbitrary_command_interpolates_selected_files_and_builds_a_readable_header() {
    let elisp_form = r##"(let (calls buffers)
                      (cl-letf
                          (((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) "/repo/"))
                           ((symbol-function 'ahg-push-window-configuration)
                            (lambda () nil))
                           ((symbol-function 'pop-to-buffer)
                            (lambda (buffer &rest _rest)
                              (push (buffer-name (get-buffer buffer)) buffers)))
                           ((symbol-function 'ahg-generic-command)
                            (lambda (&rest arguments)
                              (push arguments calls)
                              'fake-process)))
                        (let ((default-directory "/repo/subdir/")
                              (ahg-do-command-show-buffer-immediately nil)
                              (ahg-do-command-insert-header t)
                              (ahg-do-command-extra-args
                               '("src/one.el" "docs/two file.md")))
                          (unwind-protect
                              (progn
                                (ahg-do-command "diff -r tip * --stat")
                                (ahg-do-command "status -m")
                                (let ((buffer (get-buffer "*hg command: /repo/*")))
                                  (list
                                   (mapcar
                                    (lambda (call)
                                      (list (nth 0 call)
                                            (nth 1 call)
                                            (nth 4 call)
                                            (nth 6 call)
                                            (functionp (nth 7 call))
                                            (nth 8 call)
                                            (nth 9 call)))
                                    (nreverse calls))
                                   (with-current-buffer buffer
                                     (buffer-substring-no-properties
                                      (point-min) (point-max)))
                                   (nreverse buffers))))
                            (when (get-buffer "*hg command: /repo/*")
                              (kill-buffer "*hg command: /repo/*"))))))"##;
    let expect = expect![[
        r#"OK ((("diff" ("-r tip src/one.el docs/two file.md --stat") t t t nil ("--config" "progress.assume-tty=True" "--config" "progress.clear-complete=False")) ("status" ("-m src/one.el docs/two file.md") t t t nil ("--config" "progress.assume-tty=True" "--config" "progress.clear-complete=False"))) "output of 'hg status -m src/one.el docs/two file.md' on /repo/subdir/\n-------------------------------------------------------------------------------\n\n" nil)"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_grep_regexp_quoting_preserves_literal_parentheses_and_complex_symbols() {
    let elisp_form = r##"(mapcar
                      (lambda (text)
                        (list text
                              (regexp-quote text)
                              (ahg-grep-regexp-quote text)))
                      '("plain"
                        "call(value)"
                        "a+b*c?"
                        "[λ] (one|two)"
                        "path/to/(file).el"))"##;
    let expect = expect![[
        r#"OK (("plain" "plain" "plain") ("call(value)" "call(value)" "call\\(value\\)") ("a+b*c?" "a\\+b\\*c\\?" "a\\+b\\*c\\?") ("[λ] (one|two)" "\\[λ] (one|two)" "\\[λ] \\(one|two\\)") ("path/to/(file).el" "path/to/(file)\\.el" "path/to/\\(file\\)\\.el"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_grep_file_setup_reads_text_unicode_large_chunks_and_rejects_binary_nuls() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (text (expand-file-name "text.txt" sandbox))
                          (binary (expand-file-name "binary.dat" sandbox))
                          (large (expand-file-name "large.txt" sandbox)))
                      (with-temp-file text
                        (insert "alpha λ\nbeta\n"))
                      (with-temp-file binary
                        (set-buffer-multibyte nil)
                        (insert "alpha\0beta\n"))
                      (with-temp-file large
                        (insert (make-string 1048580 ?x))
                        (insert "\nneedle\n"))
                      (mapcar
                       (lambda (file)
                         (with-temp-buffer
                           (let ((accepted
                                  (ahg-grep-filename-setup file)))
                             (list
                              accepted
                              (buffer-size)
                              (and accepted
                                   (save-excursion
                                     (goto-char (point-min))
                                     (search-forward "needle" nil t)))))))
                       (list text binary large)))"##;
    let expect = expect!["OK ((t 14 nil) (nil 22 nil) (t 1048588 12))"];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_elisp_grep_emits_every_matching_line_with_counts_and_match_properties() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (file (expand-file-name "repo/src/code.el" sandbox)))
                      (make-directory (file-name-directory file) t)
                      (with-temp-file file
                        (insert
                         "alpha needle one\n"
                         "no match\n"
                         "needle two and needle three\n"
                         "λ needle four\n"))
                      (with-temp-buffer
                        (let ((count (ahg-grep-filename file "needle")))
                          (goto-char (point-min))
                          (let (faces)
                            (while (search-forward "needle" nil t)
                              (push
                               (get-text-property (1- (point))
                                                  'font-lock-face)
                               faces))
                            (list count
                                  (buffer-substring-no-properties
                                   (point-min) (point-max))
                                  (nreverse faces))))))"##;
    let expect = expect![[
        r#"OK (4 "[ORACLE-SANDBOX]/repo/src/code.el:1:alpha needle one\n[ORACLE-SANDBOX]/repo/src/code.el:3:needle two and needle three\n[ORACLE-SANDBOX]/repo/src/code.el:4:\316\273 needle four\n" (match match match match))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_manifest_glob_reader_rebases_dot_paths_and_command_builder_quotes_globs() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (default-directory
                           (file-name-as-directory
                            (expand-file-name "src/lib" root)))
                          answers)
                      (make-directory default-directory t)
                      (cl-letf
                          (((symbol-function 'read-string)
                            (lambda (_prompt) (pop answers)))
                           ((symbol-function 'ahg-hg-command)
                            (lambda () "/opt/hg binary")))
                        (setq answers '("./*.el" "docs/**" ""))
                        (list
                         (ahg-manifest-grep-read root)
                         (ahg-manifest-grep-read root)
                         (ahg-manifest-grep-read root)
                         (ahg-manifest-grep-get-files nil)
                         (ahg-manifest-grep-get-files "*.el")
                         (ahg-manifest-grep-get-files "path with spaces/**"))))"##;
    let expect = expect![[
        r#"OK ("src/lib/*.el" "docs/**" "" "/opt/hg binary files -0 " "/opt/hg binary files -0 'glob:*.el'" "/opt/hg binary files -0 'glob:path with spaces/**'")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_shell_completion_expands_real_files_and_preserves_command_prefix() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (default-directory root))
                      (make-directory (expand-file-name "build-dir" root) t)
                      (with-temp-file (expand-file-name "build.log" root)
                        (insert "log"))
                      (with-temp-file (expand-file-name "bundle.zip" root)
                        (insert "zip"))
                      (list
                       (ahg-complete-shell-command "rm bu")
                       (ahg-complete-shell-command "printf x > build")
                       (ahg-complete-shell-command "echo missing")))"##;
    let expect = expect![[
        r#"OK (("rm build-dir" "rm build.log" "rm bundle.zip") ("printf x > build-dir" "printf x > build.log") nil)"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_process_wrappers_construct_exact_sync_and_async_mercurial_argv_and_restore_environment() {
    let elisp_form = r##"(let ((original-lang (getenv "LANG"))
                          (original-plain (getenv "HGPLAINEXCEPT"))
                          starts sync-calls sentinels filters codings)
                      (cl-letf
                          (((symbol-function 'process-file)
                            (lambda (&rest arguments)
                              (push arguments sync-calls)
                              37))
                           ((symbol-function 'start-file-process)
                            (lambda (&rest arguments)
                              (push arguments starts)
                              'fake-process))
                           ((symbol-function 'set-process-sentinel)
                            (lambda (process sentinel)
                              (push (list process (functionp sentinel))
                                    sentinels)))
                           ((symbol-function 'set-process-filter)
                            (lambda (process filter)
                              (push (list process filter) filters)))
                           ((symbol-function 'set-process-coding-system)
                            (lambda (&rest arguments)
                              (push arguments codings))))
                        (let ((ahg-hg-command "/opt/hg")
                              (ahg-i18n nil)
                              (ahg-subprocess-coding-system "utf-8"))
                          (list
                           (ahg-call-process
                            "log" '("-r" "tip") '("--traceback"))
                           (ahg-generic-command
                            "status" '("-m" "path with spaces")
                            #'ignore nil nil t nil #'ignore nil
                            '("--debug") nil)
                           (nreverse sync-calls)
                           (nreverse starts)
                           (nreverse sentinels)
                           (nreverse filters)
                           (nreverse codings)
                           (getenv "LANG")
                           (getenv "HGPLAINEXCEPT")
                           original-lang
                           original-plain))))"##;
    let expect = expect![[
        r#"OK (37 fake-process (("/opt/hg" nil t nil "--config" "ui.report_untrusted=0" "--traceback" "log" "-r" "tip")) (("*ahg-command-status*" (:buffer "*ahg-command*") "/opt/hg" "--config" "ui.report_untrusted=0" "--debug" "status" "-m" "path with spaces")) ((fake-process t)) ((fake-process ignore)) ((fake-process "utf-8")) "C.UTF-8" "alias,revsetalias" "C.UTF-8" nil)"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
