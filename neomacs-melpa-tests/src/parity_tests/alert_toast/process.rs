use expect_test::expect;

use super::assert_alert_toast_parity;

#[test]
fn coding_page_queries_hidden_noninteractive_powershell_and_interns_chomped_output() {
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
          (insert "utf-16le\r\n")
          0)))
    (list
     (alert-toast--coding-page)
     (nreverse calls))))
"##;
    let expect = expect![[
        r#"OK (utf-16le (("[console]::InputEncoding.BodyName" "powershell.exe" nil t nil ("-noprofile" "-NonInteractive" "-WindowStyle" "Hidden" "-Command" "-") utf-8-dos)))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn coding_page_handles_empty_and_unregistered_encoding_names_without_extra_policy() {
    let elisp_form = r##"
(mapcar
 (lambda (output)
   (cl-letf
       (((symbol-function 'call-process-region)
         (lambda (&rest _arguments)
           (insert output)
           0)))
     (list
      output
      (alert-toast--coding-page))))
 '("\r\n" "definitely-not-a-coding-system\r\n" "utf-8\n"))
"##;
    let expect = expect![[
        r#"OK (("\15\n" nil) ("definitely-not-a-coding-system\15\n" nil) ("utf-8\n" utf-8))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn native_process_init_uses_discovered_coding_exact_command_and_bootstrap_script() {
    let elisp_form = r##"
(let ((alert-toast--wsl nil)
      (alert-toast--psprocess nil)
      calls)
  (cl-letf
      (((symbol-function 'alert-toast--coding-page)
        (lambda ()
          (push '(coding-page) calls)
          'utf-16le))
       ((symbol-function 'make-process)
        (lambda (&rest arguments)
          (push (cons 'make-process arguments) calls)
          'native-process))
       ((symbol-function 'process-send-string)
        (lambda (process string)
          (push (list 'send process string) calls)
          nil)))
    (list
     (alert-toast--psprocess-init)
     alert-toast--psprocess
     (nreverse calls))))
"##;
    let expect = expect![[
        r#"OK (nil native-process ((coding-page) (make-process :name "powershell-toast" :buffer "*powershell-toast*" :command ("powershell.exe" "-noprofile" "-NoExit" "-NonInteractive" "-WindowStyle" "Hidden" "-Command" "-") :coding utf-16le :noquery t :connection-type pipe) (send native-process "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null\n[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml, ContentType=WindowsRuntime] > $null\n")))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn wsl_process_init_forces_utf8_and_never_probes_windows_coding_page() {
    let elisp_form = r##"
(let ((alert-toast--wsl t)
      (alert-toast--psprocess nil)
      calls)
  (cl-letf
      (((symbol-function 'alert-toast--coding-page)
        (lambda ()
          (push '(unexpected-coding-probe) calls)
          'utf-16le))
       ((symbol-function 'make-process)
        (lambda (&rest arguments)
          (push (cons 'make-process arguments) calls)
          'wsl-process))
       ((symbol-function 'process-send-string)
        (lambda (process string)
          (push (list 'send process string) calls)
          nil)))
    (list
     (alert-toast--psprocess-init)
     alert-toast--psprocess
     (nreverse calls))))
"##;
    let expect = expect![[
        r#"OK (nil wsl-process ((make-process :name "powershell-toast" :buffer "*powershell-toast*" :command ("powershell.exe" "-noprofile" "-NoExit" "-NonInteractive" "-WindowStyle" "Hidden" "-Command" "-") :coding utf-8 :noquery t :connection-type pipe) (send wsl-process "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null\n[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml, ContentType=WindowsRuntime] > $null\n")))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn process_kill_deletes_exact_handle_and_clears_persistent_state() {
    let elisp_form = r##"
(let ((alert-toast--psprocess 'toast-process)
      calls)
  (cl-letf
      (((symbol-function 'delete-process)
        (lambda (process)
          (push process calls)
          'deleted)))
    (list
     (alert-toast--psprocess-kill)
     alert-toast--psprocess
     (nreverse calls))))
"##;
    let expect = expect!["OK (nil nil (toast-process))"];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn process_kill_without_initialized_process_surfaces_the_editor_error() {
    let elisp_form = r##"
(let ((alert-toast--psprocess nil)
      outcome)
  (condition-case error-data
      (alert-toast--psprocess-kill)
    (error
     (setq outcome
           (list
            (car error-data)
            (cadr error-data)
            alert-toast--psprocess))))
  outcome)
"##;
    let expect = expect![[r#"OK (error "Buffer *scratch* has no process" nil)"#]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn process_creation_failure_propagates_and_does_not_publish_a_partial_handle() {
    let elisp_form = r##"
(let ((alert-toast--wsl t)
      (alert-toast--psprocess nil)
      outcome)
  (cl-letf
      (((symbol-function 'make-process)
        (lambda (&rest _arguments)
          (error "powershell.exe unavailable")))
       ((symbol-function 'process-send-string)
        (lambda (&rest _arguments)
          (error "bootstrap should not run"))))
    (condition-case error-data
        (alert-toast--psprocess-init)
      (error
       (setq outcome
             (list
              (car error-data)
              (cadr error-data)
              alert-toast--psprocess))))
    outcome))
"##;
    let expect = expect![[r#"OK (error "powershell.exe unavailable" nil)"#]];
    assert_alert_toast_parity(elisp_form, expect);
}
