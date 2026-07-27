use expect_test::expect;

use super::{assert_alert_autoload_parity, assert_alert_parity};

#[test]
fn alert_registry_defaults_severity_maps_styles_and_optional_features_match() {
    let elisp_form = r##"(list
         (featurep 'alert)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (symbol-value symbol)
                  (get symbol 'custom-type)
                  (get symbol 'custom-group)))
          '(alert-severity-faces
            alert-severity-colors
            alert-log-severity-functions
            alert-log-level
            alert-reveal-idle-time
            alert-persist-idle-time
            alert-fade-time
            alert-hide-all-notifications
            alert-log-messages
            alert-default-style
            alert-user-configuration))
         (mapcar #'car alert-styles)
         (mapcar
          (lambda (feature) (list feature (featurep feature)))
          '(gntp notifications log4e))
         (list (hash-table-p alert-notifications-ids)
               (hash-table-test alert-notifications-ids)
               (hash-table-count alert-notifications-ids)))"##;
    let expect = expect![[
        r#"OK (t ((alert-severity-faces ((urgent . alert-urgent-face) (high . alert-high-face) (moderate . alert-moderate-face) (normal . alert-normal-face) (low . alert-low-face) (trivial . alert-trivial-face)) (alist :key-type symbol :value-type color) nil) (alert-severity-colors ((urgent . "red") (high . "orange") (moderate . "yellow") (normal . "green") (low . "blue") (trivial . "purple")) (alist :key-type symbol :value-type color) nil) (alert-log-severity-functions ((urgent . alert--log-fatal) (high . alert--log-error) (moderate . alert--log-warn) (normal . alert--log-info) (low . alert--log-debug) (trivial . alert--log-trace)) (alist :key-type symbol :value-type color) nil) (alert-log-level normal symbol nil) (alert-reveal-idle-time 15 integer nil) (alert-persist-idle-time 900 integer nil) (alert-fade-time 5 integer nil) (alert-hide-all-notifications nil boolean nil) (alert-log-messages t boolean nil) (alert-default-style message (radio :tag "Style") nil) (alert-user-configuration nil (repeat (list :tag "Select style if alert matches selector" (repeat :tag "Selector" (choice (cons :tag "Severity" (const :format "" :severity) (set (const :tag "Urgent" urgent) (const :tag "High" high) (const :tag "Moderate" moderate) (const :tag "Normal" normal) (const :tag "Low" low) (const :tag "Trivial" trivial))) (cons :tag "User Status" (const :format "" :status) (set (const :tag "Buffer not visible" buried) (const :tag "Buffer visible" visible) (const :tag "Buffer selected" selected) (const :tag "Buffer selected, user idle" idle))) (cons :tag "Major Mode" (const :format "" :mode) regexp) (cons :tag "Category" (const :format "" :category) regexp) (cons :tag "Title" (const :format "" :title) regexp) (cons :tag "Message" (const :format "" :message) regexp) (cons :tag "Predicate" (const :format "" :predicate) function) (cons :tag "Icon" (const :format "" :icon) regexp))) (choice :tag "Style" (const :tag "Change the fringe color" fringe) (const :tag "Notify using gntp" gntp) (const :tag "Notify using Growl" growl) (const :tag "Don't display alerts" ignore) (const :tag "Ignore Alert" ignore) (const :tag "Notify using libnotify" libnotify) (const :tag "Log to *Alerts* buffer" log) (const :tag "Display message in minibuffer" message) (const :tag "Change the mode-line color" mode-line) (const :tag "Display message momentarily in buffer" momentary) (const :tag "Notify using notifications" notifications) (const :tag "Notify using terminal-notifier" notifier) (const :tag "Notify using native OSX notification" osx-notifier) (const :tag "Notify using termux" termux) (const :tag "Notify using Toaster" toaster) (const :tag "Set the X11 window property" x11)) (set :tag "Options" (cons :tag "Make alert persistent" (const :format "" :persistent) (choice :value t (const :tag "Yes" t) (function :tag "Predicate"))) (cons :tag "Never persist" (const :format "" :never-persist) (choice :value t (const :tag "Yes" t) (function :tag "Predicate"))) (cons :tag "Continue to next rule" (const :format "" :continue) (choice :value t (const :tag "Yes" t) (function :tag "Predicate")))))) nil)) (fringe gntp growl ignore ignore libnotify log message mode-line momentary notifications notifier osx-notifier termux toaster x11) ((gntp t) (notifications t) (log4e t)) (t equal 0))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_all_severity_levels_have_faces_colors_loggers_and_face_specs() {
    let elisp_form = r##"(mapcar
         (lambda (severity)
           (let ((face (cdr (assq severity alert-severity-faces))))
             (list severity
                   face
                   (facep face)
                   (get face 'face-defface-spec)
                   (cdr (assq severity alert-severity-colors))
                   (cdr (assq severity
                              alert-log-severity-functions)))))
         '(urgent high moderate normal low trivial))"##;
    let expect = expect![[
        r#"OK ((urgent alert-urgent-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:foreground "Red" :bold t))) "red" alert--log-fatal) (high alert-high-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:foreground "Dark Orange" :bold t))) "orange" alert--log-error) (moderate alert-moderate-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:foreground "Gold" :bold t))) "yellow" alert--log-warn) (normal alert-normal-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t)) "green" alert--log-info) (low alert-low-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:foreground "Dark Blue"))) "blue" alert--log-debug) (trivial alert-trivial-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:foreground "Dark Violet"))) "purple" alert--log-trace))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_complete_callable_surface_arglists_commands_and_autoload_flags_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (macrop symbol)
                 (autoloadp (symbol-function symbol))))
         '(alert-styles-radio-type
           alert-configuration-type
           alert-define-style
           alert-add-rule
           alert-log-notify
           alert-legacy-log-notify
           alert-log-clear
           alert-message-notify
           alert-message-remove
           alert-momentary-notify
           alert-fringe-notify
           alert-fringe-restore
           alert-mode-line-notify
           alert-mode-line-restore
           alert-encode-string
           alert-growl-notify
           alert-libnotify-notify
           alert-notifier-notify
           alert-osx-notifier-notify
           alert-frame-notify
           alert-frame-remove
           x-urgency-hint
           x-urgent
           alert-x11-notify
           alert-toaster-notify
           alert-termux-notify
           alert-buffer-status
           alert-remove-when-active
           alert-remove-on-command
           alert-send-notification
           alert))"##;
    let expect = expect![
        "OK ((alert-styles-radio-type (widget-name) nil nil nil) (alert-configuration-type nil nil nil nil) (alert-define-style (name &rest plist) nil nil nil) (alert-add-rule (&rest --cl-rest--) nil nil nil) (alert-log-notify (info) nil nil nil) (alert-legacy-log-notify (mes sev len) nil nil nil) (alert-log-clear (info) nil nil nil) (alert-message-notify (info) nil nil nil) (alert-message-remove (_info) nil nil nil) (alert-momentary-notify (info) nil nil nil) (alert-fringe-notify (info) nil nil nil) (alert-fringe-restore (_info) nil nil nil) (alert-mode-line-notify (info) nil nil nil) (alert-mode-line-restore (_info) nil nil nil) (alert-encode-string (str) nil nil nil) (alert-growl-notify (info) nil nil nil) (alert-libnotify-notify (info) nil nil nil) (alert-notifier-notify (info) nil nil nil) (alert-osx-notifier-notify (info) nil nil nil) (alert-frame-notify (info) nil nil nil) (alert-frame-remove (info) nil nil nil) (x-urgency-hint (frame arg &optional source) nil nil nil) (x-urgent (&optional arg) t nil nil) (alert-x11-notify (_info) nil nil nil) (alert-toaster-notify (info) nil nil nil) (alert-termux-notify (info) nil nil nil) (alert-buffer-status (&optional buffer) nil nil nil) (alert-remove-when-active (remover info) nil nil nil) (alert-remove-on-command nil nil nil nil) (alert-send-notification (alert-buffer info style-def &optional persist never-per) nil nil nil) (alert (message &rest --cl-rest--) nil nil nil))"
    ];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_autoload_contract_exposes_rule_and_dispatch_without_loading_source() {
    let elisp_form = r##"(list
         (featurep 'alert)
         (mapcar
          (lambda (symbol)
            (let ((definition (symbol-function symbol)))
              (list symbol
                    (autoloadp definition)
                    (nth 1 definition)
                    (nth 4 definition)
                    (commandp symbol))))
          '(alert-add-rule alert alert-define-style alert-buffer-status)))"##;
    let expect = expect![[
        r#"OK (nil ((alert-add-rule t "alert" nil nil) (alert t "alert" nil nil) (alert-define-style nil nil nil nil) (alert-buffer-status nil nil nil nil)))"#
    ]];
    assert_alert_autoload_parity(elisp_form, expect);
}

#[test]
fn alert_define_style_ports_upstream_registration_and_refreshes_custom_widgets() {
    let elisp_form = r##"(let ((alert-styles nil))
         (alert-define-style
          'test-register
          :title "Test Register"
          :notifier #'ignore
          :remover #'ignore)
         (let ((entry (assq 'test-register alert-styles)))
           (list
            entry
            (get 'alert-user-configuration 'custom-type)
            (get 'alert-define-style 'custom-type)
            (alert-styles-radio-type 'choice))))"##;
    let expect = expect![[
        r#"OK ((test-register :title "Test Register" :notifier ignore :remover ignore) (repeat (list :tag "Select style if alert matches selector" (repeat :tag "Selector" (choice (cons :tag "Severity" (const :format "" :severity) (set (const :tag "Urgent" urgent) (const :tag "High" high) (const :tag "Moderate" moderate) (const :tag "Normal" normal) (const :tag "Low" low) (const :tag "Trivial" trivial))) (cons :tag "User Status" (const :format "" :status) (set (const :tag "Buffer not visible" buried) (const :tag "Buffer visible" visible) (const :tag "Buffer selected" selected) (const :tag "Buffer selected, user idle" idle))) (cons :tag "Major Mode" (const :format "" :mode) regexp) (cons :tag "Category" (const :format "" :category) regexp) (cons :tag "Title" (const :format "" :title) regexp) (cons :tag "Message" (const :format "" :message) regexp) (cons :tag "Predicate" (const :format "" :predicate) function) (cons :tag "Icon" (const :format "" :icon) regexp))) (choice :tag "Style" (const :tag "Test Register" test-register)) (set :tag "Options" (cons :tag "Make alert persistent" (const :format "" :persistent) (choice :value t (const :tag "Yes" t) (function :tag "Predicate"))) (cons :tag "Never persist" (const :format "" :never-persist) (choice :value t (const :tag "Yes" t) (function :tag "Predicate"))) (cons :tag "Continue to next rule" (const :format "" :continue) (choice :value t (const :tag "Yes" t) (function :tag "Predicate")))))) (radio :tag "Style" (const :tag "Test Register" test-register)) (choice :tag "Style" (const :tag "Test Register" test-register)))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_style_radio_sorting_mutates_registry_into_symbol_order_with_titles() {
    let elisp_form = r##"(let ((alert-styles
                '((zeta :notifier ignore)
                  (alpha :title "Alpha title" :notifier ignore)
                  (middle :title nil :notifier ignore))))
         (list
          (alert-styles-radio-type 'radio)
          (mapcar #'car alert-styles)))"##;
    let expect = expect![[
        r#"OK ((radio :tag "Style" (const :tag "Alpha title" alpha) (const :tag "middle" middle) (const :tag "zeta" zeta)) (alpha middle zeta))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}
