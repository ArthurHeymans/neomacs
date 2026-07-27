use expect_test::expect;

use super::assert_alert_parity;

#[test]
fn alert_add_rule_ports_upstream_prepend_append_and_scalar_normalization() {
    let elisp_form = r##"(let ((alert-internal-configuration nil)
                (alert-default-style 'message))
         (let ((first
                (alert-add-rule
                 :severity 'high
                 :status 'buried
                 :mode 'erc-mode
                 :style 'message))
               (second
                (alert-add-rule
                 :severity '(low trivial)
                 :style 'log
                 :append t)))
           (list first second alert-internal-configuration)))"##;
    let expect = expect![[
        r#"OK (#1=(((:severity high) (:status buried) (:mode . "\\`erc-mode\\'")) message nil) #2=(((:severity low trivial)) log nil) (#1# #2#))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_add_rule_preserves_every_selector_and_dynamic_option() {
    let elisp_form = r##"(let ((alert-internal-configuration nil)
                (persistent (lambda (info)
                              (eq (plist-get info :severity)
                                  'urgent)))
                (continue (lambda (info)
                            (plist-get info :data))))
         (alert-add-rule
          :severity '(urgent high)
          :status '(buried idle)
          :mode "erc-.*"
          :category "chat\\|mention"
          :title "^Team"
          :message "build"
          :predicate #'listp
          :icon "mail"
          :style 'fringe
          :persistent persistent
          :never-persist nil
          :continue continue)
         (let* ((rule (car alert-internal-configuration))
                (selectors (nth 0 rule))
                (options (nth 2 rule)))
           (list
            selectors
            (nth 1 rule)
            (mapcar
             (lambda (option)
               (list (car option)
                     (if (functionp (cdr option))
                         (funcall (cdr option)
                                  '(:severity urgent :data t))
                       (cdr option))))
             options))))"##;
    let expect = expect![[
        r#"OK (((:severity urgent high) (:status buried idle) (:mode . "erc-.*") (:category . "chat\\|mention") (:title . "^Team") (:message . "build") (:predicate . listp) (:icon . "mail")) fringe ((:persistent t) (:continue t)))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_rule_matching_ports_upstream_severity_capture_with_complete_info() {
    let elisp_form = r##"(let (captured)
         (alert-define-style
          'test-rule-severity
          :title "Rule Severity"
          :notifier (lambda (info) (setq captured info)))
         (let ((alert-default-style 'ignore)
               (alert-user-configuration nil)
               (alert-internal-configuration nil)
               (alert-active-alerts nil)
               (alert-log-messages nil)
               (alert-hide-all-notifications nil))
           (alert-add-rule
            :severity 'high :style 'test-rule-severity)
           (alert "High severity"
                  :severity 'high
                  :title "Build"
                  :category 'ci
                  :data '(:job 42))
           (list
            (mapcar
             (lambda (key) (list key (plist-get captured key)))
             '(:message :title :severity :category :mode
               :persistent :never-persist :id :data))
            (buffer-name (plist-get captured :buffer))
            (length alert-active-alerts))))"##;
    let expect = expect![[
        r#"OK (((:message "High severity") (:title "Build") (:severity high) (:category ci) (:mode lisp-interaction-mode) (:persistent nil) (:never-persist nil) (:id nil) (:data (:job 42))) "*scratch*" 1)"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_rule_selectors_match_title_message_category_mode_icon_and_predicate_together() {
    let elisp_form = r##"(let (captured)
         (alert-define-style
          'test-all-selectors
          :notifier (lambda (info) (setq captured info)))
         (let ((alert-default-style 'ignore)
               (alert-user-configuration
                '((((:mode . "\\`text-mode\\'")
                    (:category . "deploy")
                    (:title . "^Team")
                    (:message . "succeeded$")
                    (:icon . "mail")
                    (:predicate .
                     (lambda (info)
                       (equal (plist-get info :data)
                              '(:job 42)))))
                   test-all-selectors nil)))
               (alert-internal-configuration nil)
               (alert-active-alerts nil)
               (alert-log-messages nil))
           (with-temp-buffer
             (text-mode)
             (alert "Build succeeded"
                    :title "Team CI"
                    :category 'deploy
                    :icon "mail-unread"
                    :data '(:job 42))
             (list
              (and captured
                   (plist-get captured :message))
              (length alert-active-alerts)))))"##;
    let expect = expect![[r#"OK ("Build succeeded" 1)"#]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_nonmatching_rule_falls_back_to_default_style_once() {
    let elisp_form = r##"(let (calls)
         (alert-define-style
          'test-rule
          :title "Rule"
          :notifier (lambda (info)
                      (push (list 'rule
                                  (plist-get info :message))
                            calls)))
         (alert-define-style
          'test-fallback
          :notifier (lambda (info)
                      (push (list 'fallback
                                  (plist-get info :message))
                            calls)))
         (let ((alert-default-style 'test-fallback)
               (alert-user-configuration
                '((((:severity high)) test-rule nil)))
               (alert-internal-configuration nil)
               (alert-active-alerts nil)
               (alert-log-messages nil))
           (alert "Normal build" :severity 'normal)
           (list (nreverse calls)
                 (length alert-active-alerts))))"##;
    let expect = expect![[r#"OK (((fallback "Normal build")) 1)"#]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_continue_rules_dispatch_multiple_styles_then_stop() {
    let elisp_form = r##"(let (calls)
         (dolist (style '(first second third))
           (alert-define-style
            style
            :notifier
            (lambda (info)
              (push
               (list (plist-get info :style-probe)
                     (plist-get info :message))
               calls))))
         (let ((alert-default-style 'ignore)
               (alert-user-configuration
                '((nil first ((:continue . t)))
                  (nil second ((:continue . t)))
                  (nil third nil)))
               (alert-internal-configuration nil)
               (alert-active-alerts nil)
               (alert-log-messages nil))
           (cl-letf (((symbol-function 'alert-send-notification)
                      (lambda (buffer info style-def &rest options)
                        (push
                         (list
                          (plist-get style-def :title)
                          (plist-get info :message)
                          options
                          (buffer-name buffer))
                         calls))))
             (alert "Fan out")
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((nil "Fan out" (nil nil) "*scratch*") (nil "Fan out" (nil nil) "*scratch*") (nil "Fan out" (nil nil) "*scratch*"))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_persistent_and_never_persist_predicates_receive_full_info() {
    let elisp_form = r##"(let (sent predicate-calls)
         (alert-define-style 'test-persist :notifier #'ignore)
         (let ((alert-default-style 'ignore)
               (alert-user-configuration
                `((nil test-persist
                       ((:persistent .
                         ,(lambda (info)
                            (push
                             (list 'persistent
                                   (plist-get info :data))
                             predicate-calls)
                            t))
                        (:never-persist .
                         ,(lambda (info)
                            (push
                             (list 'never
                                   (plist-get info :severity))
                             predicate-calls)
                            nil))))))
               (alert-internal-configuration nil)
               (alert-log-messages nil))
           (cl-letf (((symbol-function 'alert-send-notification)
                      (lambda (_buffer info _style persist never)
                        (setq sent
                              (list
                               (plist-get info :persistent)
                               (plist-get info :never-persist)
                               persist never)))))
             (alert "Persist me"
                    :severity 'high
                    :data 'payload)
             (list sent (nreverse predicate-calls)))))"##;
    let expect = expect!["OK ((nil nil t nil) ((persistent payload) (never high)))"];
    assert_alert_parity(elisp_form, expect);
}
