use expect_test::expect;

use super::{
    assert_alert_toast_autoload_parity, assert_alert_toast_parity,
    assert_alert_toast_with_prelude_parity,
};

#[test]
fn exact_release_descriptor_feature_defaults_and_customization_surface() {
    let elisp_form = r##"
(let* ((descriptor
        (cadr (assq 'alert-toast package-alist)))
       (extras (package-desc-extras descriptor)))
  (list
   (package-desc-name descriptor)
   (package-version-join (package-desc-version descriptor))
   (package-desc-reqs descriptor)
   (alist-get :commit extras)
   (alist-get :url extras)
   (featurep 'alert-toast)
   alert-toast--wsl
   (file-name-nondirectory alert-toast-default-icon)
   (mapcar
    (lambda (option)
      (list
       option
       (custom-variable-p option)
       (get option 'custom-type)
       (get option 'custom-group)))
    '(alert-toast-priorities))))
"##;
    let expect = expect![[
        r#"OK (alert-toast "20220312.229" ((emacs (25 1)) (alert (1 2)) (f (0 20 0)) (s (1 12 0))) "96c88c93c1084de681700f655223142ee0eb944a" "https://github.com/gkowzan/alert-toast" t nil "emacs.png" ((alert-toast-priorities ((funcall #'#[nil ('((urgent . "[Windows.UI.Notifications.ToastNotificationPriority]::High") (high . "[Windows.UI.Notifications.ToastNotificationPriority]::High") (moderate . "[Windows.UI.Notifications.ToastNotificationPriority]::Default") (normal . "[Windows.UI.Notifications.ToastNotificationPriority]::Default") (low . "[Windows.UI.Notifications.ToastNotificationPriority]::Default") (trivial . "[Windows.UI.Notifications.ToastNotificationPriority]::Default"))) (t)])) (alist :key-type symbol :value-type string) nil)))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn package_installed_alert_f_and_s_dependencies_meet_exact_declared_contracts() {
    let elisp_form = r##"
(let ((requirements
       '((alert (1 2) alert-define-style)
         (f (0 20 0) f-copy)
         (s (1 12 0) s-replace-all))))
  (mapcar
   (lambda (requirement)
     (let* ((package (car requirement))
            (minimum (cadr requirement))
            (function (caddr requirement))
            (descriptor
             (cadr (assq package package-alist)))
            (library (locate-library (symbol-name package))))
       (list
        package
        (and descriptor t)
        (package-installed-p package minimum)
        (featurep package)
        (fboundp function)
        (file-name-nondirectory library)
        (and
         (string-prefix-p
          (file-name-as-directory package-user-dir)
          library)
         t))))
   requirements))
"##;
    let expect =
        expect![[r#"OK ((alert t t t t "alert.el" t) (f t t t t "f.el" t) (s t t t t "s.el" t))"#]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn complete_function_signatures_interactive_contracts_and_docs_are_stable() {
    let elisp_form = r##"
(mapcar
 (lambda (function)
   (list
    function
    (help-function-arglist function t)
    (interactive-form function)
    (secure-hash 'sha256 (documentation function))))
 '(alert-toast--check-wsl
   alert-toast--appdir
   alert-toast--default-wsl-icon-path
   alert-toast--init-wsl-icon
   alert-toast--icon-path
   alert-toast--coding-page
   alert-toast--psprocess-init
   alert-toast--psprocess-kill
   alert-toast--fill-template
   alert-toast--fill-shoulder
   alert-toast-notify))
"##;
    let expect = expect![[
        r#"OK ((alert-toast--check-wsl nil nil "a2ee11bc4fb774ef32d112cee563ae97bf22a6cea5332b6b1ef6392129c47b5b") (alert-toast--appdir nil nil "d89eb392d7e6d8acfd22ddd407643acdc9345c24d153560ba05dc598efd495f4") (alert-toast--default-wsl-icon-path nil nil "6df2a9d63d5119bb6b9838e3dcd927cc927066c4cd8fc91bbedb517d967f765c") (alert-toast--init-wsl-icon nil nil "bc96b6bc0ef362c56748638932581d6bd80f22b304e552b5bec08d3c239ca420") (alert-toast--icon-path (path) nil "479f841a000853a7a5e19b1c41ed4832fcfdd27740f6eb685cdf0d2d0cc843a2") (alert-toast--coding-page nil nil "beccf4d7d4937f96c511aafaca9ad00d037edbbaad59c6aafcdf0b5cd83db8e9") (alert-toast--psprocess-init nil nil "bac787f21cb2f2919c4fa842249f2c553e5d0d46f2366ed5a1d0c1d91edfd7eb") (alert-toast--psprocess-kill nil nil "b5abadd57f11c14378dbf072cc25501fa6afe5c826be10943399ee9c9d1303b0") (alert-toast--fill-template (title message icon-path &optional audio silent long loop) nil "00582a461fda7350f032e4c14f467565d08f8b1b877b6dedbf3848691339d80b") (alert-toast--fill-shoulder (title message icon-path person payload) nil "e88e6a0a47dc90aa405cea39b011f97eacf670dea5f2862a46a46d2b8ac82c3a") (alert-toast-notify (info) nil "505c2e9a9db3539cfa798ebc733eae53c00445fcfa085fe3b97346bcc3f1f708"))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn priority_sound_looping_sound_and_powershell_templates_are_complete_and_stable() {
    let elisp_form = r##"
(list
 alert-toast-priorities
 alert-toast--sounds
 (length alert-toast--looping-sounds)
 alert-toast--looping-sounds
 alert-toast--psquote-replacements
 alert-toast--appdir-text
 (secure-hash 'sha256 alert-toast--psscript-text)
 (secure-hash 'sha256 alert-toast--psscript-shoulder))
"##;
    let expect = expect![[
        r#"OK (((urgent . "[Windows.UI.Notifications.ToastNotificationPriority]::High") (high . "[Windows.UI.Notifications.ToastNotificationPriority]::High") (moderate . "[Windows.UI.Notifications.ToastNotificationPriority]::Default") (normal . "[Windows.UI.Notifications.ToastNotificationPriority]::Default") (low . "[Windows.UI.Notifications.ToastNotificationPriority]::Default") (trivial . "[Windows.UI.Notifications.ToastNotificationPriority]::Default")) ((default . "ms-winsoundevent:Notification.Default") (im . "ms-winsoundevent:Notification.IM") (mail . "ms-winsoundevent:Notification.Mail") (reminder . "ms-winsoundevent:Notification.Reminder") (sms . "ms-winsoundevent:Notification.SMS")) 20 ((alarm10 . "ms-winsoundevent:Notification.Looping.Alarm10") (call10 . "ms-winsoundevent:Notification.Looping.Call10") (alarm9 . "ms-winsoundevent:Notification.Looping.Alarm9") (call9 . "ms-winsoundevent:Notification.Looping.Call9") (alarm8 . "ms-winsoundevent:Notification.Looping.Alarm8") (call8 . "ms-winsoundevent:Notification.Looping.Call8") (alarm7 . "ms-winsoundevent:Notification.Looping.Alarm7") (call7 . "ms-winsoundevent:Notification.Looping.Call7") (alarm6 . "ms-winsoundevent:Notification.Looping.Alarm6") (call6 . "ms-winsoundevent:Notification.Looping.Call6") (alarm5 . "ms-winsoundevent:Notification.Looping.Alarm5") (call5 . "ms-winsoundevent:Notification.Looping.Call5") (alarm4 . "ms-winsoundevent:Notification.Looping.Alarm4") (call4 . "ms-winsoundevent:Notification.Looping.Call4") (alarm3 . "ms-winsoundevent:Notification.Looping.Alarm3") (call3 . "ms-winsoundevent:Notification.Looping.Call3") (alarm2 . "ms-winsoundevent:Notification.Looping.Alarm2") (call2 . "ms-winsoundevent:Notification.Looping.Call2") (call . "ms-winsoundevent:Notification.Looping.Call") (alarm . "ms-winsoundevent:Notification.Looping.Alarm")) (("'" . "''")) "[System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData) | Join-Path -ChildPath Emacs-Toast\\Emacs.png" "72025423fea55a716b87a52e445ecb4f4fc360c608dff9fb72542d0382c31cc8" "825a78792783d21fbe5c50786b6a5154275712f12b7a9773fa280ab25eb9fab9")"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn toast_style_registration_points_alert_dispatch_at_the_exact_notifier() {
    let elisp_form = r##"
(let ((style (assq 'toast alert-styles)))
  (list
   (and style t)
   (copy-tree style)
   (plist-get (cdr style) :title)
   (plist-get (cdr style) :notifier)
   (eq
    (plist-get (cdr style) :notifier)
    #'alert-toast-notify)))
"##;
    let expect = expect![[
        r#"OK (t (toast :title "Windows 10 toast notification" :notifier alert-toast-notify) "Windows 10 toast notification" alert-toast-notify t)"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_notify_without_eagerly_loading_runtime_or_style() {
    let elisp_form = r##"
(let ((definition
       (symbol-function 'alert-toast-notify)))
  (list
   (autoloadp definition)
   (nth 1 definition)
   (nth 3 definition)
   (nth 4 definition)
   (featurep 'alert-toast)
   (boundp 'alert-styles)
   (and
    (boundp 'alert-styles)
    (assq 'toast alert-styles))))
"##;
    let expect = expect![[r#"OK (t "alert-toast" nil nil nil nil nil)"#]];
    assert_alert_toast_autoload_parity(elisp_form, expect);
}

#[test]
fn source_initialization_probes_kernel_once_and_registers_style_after_dependencies() {
    let prelude = r##"
(defvar alert-toast-test-shell-calls nil)
(fset 'shell-command-to-string
      (lambda (command)
        (push command alert-toast-test-shell-calls)
        "6.8.0-generic\n"))
"##;
    let elisp_form = r##"
(list
 (nreverse alert-toast-test-shell-calls)
 alert-toast--wsl
 (featurep 'alert)
 (featurep 'f)
 (featurep 's)
 (and (assq 'toast alert-styles) t))
"##;
    let expect = expect![[r#"OK (("uname --kernel-release") nil t t t t)"#]];
    assert_alert_toast_with_prelude_parity(prelude, elisp_form, expect);
}
