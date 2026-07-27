use expect_test::expect;

use super::assert_ameba_parity;

#[test]
fn real_current_file_compilation_invokes_workspace_local_ameba_and_captures_diagnostic_output() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (project
                           (file-name-as-directory
                            (expand-file-name "real-file-project" sandbox)))
                          (source
                           (expand-file-name "src/cart.cr" project))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin))
                          (arguments
                           (expand-file-name "file-arguments" sandbox))
                          (working-directory
                           (expand-file-name "file-cwd" sandbox))
                          (exec-path (cons bin exec-path))
                          (process-environment
                           (copy-sequence process-environment))
                          (ameba-project-root-files
                           '(".ameba.yml"))
                          compilation-buffer)
                      (make-directory
                       (file-name-directory source) t)
                      (make-directory bin t)
                      (with-temp-file
                          (expand-file-name ".ameba.yml" project)
                        (insert "Lint/Formatting:\n  Enabled: true\n"))
                      (with-temp-file source
                        (insert
                         "class Cart\n"
                         "  def total; 42; end\n"
                         "end\n"))
                      (with-temp-file executable
                        (insert
                         "#!/bin/sh\n"
                         "printf '%s\\n' \"$@\" > \"$NEOMACS_TEST_SANDBOX_ROOT/file-arguments\"\n"
                         "pwd > \"$NEOMACS_TEST_SANDBOX_ROOT/file-cwd\"\n"
                         "printf 'src/cart.cr:2:3: W: Style/Parity: deterministic warning\\n'\n"))
                      (set-file-modes executable #o755)
                      (setenv "PATH"
                              (concat bin path-separator
                                      (getenv "PATH")))
                      (unwind-protect
                          (with-temp-buffer
                            (setq buffer-file-name source
                                  default-directory
                                  (file-name-directory source))
                            (setq compilation-buffer
                                  (ameba-check-current-file))
                            (while
                                (get-buffer-process
                                 compilation-buffer)
                              (accept-process-output
                               (get-buffer-process
                                compilation-buffer)
                               0.1))
                            (list
                             (buffer-name compilation-buffer)
                             (with-current-buffer compilation-buffer
                               (list
                                major-mode
                                (file-relative-name
                                 default-directory sandbox)
                                (and
                                 (string-match-p
                                  "Style/Parity: deterministic warning"
                                  (buffer-string))
                                 t)
                                (and
                                 (string-match-p
                                  "finished"
                                  (buffer-string))
                                 t)))
                             (with-temp-buffer
                               (insert-file-contents arguments)
                               (split-string
                                (buffer-string) "\n" t))
                             (with-temp-buffer
                               (insert-file-contents working-directory)
                               (file-relative-name
                                (string-trim
                                 (buffer-string))
                                sandbox))))
                        (when (buffer-live-p compilation-buffer)
                          (kill-buffer compilation-buffer))))"##;
    let expect = expect![[
        r#"OK ("*Ameba [ORACLE-SANDBOX]/real-file-project/src/cart.cr*" (compilation-mode "real-file-project/" t t) ("--format" "flycheck" "[ORACLE-SANDBOX]/real-file-project/src/cart.cr") "real-file-project")"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn real_project_compilation_passes_root_and_lib_exclusion_and_reports_nonzero_exit() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (project
                           (file-name-as-directory
                            (expand-file-name "real-project" sandbox)))
                          (nested
                           (file-name-as-directory
                            (expand-file-name "src/services" project)))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin))
                          (arguments
                           (expand-file-name
                            "project-arguments" sandbox))
                          (working-directory
                           (expand-file-name "project-cwd" sandbox))
                          (default-directory nested)
                          (exec-path (cons bin exec-path))
                          (process-environment
                           (copy-sequence process-environment))
                          (ameba-project-root-files
                           '("shard.yml"))
                          compilation-buffer)
                      (make-directory nested t)
                      (make-directory
                       (expand-file-name "lib" project) t)
                      (make-directory bin t)
                      (with-temp-file
                          (expand-file-name "shard.yml" project)
                        (insert "name: real_project\n"))
                      (with-temp-file executable
                        (insert
                         "#!/bin/sh\n"
                         "printf '%s\\n' \"$@\" > \"$NEOMACS_TEST_SANDBOX_ROOT/project-arguments\"\n"
                         "pwd > \"$NEOMACS_TEST_SANDBOX_ROOT/project-cwd\"\n"
                         "printf 'src/services/billing.cr:7:9: E: deterministic failure\\n'\n"
                         "exit 7\n"))
                      (set-file-modes executable #o755)
                      (setenv "PATH"
                              (concat bin path-separator
                                      (getenv "PATH")))
                      (unwind-protect
                          (progn
                            (setq compilation-buffer
                                  (ameba-check-project))
                            (while
                                (get-buffer-process
                                 compilation-buffer)
                              (accept-process-output
                               (get-buffer-process
                                compilation-buffer)
                               0.1))
                            (list
                             (buffer-name compilation-buffer)
                             (with-current-buffer compilation-buffer
                               (list
                                major-mode
                                (file-relative-name
                                 default-directory sandbox)
                                (and
                                 (string-match-p
                                  "deterministic failure"
                                  (buffer-string))
                                 t)
                                (and
                                 (string-match-p
                                  "exited abnormally with code 7"
                                  (buffer-string))
                                 t)))
                             (with-temp-buffer
                               (insert-file-contents arguments)
                               (mapcar
                                (lambda (argument)
                                  (if
                                      (string-prefix-p
                                       sandbox argument)
                                      (file-relative-name
                                       argument sandbox)
                                    argument))
                                (split-string
                                 (buffer-string) "\n" t)))
                             (with-temp-buffer
                               (insert-file-contents working-directory)
                               (file-relative-name
                                (string-trim
                                 (buffer-string))
                                sandbox))))
                        (when (buffer-live-p compilation-buffer)
                          (kill-buffer compilation-buffer))))"##;
    let expect = expect![[
        r#"OK ("*Ameba [ORACLE-SANDBOX]/real-project/ ![ORACLE-SANDBOX]/real-project/lib*" (compilation-mode "real-project/" t t) ("--format" "flycheck" "real-project/" "![ORACLE-SANDBOX]/real-project/lib") "real-project")"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}
