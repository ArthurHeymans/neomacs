use expect_test::expect;

use super::{assert_alert_toast_parity, assert_alert_toast_with_prelude_parity};

#[test]
fn wsl_detection_covers_linux_case_insensitive_markers_other_kernels_and_platforms() {
    let elisp_form = r##"
(mapcar
 (lambda (case)
   (let ((system-type (car case))
         calls)
     (cl-letf
         (((symbol-function 'shell-command-to-string)
           (lambda (command)
             (push command calls)
             (cadr case))))
       (list
        case
        (and (alert-toast--check-wsl) t)
        (nreverse calls)))))
 '((gnu/linux "5.15.90.1-microsoft-standard-WSL2\n")
   (gnu/linux "4.4.0-MICROSOFT\n")
   (gnu/linux "6.8.0-generic\n")
   (windows-nt "microsoft\n")
   (cygwin "wsl\n")
   (darwin "microsoft\n")))
"##;
    let expect = expect![[
        r#"OK (((gnu/linux "5.15.90.1-microsoft-standard-WSL2\n") t ("uname --kernel-release")) ((gnu/linux "4.4.0-MICROSOFT\n") t ("uname --kernel-release")) ((gnu/linux "6.8.0-generic\n") nil ("uname --kernel-release")) ((windows-nt "microsoft\n") nil nil) ((cygwin "wsl\n") nil nil) ((darwin "microsoft\n") nil nil))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn application_data_discovery_sends_exact_powershell_stdin_and_chomps_dos_output() {
    let elisp_form = r##"
(let (calls)
  (cl-letf
      (((symbol-function 'call-process-region)
        (lambda
          (start end program delete destination display
                 &rest arguments)
          (push
           (list
            (if
                (stringp start)
                start
              (buffer-substring-no-properties start end))
            program delete destination display arguments
            coding-system-for-read)
           calls)
          (insert
           "C:\\Users\\Tester\\AppData\\Local\\Emacs-Toast\\Emacs.png\r\n")
          0)))
    (list
     (alert-toast--appdir)
     (nreverse calls))))
"##;
    let expect = expect![[
        r#"OK ("C:\\Users\\Tester\\AppData\\Local\\Emacs-Toast\\Emacs.png" (("[System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData) | Join-Path -ChildPath Emacs-Toast\\Emacs.png" "powershell.exe" nil t nil ("-noprofile" "-NonInteractive" "-WindowStyle" "Hidden" "-Command" "-") utf-8-dos)))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn default_wsl_icon_path_converts_discovered_windows_path_through_wslpath() {
    let elisp_form = r##"
(let (calls)
  (cl-letf
      (((symbol-function 'alert-toast--appdir)
        (lambda ()
          "C:\\Users\\Tester\\AppData\\Local\\Emacs-Toast\\Emacs.png"))
       ((symbol-function 'call-process)
        (lambda
          (program infile destination display
                   &rest arguments)
          (push
           (list program infile destination display arguments)
           calls)
          (insert
           "/mnt/c/Users/Tester/AppData/Local/Emacs-Toast/Emacs.png\n")
          0)))
    (list
     (alert-toast--default-wsl-icon-path)
     (nreverse calls))))
"##;
    let expect = expect![[
        r#"OK ("/mnt/c/Users/Tester/AppData/Local/Emacs-Toast/Emacs.png" (("wslpath" nil t nil ("C:\\Users\\Tester\\AppData\\Local\\Emacs-Toast\\Emacs.png"))))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn icon_path_handles_native_wsl_and_cygwin_conversions_at_exact_process_boundaries() {
    let elisp_form = r##"
(mapcar
 (lambda (case)
   (let ((system-type (car case))
         (alert-toast--wsl (cadr case))
         calls)
     (cl-letf
         (((symbol-function 'call-process)
           (lambda
             (program infile destination display
                      &rest arguments)
             (push
              (list
               program infile destination display arguments)
              calls)
             (insert (cadddr case))
             0)))
       (list
        (car case)
        (cadr case)
        (alert-toast--icon-path (caddr case))
        (nreverse calls)))))
 '((gnu/linux nil "/home/user/icon.png" "unused\n")
   (gnu/linux t "/mnt/c/Icons/toast image.png"
              "C:/Icons/toast image.png\n")
   (cygwin nil "/cygdrive/c/Icons/toast.png"
           "C:\\Icons\\toast.png\r\n")
   (windows-nt nil "C:\\Icons\\toast.png" "unused\n")))
"##;
    let expect = expect![[
        r#"OK ((gnu/linux nil "/home/user/icon.png" nil) (gnu/linux t "C:/Icons/toast image.png" (("wslpath" nil t nil ("-m" "/mnt/c/Icons/toast image.png")))) (cygwin nil "C:\\Icons\\toast.png" (("cygpath.exe" nil t nil ("-w" "/cygdrive/c/Icons/toast.png")))) (windows-nt nil "C:\\Icons\\toast.png" nil))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn init_wsl_icon_creates_real_sandbox_directories_copies_once_and_is_idempotent() {
    let elisp_form = r##"
(let* ((root (make-temp-file "alert-toast-icon-" t))
       (data-root
        (file-name-as-directory
         (expand-file-name "emacs-data" root)))
       (source
        (expand-file-name
         "images/icons/hicolor/128x128/apps/emacs.png"
         data-root))
       (target
        (expand-file-name
         "windows/AppData/Local/Emacs-Toast/Emacs.png"
         root))
       (data-directory data-root))
  (unwind-protect
      (progn
        (make-directory (file-name-directory source) t)
        (with-temp-file source
          (set-buffer-multibyte nil)
          (insert "PNG-FIRST"))
        (cl-letf
            (((symbol-function
               'alert-toast--default-wsl-icon-path)
              (lambda () target)))
          (alert-toast--init-wsl-icon)
          (let ((first
                 (with-temp-buffer
                   (set-buffer-multibyte nil)
                   (insert-file-contents-literally target)
                   (buffer-string))))
            (with-temp-file source
              (set-buffer-multibyte nil)
              (insert "PNG-CHANGED"))
            (alert-toast--init-wsl-icon)
            (list
             first
             (with-temp-buffer
               (set-buffer-multibyte nil)
               (insert-file-contents-literally target)
               (buffer-string))
             (file-directory-p
              (file-name-directory target))
             (file-exists-p target)
             (file-attribute-size
              (file-attributes target))))))
    (delete-directory root t)))
"##;
    let expect = expect![[r#"OK ("PNG-FIRST" "PNG-FIRST" t t 9)"#]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn init_wsl_icon_surfaces_missing_bundled_source_without_leaving_target_file() {
    let elisp_form = r##"
(let* ((root (make-temp-file "alert-toast-missing-icon-" t))
       (data-directory
        (file-name-as-directory
         (expand-file-name "missing-data" root)))
       (target
        (expand-file-name "windows/Emacs.png" root))
       outcome)
  (unwind-protect
      (cl-letf
          (((symbol-function
             'alert-toast--default-wsl-icon-path)
            (lambda () target)))
        (condition-case error-data
            (alert-toast--init-wsl-icon)
          (error
           (setq outcome
                 (list
                  (car error-data)
                  (file-exists-p target)
                  (file-directory-p
                   (file-name-directory target))))))
        outcome)
    (delete-directory root t)))
"##;
    let expect = expect!["OK (file-missing t t)"];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn simulated_wsl_source_initialization_discovers_default_icon_and_skips_existing_copy() {
    let prelude = r##"
(require 'f)
(defvar alert-toast-test-boundary-calls nil)
(fset 'shell-command-to-string
      (lambda (command)
        (push (list 'shell command)
              alert-toast-test-boundary-calls)
        "5.15.90.1-microsoft-standard-WSL2\n"))
(fset 'call-process-region
      (lambda
        (start end program delete destination display
               &rest arguments)
        (push
         (list
          'region
          (if
              (stringp start)
              start
            (buffer-substring-no-properties start end))
          program delete destination display arguments)
         alert-toast-test-boundary-calls)
        (insert
         "C:\\Users\\Tester\\AppData\\Local\\Emacs-Toast\\Emacs.png\r\n")
        0))
(fset 'call-process
      (lambda
        (program infile destination display &rest arguments)
        (push
         (list 'process program infile destination display arguments)
         alert-toast-test-boundary-calls)
        (insert
         "/mnt/c/Users/Tester/AppData/Local/Emacs-Toast/Emacs.png\n")
        0))
(fset 'f-exists? (lambda (_path) t))
"##;
    let elisp_form = r##"
(list
 alert-toast--wsl
 alert-toast-default-icon
 (nreverse alert-toast-test-boundary-calls))
"##;
    let expect = expect![[
        r#"OK (t "/mnt/c/Users/Tester/AppData/Local/Emacs-Toast/Emacs.png" ((shell "uname --kernel-release") (region "[System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData) | Join-Path -ChildPath Emacs-Toast\\Emacs.png" "powershell.exe" nil t nil ("-noprofile" "-NonInteractive" "-WindowStyle" "Hidden" "-Command" "-")) (process "wslpath" nil t nil ("C:\\Users\\Tester\\AppData\\Local\\Emacs-Toast\\Emacs.png")) (region "[System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData) | Join-Path -ChildPath Emacs-Toast\\Emacs.png" "powershell.exe" nil t nil ("-noprofile" "-NonInteractive" "-WindowStyle" "Hidden" "-Command" "-")) (process "wslpath" nil t nil ("C:\\Users\\Tester\\AppData\\Local\\Emacs-Toast\\Emacs.png"))))"#
    ]];
    assert_alert_toast_with_prelude_parity(prelude, elisp_form, expect);
}
