use expect_test::expect;

use super::assert_alert_toast_parity;

#[test]
fn regular_notification_builds_and_sends_complete_unicode_powershell_script() {
    let elisp_form = r##"
(let ((alert-toast--psprocess 'existing-powershell)
      (alert-toast-default-icon "C:\\Default\\Emacs.png")
      calls)
  (cl-letf
      (((symbol-function 'alert-toast--icon-path)
        (lambda (path)
          (push (list 'icon path) calls)
          (concat "WIN:" path)))
       ((symbol-function 'process-send-string)
        (lambda (process script)
          (push (list 'send process script) calls)
          'queued)))
    (list
     (alert-toast-notify
      '(:title "Tytuł: O'Brien & <status>"
        :message "Zażółć gęślą jaźń — it's paid"
        :icon "/mnt/c/Icons/O'Brien.png"
        :severity urgent
        :persistent t
        :data (:audio mail :long t)))
     alert-toast--psprocess
     (nreverse calls))))
"##;
    let expect = expect![[
        r#"OK (queued existing-powershell ((icon "/mnt/c/Icons/O'Brien.png") (send existing-powershell "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast duration=\"long\"> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Tytuł: O''Brien &amp; &lt;status&gt;</text> <text id=\"2\">Zażółć gęślą jaźń — it''s paid</text> <image id=\"1\" src=\"WIN:/mnt/c/Icons/O''Brien.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Mail\" silent=\"false\" loop=\"false\"></audio></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(604800.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n")))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn severity_and_persistence_matrix_selects_priority_and_exact_expiration_policy() {
    let elisp_form = r##"
(let ((alert-toast--psprocess 'powershell)
      (alert-toast-default-icon "C:\\Emacs.png")
      (alert-fade-time 9.25))
  (cl-letf
      (((symbol-function 'alert-toast--icon-path) #'identity)
       ((symbol-function 'process-send-string)
        (lambda (_process script) script)))
    (mapcar
     (lambda (case)
       (let ((info
              (list
               :title (symbol-name (car case))
               :message "Settlement status"
               :severity (cadr case)
               :persistent (caddr case)
               :never-persist (cadddr case))))
         (list
          (car case)
          (alert-toast-notify info))))
     '((urgent-week urgent t nil)
       (high-fade high nil nil)
       (moderate-fade moderate nil nil)
       (normal-week normal t nil)
       (low-never low t t)
       (trivial-fade trivial nil nil)
       (unknown-fallback unexpected nil nil)
       (missing-fallback nil nil nil)))))
"##;
    let expect = expect![[
        r#"OK ((urgent-week "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">urgent-week</text> <text id=\"2\">Settlement status</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(604800.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (high-fade "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">high-fade</text> <text id=\"2\">Settlement status</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(9.250000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (moderate-fade "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">moderate-fade</text> <text id=\"2\">Settlement status</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(9.250000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (normal-week "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">normal-week</text> <text id=\"2\">Settlement status</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(604800.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (low-never "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">low-never</text> <text id=\"2\">Settlement status</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(9.250000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (trivial-fade "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">trivial-fade</text> <text id=\"2\">Settlement status</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(9.250000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (unknown-fallback "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">unknown-fallback</text> <text id=\"2\">Settlement status</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(9.250000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (missing-fallback "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">missing-fallback</text> <text id=\"2\">Settlement status</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(9.250000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n"))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn notification_data_audio_options_flow_into_real_xml_and_script_generation() {
    let elisp_form = r##"
(let ((alert-toast--psprocess 'powershell)
      (alert-toast-default-icon "C:\\Emacs.png")
      (alert-fade-time 4)
      sent)
  (cl-letf
      (((symbol-function 'alert-toast--icon-path) #'identity)
       ((symbol-function 'process-send-string)
        (lambda (_process script)
          (push script sent)
          'sent)))
    (mapcar
     (lambda (case)
       (setq sent nil)
       (let ((result
              (alert-toast-notify
               (list
                :title (symbol-name (car case))
                :message "Audio policy"
                :severity 'normal
                :data (cdr case)))))
         (list
          (car case)
          result
          (car sent))))
     '((default :audio default)
       (silent :silent t)
       (loop :audio reminder :loop t)
       (long :audio sms :long t)
       (looping-alarm :audio alarm)
       (unknown :audio not-a-windows-sound)))))
"##;
    let expect = expect![[
        r#"OK ((default sent "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">default</text> <text id=\"2\">Audio policy</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"false\" loop=\"false\"></audio></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(4.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (silent sent "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">silent</text> <text id=\"2\">Audio policy</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"true\" loop=\"false\"></audio></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(4.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (loop sent "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">loop</text> <text id=\"2\">Audio policy</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Reminder\" silent=\"false\" loop=\"true\"></audio></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(4.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (long sent "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast duration=\"long\"> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">long</text> <text id=\"2\">Audio policy</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.SMS\" silent=\"false\" loop=\"false\"></audio></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(4.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (looping-alarm sent "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast duration=\"long\"> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">looping-alarm</text> <text id=\"2\">Audio policy</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Looping.Alarm\" silent=\"false\" loop=\"true\"></audio></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(4.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (unknown sent "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">unknown</text> <text id=\"2\">Audio policy</text> <image id=\"1\" src=\"C:\\Emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"false\" loop=\"false\"></audio></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(4.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n"))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn shoulder_notification_requires_both_fields_and_preserves_fallback_payloads() {
    let elisp_form = r##"
(let ((alert-toast--psprocess 'powershell)
      (alert-toast-default-icon "C:\\Default\\Emacs.png")
      (alert-fade-time 5))
  (cl-letf
      (((symbol-function 'alert-toast--icon-path)
        (lambda (path) (concat "WIN:" path)))
       ((symbol-function 'process-send-string)
        (lambda (_process script) script)))
    (mapcar
     (lambda (case)
       (list
        (car case)
        (alert-toast-notify
         (list
          :title "Colleague's approval"
          :message "It's ready & waiting"
          :icon "/mnt/c/Icons/O'Brien.png"
          :severity 'high
          :data (cdr case)))))
     '((both
        :shoulder-person "mailto:o'brien@example.invalid"
        :shoulder-payload "https://example.invalid/tap.gif?x=1&y=2")
       (person-only
        :shoulder-person "mailto:o'brien@example.invalid")
       (payload-only
        :shoulder-payload "C:\\Payloads\\tap.png")
       (neither)))))
"##;
    let expect = expect![[
        r#"OK ((both "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast hint-people=\"mailto:o''brien@example.invalid\"> <visual> <binding template=\"ToastGeneric\"> <text>Colleague''s approval</text> <text>It''s ready &amp; waiting</text> <image src=\"WIN:/mnt/c/Icons/O''Brien.png\" placement=\"appLogoOverride\" hint-crop=\"circle\"></image></binding> <binding template=\"ToastGeneric\" experienceType=\"shoulderTap\"> <image src=\"https://example.invalid/tap.gif?x=1&amp;y=2\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Microsoft.People_8wekyb3d8bbwe!x4c7a3b7dy2188y46d4ya362y19ac5a5805e5x')\n    $Notifier.Show($Toast);\n") (person-only "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Colleague''s approval</text> <text id=\"2\">It''s ready &amp; waiting</text> <image id=\"1\" src=\"WIN:/mnt/c/Icons/O''Brien.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(5.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (payload-only "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Colleague''s approval</text> <text id=\"2\">It''s ready &amp; waiting</text> <image id=\"1\" src=\"WIN:/mnt/c/Icons/O''Brien.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(5.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n") (neither "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Colleague''s approval</text> <text id=\"2\">It''s ready &amp; waiting</text> <image id=\"1\" src=\"WIN:/mnt/c/Icons/O''Brien.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(5.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n"))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn lazy_process_initialization_happens_once_and_subsequent_notifications_reuse_handle() {
    let elisp_form = r##"
(let ((alert-toast--psprocess nil)
      (alert-toast-default-icon "C:\\Emacs.png")
      calls)
  (cl-letf
      (((symbol-function 'alert-toast--psprocess-init)
        (lambda ()
          (push '(initialize) calls)
          (setq alert-toast--psprocess 'new-powershell)
          'initialized))
       ((symbol-function 'alert-toast--icon-path) #'identity)
       ((symbol-function 'process-send-string)
        (lambda (process script)
          (push
           (list
            'send
            process
            (and (string-match-p "First settlement" script) t)
            (and (string-match-p "Second settlement" script) t))
           calls)
          'queued)))
    (let ((first
           (alert-toast-notify
            '(:title "First settlement"
              :message "accepted"
              :severity normal)))
          (second
           (alert-toast-notify
            '(:title "Second settlement"
              :message "archived"
              :severity normal))))
      (list
       first
       second
       alert-toast--psprocess
       (nreverse calls)))))
"##;
    let expect = expect![
        "OK (queued queued new-powershell ((initialize) (send new-powershell t nil) (send new-powershell nil t)))"
    ];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn process_initialization_failure_propagates_before_any_send_attempt() {
    let elisp_form = r##"
(let ((alert-toast--psprocess nil)
      (alert-toast-default-icon "C:\\Emacs.png")
      calls
      outcome)
  (cl-letf
      (((symbol-function 'alert-toast--psprocess-init)
        (lambda ()
          (push '(initialize) calls)
          (error "PowerShell unavailable")))
       ((symbol-function 'alert-toast--icon-path) #'identity)
       ((symbol-function 'process-send-string)
        (lambda (&rest arguments)
          (push (cons 'unexpected-send arguments) calls))))
    (condition-case error-data
        (alert-toast-notify
         '(:title "Failure"
           :message "should not send"
           :severity urgent))
      (error
       (setq outcome
             (list
              (car error-data)
              (cadr error-data)
              alert-toast--psprocess
              (nreverse calls)))))
    outcome))
"##;
    let expect = expect![[r#"OK (error "PowerShell unavailable" nil ((initialize)))"#]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn send_failure_propagates_without_discarding_the_persistent_process_handle() {
    let elisp_form = r##"
(let ((alert-toast--psprocess 'live-powershell)
      (alert-toast-default-icon "C:\\Emacs.png")
      outcome)
  (cl-letf
      (((symbol-function 'alert-toast--icon-path) #'identity)
       ((symbol-function 'process-send-string)
        (lambda (process script)
          (error
           "broken pipe to %S after %d bytes"
           process
           (length script)))))
    (condition-case error-data
        (alert-toast-notify
         '(:title "Delivery"
           :message "must surface failure"
           :severity urgent
           :persistent t))
      (error
       (setq outcome
             (list
              (car error-data)
              (cadr error-data)
              alert-toast--psprocess))))
    outcome))
"##;
    let expect =
        expect![[r#"OK (error "broken pipe to live-powershell after 702 bytes" live-powershell)"#]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn alert_style_end_to_end_dispatches_public_alert_call_to_toast_notifier() {
    let elisp_form = r##"
(let ((alert-toast--psprocess 'powershell)
      (alert-toast-default-icon "C:\\Default\\Emacs.png")
      (alert-fade-time 6)
      calls)
  (cl-letf
      (((symbol-function 'alert-toast--icon-path)
        (lambda (path)
          (push (list 'icon path) calls)
          path))
       ((symbol-function 'process-send-string)
        (lambda (process script)
          (push (list 'send process script) calls)
          'queued)))
    (list
     (alert
      "Invoice 42 paid & archived"
      :title "Accounts: O'Brien"
      :style 'toast
      :severity 'urgent
      :persistent t
      :icon "C:\\Custom\\invoice.png"
      :data '(:audio mail))
     alert-toast--psprocess
     (nreverse calls))))
"##;
    let expect = expect![[
        r#"OK (nil powershell ((icon "C:\\Custom\\invoice.png") (send powershell "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Accounts: O''Brien</text> <text id=\"2\">Invoice 42 paid &amp; archived</text> <image id=\"1\" src=\"C:\\Custom\\invoice.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Mail\" silent=\"false\" loop=\"false\"></audio></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(604800.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n")))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn sparse_and_malformed_info_plists_follow_package_fallbacks_without_hidden_validation() {
    let elisp_form = r##"
(let ((alert-toast--psprocess 'powershell)
      (alert-toast-default-icon "C:\\Default\\Emacs.png")
      (alert-fade-time 3))
  (cl-letf
      (((symbol-function 'alert-toast--icon-path)
        (lambda (path) (list 'converted path)))
       ((symbol-function 'process-send-string)
        (lambda (process script)
          (list process script))))
    (mapcar
     (lambda (info)
       (condition-case error-data
           (list 'value (alert-toast-notify info))
         (error
          (list
           'signal
           (car error-data)
           (cadr error-data)))))
     (list
      nil
      '(:title nil :message nil :severity nil :data nil)
      '(:title 42 :message (structured body)
        :severity missing :icon 99 :data (:audio missing))
      '(:title "Odd plist" :message "still accepted"
        :data (:shoulder-person 17 :shoulder-payload 23))))))
"##;
    let expect = expect![
        "OK ((signal wrong-type-argument stringp) (signal wrong-type-argument stringp) (signal wrong-type-argument listp) (signal wrong-type-argument sequencep))"
    ];
    assert_alert_toast_parity(elisp_form, expect);
}
