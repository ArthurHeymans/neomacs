use expect_test::expect;

use super::assert_alert_parity;

#[test]
fn alert_growl_backend_builds_unix_priority_sticky_and_message_arguments() {
    let elisp_form = r##"(let ((alert-growl-command "/usr/bin/growlnotify")
                (system-type 'gnu/linux)
                calls)
         (cl-letf
             (((symbol-function 'alert-encode-string) #'identity)
              ((symbol-function 'call-process)
               (lambda (program infile destination display &rest args)
                 (push
                  (list program infile destination display args)
                  calls)
                 0)))
           (list
            (alert-growl-notify
             '(:title "Build" :message "Done"
               :severity high :persistent t
               :never-persist nil))
            (alert-growl-notify
             '(:title "Quiet" :message "Info"
               :severity trivial :persistent nil))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (0 0 (("/usr/bin/growlnotify" nil nil nil ("--appIcon" "Emacs" "--name" "Emacs" "--title" "Build" "--priority" "2" "--sticky" "--message" "Done")) ("/usr/bin/growlnotify" nil nil nil ("--appIcon" "Emacs" "--name" "Emacs" "--title" "Quiet" "--priority" "-2" "--message" "Info"))))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_growl_missing_command_falls_back_to_message_style() {
    let elisp_form = r##"(let ((alert-growl-command nil)
                calls)
         (cl-letf (((symbol-function 'alert-message-notify)
                    (lambda (info)
                      (push info calls)
                      'fallback)))
           (list
            (alert-growl-notify
             '(:title "Build" :message "Done"))
            (mapcar
             (lambda (info)
               (list (plist-get info :title)
                     (plist-get info :message)))
             calls))))"##;
    let expect = expect![[r#"OK (fallback (("Build" "Done")))"#]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_libnotify_backend_handles_list_category_icon_urgency_and_persistence() {
    let elisp_form = r##"(let ((alert-libnotify-command "/usr/bin/notify-send")
                (alert-libnotify-additional-args
                 '("--hint" "string:desktop-entry:emacs"))
                (alert-default-icon "/icons/emacs.svg")
                (alert-fade-time 5)
                calls)
         (cl-letf
             (((symbol-function 'alert-encode-string) #'identity)
              ((symbol-function 'call-process)
               (lambda (program infile destination display &rest args)
                 (push
                  (list program infile
                        (and (listp destination)
                             (buffer-name (car destination)))
                        (and (listp destination)
                             (cadr destination))
                        display args)
                  calls)
                 0)))
           (unwind-protect
               (list
                (alert-libnotify-notify
                 '(:title "Build" :message "Failed"
                   :severity urgent
                   :category (ci deploy)
                   :icon nil :persistent t
                   :never-persist nil))
                (nreverse calls))
             (when (get-buffer " *libnotify output*")
               (kill-buffer " *libnotify output*")))))"##;
    let expect = expect![[
        r#"OK (0 (("/usr/bin/notify-send" nil " *libnotify output*" t nil ("--icon" "/icons/emacs.svg" "--app-name" "Emacs" "--urgency" "critical" "--hint" "string:desktop-entry:emacs" "--expire-time" "0" "--category" "ci,deploy" "Build" "Failed"))))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_notifier_osx_toaster_and_termux_build_platform_cli_contracts() {
    let elisp_form = r##"(let ((alert-notifier-command "/bin/terminal-notifier")
                (alert-notifier-default-icon "/icons/default.png")
                (alert-toaster-command "/bin/toast")
                (alert-toaster-default-icon "/icons/toast.png")
                (alert-termux-command "/bin/termux-notification")
                calls)
         (cl-letf
             (((symbol-function 'alert-encode-string) #'identity)
              ((symbol-function 'call-process)
               (lambda (program infile destination display &rest args)
                 (push
                  (list program infile
                        (if (and (listp destination)
                                 (bufferp (car destination)))
                            (list (buffer-name (car destination))
                                  (cadr destination))
                          destination)
                        display args)
                  calls)
                 0))
              ((symbol-function 'alert-message-notify)
               (lambda (info)
                 (push
                  (list 'message
                        (plist-get info :message))
                  calls))))
           (unwind-protect
               (list
                (alert-notifier-notify
                 '(:title "Title" :message "Body" :icon nil))
                (alert-osx-notifier-notify
                 '(:title "Title" :message "Body"))
                (alert-toaster-notify
                 '(:title "Title" :message "Body" :icon nil))
                (alert-termux-notify
                 '(:title "Title" :message "Body"))
                (nreverse calls))
             (when (get-buffer " *termux-notification output*")
               (kill-buffer " *termux-notification output*")))))"##;
    let expect = expect![[
        r#"OK (0 #1=((message "Body") ("/bin/toast" nil nil nil ("-t" "Title" "-m" "Body" "-p" "/icons/toast.png")) ("/bin/termux-notification" nil (" *termux-notification output*" t) nil ("-t" "Title" "-c" "Body"))) 0 0 (("/bin/terminal-notifier" nil nil nil ("-title" "Title" "-appIcon" "/icons/default.png" "-message" "Body")) ("osascript" nil nil nil ("-e" "display notification \"Body\" with title \"Title\"")) . #1#))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_fringe_and_mode_line_styles_apply_severity_colors_then_restore_faces() {
    let elisp_form = r##"(let ((alert-severity-colors
                '((urgent . "red") (low . "blue"))))
         (cl-letf
             (((symbol-function 'set-face-background)
               (lambda (face color &optional frame)
                 (list 'background face color frame)))
              ((symbol-function 'set-face-foreground)
               (lambda (face color &optional frame)
                 (list 'foreground face color frame)))
              ((symbol-function 'copy-face)
               (lambda (from to &rest args)
                 (list 'copy from to args))))
           (list
            (alert-fringe-notify '(:severity urgent))
            (alert-fringe-restore nil)
            (alert-mode-line-notify '(:severity low))
            (alert-mode-line-restore nil))))"##;
    let expect = expect![[
        r#"OK ((background fringe "red" nil) (copy alert-saved-fringe-face fringe nil) (foreground mode-line "white" nil) (copy alert-saved-mode-line-face mode-line nil))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_x_urgency_hint_sets_and_clears_only_the_urgency_bit() {
    let elisp_form = r##"(let (writes)
         (cl-letf
             (((symbol-function 'x-window-property)
               (lambda (&rest _)
                 (vector #x00000005 11 22)))
              ((symbol-function 'x-change-window-property)
               (lambda (property value frame type format append)
                 (push
                  (list property value frame type format append)
                  writes)
                 'written)))
           (list
            (x-urgency-hint 'frame t 'source)
            (x-urgency-hint 'frame nil 'source)
            (nreverse writes))))"##;
    let expect = expect![[
        r#"OK (written written (("WM_HINTS" (261 11 22) frame "WM_HINTS" 32 t) ("WM_HINTS" (5 11 22) frame "WM_HINTS" 32 t)))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_notifications_backend_replaces_id_switches_buffer_and_removes_hash_entry() {
    let elisp_form = r##"(let ((alert-notifications-ids
                (make-hash-table :test #'equal))
               notify-args
               closed
               switched)
         (if (not (fboundp 'alert-notifications-notify))
             (list :available nil
                   (featurep 'notifications))
           (cl-letf
               (((symbol-function 'notifications-notify)
                 (lambda (&rest args)
                   (setq notify-args args)
                   77))
                ((symbol-function
                  'notifications-close-notification)
                 (lambda (id) (setq closed id)))
                ((symbol-function 'switch-to-buffer)
                 (lambda (buffer) (setq switched buffer)))
                ((symbol-function 'alert-message-notify) #'ignore))
             (with-temp-buffer
               (let ((origin (current-buffer)))
                 (puthash 'job 42 alert-notifications-ids)
                 (alert-notifications-notify
                  (list :title "Build" :message "Done"
                        :icon "mail" :persistent t
                        :severity 'high :id 'job
                        :buffer origin))
                 (funcall (plist-get notify-args :on-action)
                          77 "default")
                 (alert-notifications-remove '(:id job))
                 (list
                  :available t
                  (plist-get notify-args :title)
                  (plist-get notify-args :body)
                  (plist-get notify-args :timeout)
                  (plist-get notify-args :replaces-id)
                  (plist-get notify-args :urgency)
                  (eq switched origin)
                  closed
                  (gethash 'job
                           alert-notifications-ids)))))))"##;
    let expect = expect![[r#"OK (:available t "Build" "Done" 0 42 critical t 77 nil)"#]];
    assert_alert_parity(elisp_form, expect);
}
