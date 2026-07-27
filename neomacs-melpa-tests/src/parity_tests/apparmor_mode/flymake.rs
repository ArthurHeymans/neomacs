use expect_test::expect;

use super::assert_apparmor_mode_parity;

#[test]
fn apparmor_mode_parser_resolver_checks_path_then_standard_fallbacks() {
    let elisp_form = r##"(let (executable-calls file-calls)
         (cl-labels
             ((resolve
               (requested executable-result executable-path)
               (let ((apparmor-mode-apparmor-parser-executable requested))
                 (cl-letf
                     (((symbol-function 'executable-find)
                       (lambda (name)
                         (push name executable-calls)
                         executable-result))
                      ((symbol-function 'file-executable-p)
                       (lambda (path)
                         (push path file-calls)
                         (equal path executable-path))))
                   (apparmor-mode-get-apparmor-parser-executable-path)))))
           (list
            (resolve "custom-parser" "/tools/custom-parser" nil)
            (resolve "missing-parser" nil "/sbin/apparmor_parser")
            (resolve "absent-parser" nil nil)
            (nreverse executable-calls)
            (nreverse file-calls))))"##;
    let expect = expect![[
        r#"OK ("/tools/custom-parser" "/sbin/apparmor_parser" nil ("custom-parser" "missing-parser" "absent-parser") ("/usr/local/sbin/apparmor_parser" "/sbin/apparmor_parser" "/usr/local/sbin/apparmor_parser" "/sbin/apparmor_parser" "/usr/sbin/apparmor_parser"))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_flymake_setup_is_buffer_local_and_idempotent() {
    let elisp_form = r##"(let ((global-before
                (default-value 'flymake-diagnostic-functions)))
         (list
          (with-temp-buffer
            (apparmor-mode-setup-flymake-backend)
            (apparmor-mode-setup-flymake-backend)
            (list
             (local-variable-p 'flymake-diagnostic-functions)
             (cl-count #'apparmor-mode-flymake
                       flymake-diagnostic-functions)
             flymake-diagnostic-functions))
          (with-temp-buffer
            (apparmor-mode)
            (list
             (local-variable-p 'flymake-diagnostic-functions)
             (cl-count #'apparmor-mode-flymake
                       flymake-diagnostic-functions)))
          (equal global-before
                 (default-value
                  'flymake-diagnostic-functions))))"##;
    let expect = expect!["OK ((t 1 (apparmor-mode-flymake t)) (t 1) t)"];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_flymake_runs_parser_and_reports_real_diagnostic() {
    let elisp_form = r##"(require 'flymake)
(let* ((fixture-dir
        (file-name-as-directory
         (expand-file-name "apparmor-flymake/" (getenv "HOME"))))
       (parser (expand-file-name "fake-apparmor-parser" fixture-dir))
       (capture (expand-file-name "captured-policy" fixture-dir))
       reports)
  (make-directory fixture-dir t)
  (with-temp-file parser
    (insert "#!/bin/sh\n"
            "cat > " (shell-quote-argument capture) "\n"
            "printf '%s\\n' 'AppArmor parser error at line 2: bad rule'\n"
            "exit 1\n"))
  (set-file-modes parser #o755)
  (with-temp-buffer
    (let ((apparmor-mode-apparmor-parser-executable parser))
      (apparmor-mode)
      (insert "profile demo /usr/bin/demo {\n  broken rule,\n}\n")
      (apparmor-mode-flymake
       (lambda (diagnostics)
         (setq reports diagnostics)))
      (while (process-live-p apparmor-mode--flymake-proc)
        (accept-process-output apparmor-mode--flymake-proc 0.05))
      (accept-process-output nil 0.05)
      (list
       (mapcar
        (lambda (diagnostic)
          (list
           (flymake-diagnostic-beg diagnostic)
           (flymake-diagnostic-end diagnostic)
           (flymake-diagnostic-type diagnostic)
           (flymake-diagnostic-text diagnostic)))
        reports)
       (with-temp-buffer
         (insert-file-contents capture)
         (buffer-string))
       (process-status apparmor-mode--flymake-proc)
       (buffer-live-p
        (process-buffer apparmor-mode--flymake-proc))))))"##;
    let expect = expect![[
        r#"OK (((32 44 :error "bad rule")) "profile demo /usr/bin/demo {\n  broken rule,\n}\n" exit nil)"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_flymake_wraps_local_include_as_synthetic_profile() {
    let elisp_form = r##"(require 'flymake)
(let* ((fixture-dir
        (file-name-as-directory
         (expand-file-name "apparmor-flymake-local/" (getenv "HOME"))))
       (parser (expand-file-name "fake-apparmor-parser" fixture-dir))
       (capture (expand-file-name "captured-policy" fixture-dir))
       reports)
  (make-directory fixture-dir t)
  (with-temp-file parser
    (insert "#!/bin/sh\n"
            "cat > " (shell-quote-argument capture) "\n"
            "exit 0\n"))
  (set-file-modes parser #o755)
  (with-temp-buffer
    (let ((apparmor-mode-apparmor-parser-executable parser))
      (setq buffer-file-name
            (expand-file-name "abstractions/base" fixture-dir))
      (rename-buffer "local-policy" t)
      (apparmor-mode)
      (insert "/etc/ssl/certs/** r,\n")
      (apparmor-mode-flymake
       (lambda (diagnostics)
         (setq reports diagnostics)))
      (while (process-live-p apparmor-mode--flymake-proc)
        (accept-process-output apparmor-mode--flymake-proc 0.05))
      (accept-process-output nil 0.05)
      (list
       reports
       (with-temp-buffer
         (insert-file-contents capture)
         (buffer-string))
       (process-status apparmor-mode--flymake-proc)
       (buffer-live-p
        (process-buffer apparmor-mode--flymake-proc))))))"##;
    let expect =
        expect![[r#"OK (nil "profile local-policy { /etc/ssl/certs/** r,\n }" exit nil)"#]];
    assert_apparmor_mode_parity(elisp_form, expect);
}
