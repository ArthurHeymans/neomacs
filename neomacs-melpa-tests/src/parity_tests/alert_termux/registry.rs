use expect_test::expect;

use super::{assert_alert_termux_autoload_parity, assert_alert_termux_parity};

#[test]
fn alert_termux_registry_loads_exact_dependency_and_registers_one_style() {
    let elisp_form = r##"(list
         (featurep 'alert)
         (featurep 'alert-termux)
         (list alert-termux-command
               (get 'alert-termux-command 'custom-type)
               (get 'alert-termux-command 'custom-group))
         (mapcar
          (lambda (entry)
            (list
             (car entry)
             (plist-get (cdr entry) :title)
             (eq (plist-get (cdr entry) :notifier)
                 #'alert-termux-notify)))
          (seq-filter
           (lambda (entry) (eq (car entry) 'termux))
           alert-styles)))"##;
    let expect = expect![[r#"OK (t t (nil file nil) ((termux "Notify using termux" t)))"#]];
    assert_alert_termux_parity(elisp_form, expect);
}

#[test]
fn alert_termux_callable_surface_and_dependency_contract_match() {
    let elisp_form = r##"(list
         (help-function-arglist 'alert-termux-notify t)
         (commandp 'alert-termux-notify)
         (macrop 'alert-termux-notify)
         (autoloadp (symbol-function 'alert-termux-notify))
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (help-function-arglist symbol t)))
          '(alert alert-define-style
            alert-encode-string alert-message-notify)))"##;
    let expect = expect![
        "OK ((info) nil nil nil ((alert t (message &rest --cl-rest--)) (alert-define-style t (name &rest plist)) (alert-encode-string t (str)) (alert-message-notify t (info))))"
    ];
    assert_alert_termux_parity(elisp_form, expect);
}

#[test]
fn alert_termux_autoload_file_does_not_load_package_or_publish_commands() {
    let elisp_form = r##"(list
         (featurep 'alert)
         (featurep 'alert-termux)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (autoloadp (and (fboundp symbol)
                                  (symbol-function symbol)))))
          '(alert-termux-notify alert)))"##;
    let expect = expect!["OK (nil nil ((alert-termux-notify nil nil) (alert nil nil)))"];
    assert_alert_termux_autoload_parity(elisp_form, expect);
}

#[test]
fn alert_termux_style_can_be_selected_as_alert_default() {
    let elisp_form = r##"(let (sent)
         (let ((alert-default-style 'termux)
               (alert-user-configuration nil)
               (alert-internal-configuration nil)
               (alert-active-alerts nil)
               (alert-log-messages nil)
               (alert-hide-all-notifications nil))
           (cl-letf
               (((symbol-function 'alert-termux-notify)
                 (lambda (info)
                   (setq sent
                         (mapcar
                          (lambda (key)
                            (list key (plist-get info key)))
                          '(:message :title :severity :category
                            :mode :persistent :data)))
                   'sent)))
             (with-temp-buffer
               (rename-buffer "termux-origin" t)
               (text-mode)
               (list
                (alert "Build finished"
                       :severity 'high
                       :category 'ci
                       :data '(:job 7))
                sent
                (length alert-active-alerts))))))"##;
    let expect = expect![[
        r#"OK (nil ((:message "Build finished") (:title "termux-origin") (:severity high) (:category ci) (:mode text-mode) (:persistent nil) (:data (:job 7))) 1)"#
    ]];
    assert_alert_termux_parity(elisp_form, expect);
}
