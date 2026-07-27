use expect_test::expect;

use super::assert_ameba_parity;

#[test]
fn installation_guard_accepts_a_workspace_local_executable_and_rejects_a_missing_binary() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin)))
                      (make-directory bin t)
                      (with-temp-file executable
                        (insert "#!/bin/sh\nexit 0\n"))
                      (set-file-modes executable #o755)
                      (let ((exec-path (list bin))
                            (process-environment
                             (copy-sequence process-environment)))
                        (setenv "PATH" bin)
                        (list
                         (ameba-ensure-installed)
                         (file-relative-name
                          (executable-find "ameba") sandbox)
                         (let ((exec-path nil))
                           (setenv "PATH" "")
                           (condition-case error-data
                               (ameba-ensure-installed)
                             (error
                              (list
                               (car error-data)
                               (cdr error-data))))))))"##;
    let expect = expect![[r#"OK (nil "bin/ameba" (error ("Ameba is not installed")))"#]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn current_file_command_runs_from_project_root_and_builds_compilation_identity() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (project
                           (file-name-as-directory
                            (expand-file-name "shop" sandbox)))
                          (source
                           (expand-file-name "src/cart.cr" project))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin))
                          (exec-path (cons bin exec-path))
                          (process-environment
                           (copy-sequence process-environment))
                          (ameba-project-root-files
                           '(".ameba.yml"))
                          events)
                      (make-directory
                       (file-name-directory source) t)
                      (make-directory bin t)
                      (with-temp-file
                          (expand-file-name ".ameba.yml" project)
                        (insert "Lint/Formatting:\n  Enabled: true\n"))
                      (with-temp-file source
                        (insert "class Cart\nend\n"))
                      (with-temp-file executable
                        (insert "#!/bin/sh\nexit 0\n"))
                      (set-file-modes executable #o755)
                      (setenv "PATH"
                              (concat bin path-separator
                                      (getenv "PATH")))
                      (with-temp-buffer
                        (setq buffer-file-name source
                              default-directory
                              (file-name-directory source))
                        (cl-letf
                            (((symbol-function 'compilation-start)
                              (lambda
                                  (command mode name-function)
                                (setq events
                                      (list
                                       command mode
                                       (funcall
                                        name-function
                                        "ignored")
                                       (file-relative-name
                                        default-directory
                                        sandbox)))
                                'compilation-buffer)))
                          (list
                           (ameba-check-current-file)
                           events))))"##;
    let expect = expect![[
        r#"OK (compilation-buffer ("ameba --format flycheck [ORACLE-SANDBOX]/shop/src/cart.cr" compilation-mode "*Ameba [ORACLE-SANDBOX]/shop/src/cart.cr*" "shop/"))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn current_file_command_falls_back_to_its_buffer_directory_outside_a_project() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (source
                           (expand-file-name "loose/check.cr" sandbox))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin))
                          (exec-path (cons bin exec-path))
                          (process-environment
                           (copy-sequence process-environment))
                          (ameba-project-root-files
                           '("marker-that-does-not-exist"))
                          event)
                      (make-directory
                       (file-name-directory source) t)
                      (make-directory bin t)
                      (with-temp-file source
                        (insert "puts :loose\n"))
                      (with-temp-file executable
                        (insert "#!/bin/sh\nexit 0\n"))
                      (set-file-modes executable #o755)
                      (setenv "PATH"
                              (concat bin path-separator
                                      (getenv "PATH")))
                      (with-temp-buffer
                        (setq buffer-file-name source
                              default-directory
                              (file-name-directory source))
                        (cl-letf
                            (((symbol-function 'compilation-start)
                              (lambda
                                  (command mode name-function)
                                (setq event
                                      (list
                                       command mode
                                       (funcall
                                        name-function nil)
                                       (file-relative-name
                                        default-directory
                                        sandbox)))
                                'started)))
                          (list
                           (ameba-check-current-file)
                           event))))"##;
    let expect = expect![[
        r#"OK (started ("ameba --format flycheck [ORACLE-SANDBOX]/loose/check.cr" compilation-mode "*Ameba [ORACLE-SANDBOX]/loose/check.cr*" "loose/"))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn current_file_command_reports_missing_visit_after_the_installation_guard() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin))
                          (exec-path (list bin))
                          (process-environment
                           (copy-sequence process-environment)))
                      (make-directory bin t)
                      (with-temp-file executable
                        (insert "#!/bin/sh\nexit 0\n"))
                      (set-file-modes executable #o755)
                      (setenv "PATH" bin)
                      (with-temp-buffer
                        (condition-case error-data
                            (ameba-check-current-file)
                          (error
                           (list
                            (car error-data)
                            (cdr error-data))))))"##;
    let expect = expect![[r#"OK (error ("Buffer is not visiting a file"))"#]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn missing_installation_error_precedes_the_nonvisiting_buffer_error() {
    let elisp_form = r##"(let ((exec-path nil)
                          (process-environment
                           (copy-sequence process-environment)))
                      (setenv "PATH" "")
                      (with-temp-buffer
                        (condition-case error-data
                            (ameba-check-current-file)
                          (error
                           (list
                            (car error-data)
                            (cdr error-data))))))"##;
    let expect = expect![[r#"OK (error ("Ameba is not installed"))"#]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn directory_command_uses_explicit_target_project_root_and_callback_message() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (project
                           (file-name-as-directory
                            (expand-file-name "monorepo" sandbox)))
                          (nested
                           (file-name-as-directory
                            (expand-file-name "src/deep" project)))
                          (target
                           (file-name-as-directory
                            (expand-file-name "components/api" project)))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin))
                          (default-directory nested)
                          (exec-path (cons bin exec-path))
                          (process-environment
                           (copy-sequence process-environment))
                          (ameba-project-root-files
                           '("shard.yml"))
                          event messages)
                      (make-directory nested t)
                      (make-directory target t)
                      (make-directory bin t)
                      (with-temp-file
                          (expand-file-name "shard.yml" project)
                        (insert "name: monorepo\n"))
                      (with-temp-file executable
                        (insert "#!/bin/sh\nexit 0\n"))
                      (set-file-modes executable #o755)
                      (setenv "PATH"
                              (concat bin path-separator
                                      (getenv "PATH")))
                      (cl-letf
                          (((symbol-function 'message)
                            (lambda (format-string &rest arguments)
                              (push
                               (apply #'format
                                      format-string arguments)
                               messages)))
                           ((symbol-function 'compilation-start)
                            (lambda
                                (command mode name-function)
                              (setq event
                                    (list
                                     command mode
                                     (funcall
                                      name-function
                                      "callback-value")
                                     (file-relative-name
                                      default-directory sandbox)))
                              'started)))
                        (list
                         (ameba-check-directory target)
                         event
                         (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (started ("ameba --format flycheck [ORACLE-SANDBOX]/monorepo/components/api/" compilation-mode "*Ameba [ORACLE-SANDBOX]/monorepo/components/api/*" "monorepo/") ("callback-value"))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn directory_command_prompts_once_and_falls_back_to_current_directory_without_project() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (current
                           (file-name-as-directory
                            (expand-file-name "outside" sandbox)))
                          (target
                           (file-name-as-directory
                            (expand-file-name "selected" sandbox)))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin))
                          (default-directory current)
                          (exec-path (cons bin exec-path))
                          (process-environment
                           (copy-sequence process-environment))
                          (ameba-project-root-files
                           '("marker-that-does-not-exist"))
                          prompts event)
                      (make-directory current t)
                      (make-directory target t)
                      (make-directory bin t)
                      (with-temp-file executable
                        (insert "#!/bin/sh\nexit 0\n"))
                      (set-file-modes executable #o755)
                      (setenv "PATH"
                              (concat bin path-separator
                                      (getenv "PATH")))
                      (cl-letf
                          (((symbol-function 'read-directory-name)
                            (lambda (prompt &rest _)
                              (push prompt prompts)
                              target))
                           ((symbol-function 'message)
                            (lambda (&rest _) nil))
                           ((symbol-function 'compilation-start)
                            (lambda
                                (command mode name-function)
                              (setq event
                                    (list
                                     command mode
                                     (funcall
                                      name-function
                                      "callback")
                                     (file-relative-name
                                      default-directory sandbox)))
                              'started)))
                        (list
                         (ameba-check-directory)
                         event
                         (nreverse prompts))))"##;
    let expect = expect![[
        r#"OK (started ("ameba --format flycheck [ORACLE-SANDBOX]/selected/" compilation-mode "*Ameba [ORACLE-SANDBOX]/selected/*" "outside/") ("Select directory: "))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn public_commands_forward_custom_configuration_and_project_include_exclude_targets() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (project
                           (file-name-as-directory
                            (expand-file-name "application" sandbox)))
                          (nested
                           (file-name-as-directory
                            (expand-file-name "src/deep" project)))
                          (default-directory nested)
                          (ameba-check-command
                           "bundle exec ameba --format flycheck")
                          (ameba-project-root-files
                           '(".ameba.yml"))
                          events)
                      (make-directory nested t)
                      (with-temp-file
                          (expand-file-name ".ameba.yml" project)
                        (insert "Globs:\n  - src/**/*.cr\n"))
                      (cl-letf
                          (((symbol-function 'ameba--file-command)
                            (lambda (command)
                              (push
                               (list 'file command)
                               events)
                              'file-checked))
                           ((symbol-function 'ameba--dir-command)
                            (lambda (command &optional directory)
                              (push
                               (list
                                'directory command
                                (and directory
                                     (replace-regexp-in-string
                                      (regexp-quote sandbox)
                                      "[SANDBOX]/"
                                      directory t t)))
                               events)
                              'directory-checked)))
                        (list
                         (ameba-check-current-file)
                         (ameba-check-directory
                          "/workspace/selected/")
                         (ameba-check-project)
                         (nreverse events))))"##;
    let expect = expect![[
        r#"OK (file-checked directory-checked directory-checked ((file "bundle exec ameba --format flycheck") (directory "bundle exec ameba --format flycheck" "/workspace/selected/") (directory "bundle exec ameba --format flycheck" "[SANDBOX]/application/ ![SANDBOX]/application/lib")))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}
