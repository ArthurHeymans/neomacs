use expect_test::expect;

use super::assert_ameba_autoload_parity;

#[test]
fn generated_autoloads_register_every_public_command_without_loading_the_feature() {
    let elisp_form = r##"(list
                      (featurep 'ameba)
                      (mapcar
                       (lambda (symbol)
                         (let ((definition
                                (symbol-function symbol)))
                           (list
                            symbol
                            (autoloadp definition)
                            (nth 1 definition)
                            (nth 2 definition)
                            (nth 3 definition)
                            (nth 4 definition))))
                       '(ameba-check-current-file
                         ameba-check-project
                         ameba-check-directory
                         ameba-mode))
                      (and
                       (member
                        (file-name-directory
                         (getenv "NEOMACS_PACKAGE_SOURCE"))
                        load-path)
                       t))"##;
    let expect = expect![[
        r#"OK (nil ((ameba-check-current-file t "ameba" "Run check on the current file." t nil) (ameba-check-project t "ameba" "Run check on the current project." t nil) (ameba-check-directory t "ameba" "Run check on the DIRECTORY if present or prompt user if not.\n\n(fn &optional DIRECTORY)" t nil) (ameba-mode t "ameba" "Minor mode to interface with Ameba.\n\nThis is a minor mode.  If called interactively, toggle the `Ameba mode'\nmode.  If the prefix argument is positive, enable the mode, and if it is\nzero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `ameba-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n(fn &optional ARG)" t nil)) nil)"#
    ]];
    assert_ameba_autoload_parity(elisp_form, expect);
}

#[test]
fn invoking_an_autoloaded_command_loads_the_exact_source_and_retains_command_identity() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (bin
                           (file-name-as-directory
                            (expand-file-name "autoload-bin" sandbox)))
                          (executable
                           (expand-file-name "ameba" bin))
                          (exec-path (cons bin exec-path))
                          (process-environment
                           (copy-sequence process-environment))
                          forwarded)
                      (make-directory bin t)
                      (with-temp-file executable
                        (insert "#!/bin/sh\nexit 0\n"))
                      (set-file-modes executable #o755)
                      (setenv "PATH"
                              (concat bin path-separator
                                      (getenv "PATH")))
                      (cl-letf
                          (((symbol-function 'compilation-start)
                            (lambda
                                (command mode name-function)
                              (setq forwarded
                                    (list
                                     command mode
                                     (funcall
                                      name-function
                                      "autoload-callback")))
                              'checked))
                           ((symbol-function 'message)
                            (lambda (&rest _) nil)))
                        (list
                         (ameba-check-directory "/workspace/app/")
                         forwarded
                         (featurep 'ameba)
                         (autoloadp
                          (symbol-function
                           'ameba-check-directory))
                         (commandp 'ameba-check-directory)
                         (file-name-nondirectory
                          (symbol-file
                           'ameba-check-directory 'defun))
                         (featurep 'ameba))))"##;
    let expect = expect![[
        r#"OK (checked ("ameba --format flycheck /workspace/app/" compilation-mode "*Ameba /workspace/app/*") t nil t "ameba.el" t)"#
    ]];
    assert_ameba_autoload_parity(elisp_form, expect);
}
