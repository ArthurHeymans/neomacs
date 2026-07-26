use super::assert_ace_popup_menu_parity;
use expect_test::expect;

#[test]
fn ace_popup_menu_mode_lifecycle_manages_one_advice_and_runs_hook_each_call() {
    let elisp_form = r##"(progn
         (ace-popup-menu-mode -1)
         (setq ace-popup-menu--test-events nil)
         (cl-letf
             (((symbol-function
                'ace-popup-menu--test-hook)
               (lambda ()
                 (push
                  (list
                   'hook
                   ace-popup-menu-mode
                   (and
                    (advice-member-p
                     #'ace-popup-menu
                     'x-popup-menu)
                    t))
                  ace-popup-menu--test-events)))
              ((symbol-function
                'ace-popup-menu--test-count-advice)
               (lambda (function _properties)
                 (when (eq
                        function
                        #'ace-popup-menu)
                   (setq
                    ace-popup-menu--test-advice-count
                    (1+
                     ace-popup-menu--test-advice-count))))))
           (let ((ace-popup-menu-mode-hook
                  '(ace-popup-menu--test-hook)))
             (unwind-protect
                 (list
                  (list
                   ace-popup-menu-mode
                   (and
                    (advice-member-p
                     #'ace-popup-menu
                     'x-popup-menu)
                    t))
                  (progn
                    (ace-popup-menu-mode 1)
                    (setq
                     ace-popup-menu--test-advice-count
                     0)
                    (advice-mapc
                     #'ace-popup-menu--test-count-advice
                     'x-popup-menu)
                    (list
                     ace-popup-menu-mode
                     (and
                      (advice-member-p
                       #'ace-popup-menu
                       'x-popup-menu)
                      t)
                     ace-popup-menu--test-advice-count))
                  (progn
                    (ace-popup-menu-mode 1)
                    (setq
                     ace-popup-menu--test-advice-count
                     0)
                    (advice-mapc
                     #'ace-popup-menu--test-count-advice
                     'x-popup-menu)
                    (list
                     ace-popup-menu-mode
                     (and
                      (advice-member-p
                       #'ace-popup-menu
                       'x-popup-menu)
                      t)
                     ace-popup-menu--test-advice-count))
                  (progn
                    (ace-popup-menu-mode -1)
                    (list
                     ace-popup-menu-mode
                     (and
                      (advice-member-p
                       #'ace-popup-menu
                       'x-popup-menu)
                      t)))
                  (progn
                    (ace-popup-menu-mode -1)
                    (list
                     ace-popup-menu-mode
                     (and
                      (advice-member-p
                       #'ace-popup-menu
                       'x-popup-menu)
                      t)))
                  (nreverse
                   ace-popup-menu--test-events))
               (ace-popup-menu-mode -1)))))"##;
    let expect = expect![
        "OK ((nil nil) (t t 1) (t t 1) (nil nil) (nil nil) ((hook t t) (hook t t) (hook nil nil) (hook nil nil)))"
    ];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_mode_nil_and_toggle_arguments_follow_global_minor_mode_semantics() {
    let elisp_form = r##"(progn
         (ace-popup-menu-mode -1)
         (unwind-protect
             (mapcar
              (lambda (argument)
                (ace-popup-menu-mode
                 argument)
                (list
                 argument
                 ace-popup-menu-mode
                 (default-value
                  'ace-popup-menu-mode)
                 (and
                  (advice-member-p
                   #'ace-popup-menu
                   'x-popup-menu)
                  t)))
              '(nil toggle toggle
                0 2 -3))
           (ace-popup-menu-mode -1)))"##;
    let expect = expect![
        "OK ((nil t t t) (toggle nil nil nil) (toggle t t t) (0 nil nil nil) (2 t t t) (-3 nil nil nil))"
    ];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_mode_interactive_prefixes_toggle_enable_and_disable_globally() {
    let elisp_form = r##"(progn
         (ace-popup-menu-mode -1)
         (unwind-protect
             (mapcar
              (lambda (prefix)
                (let ((current-prefix-arg
                       prefix))
                  (call-interactively
                   'ace-popup-menu-mode)
                  (list
                   prefix
                   ace-popup-menu-mode
                   (default-value
                    'ace-popup-menu-mode)
                   (and
                    (advice-member-p
                     #'ace-popup-menu
                     'x-popup-menu)
                    t))))
              '(nil (4) (-1) (0)))
           (ace-popup-menu-mode -1)))"##;
    let expect = expect!["OK ((nil t t t) ((4) t t t) ((-1) nil nil nil) ((0) nil nil nil))"];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_mode_actual_advice_routes_supported_menu_without_original_call() {
    let elisp_form = r##"(progn
         (ace-popup-menu-mode -1)
         (setq ace-popup-menu--test-events nil)
         (cl-letf
             (((symbol-function 'x-popup-menu)
               (lambda (position menu)
                 (push
                  (list
                   'original
                   position
                   menu)
                  ace-popup-menu--test-events)
                 'original-result))
              ((symbol-function 'avy-menu)
               (lambda (buffer menu header)
                 (push
                  (list
                   'avy
                   buffer
                   menu
                   header)
                  ace-popup-menu--test-events)
                 'avy-result)))
           (unwind-protect
               (progn
                 (ace-popup-menu-mode 1)
                 (list
                  (x-popup-menu
                   '(10 20)
                   '("Menu"
                     ("Pane"
                      ("Choice" . selected))))
                  (nreverse
                   ace-popup-menu--test-events)))
             (ace-popup-menu-mode -1))))"##;
    let expect = expect![[
        r#"OK (avy-result ((avy "*ace-popup-menu*" ("Menu" ("Pane" ("Choice" . selected))) nil)))"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_mode_actual_advice_preserves_original_fallback_contract() {
    let elisp_form = r##"(progn
         (ace-popup-menu-mode -1)
         (setq ace-popup-menu--test-events nil)
         (cl-letf
             (((symbol-function 'x-popup-menu)
               (lambda (position menu)
                 (push
                  (list
                   'original
                   position
                   menu)
                  ace-popup-menu--test-events)
                 'original-result))
              ((symbol-function 'avy-menu)
               (lambda (&rest arguments)
                 (push
                  (cons 'avy arguments)
                  ace-popup-menu--test-events)
                 'avy-result)))
           (unwind-protect
               (progn
                 (ace-popup-menu-mode 1)
                 (list
                  (x-popup-menu
                   nil
                   '("Menu"
                     ("Pane"
                      ("Choice" . selected))))
                  (nreverse
                   ace-popup-menu--test-events)))
             (ace-popup-menu-mode -1))))"##;
    let expect = expect![[
        r#"OK (original-result ((original nil ("Menu" ("Pane" ("Choice" . selected))))))"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_mode_disable_preserves_unrelated_advice() {
    let elisp_form = r##"(progn
         (ace-popup-menu-mode -1)
         (setq ace-popup-menu--test-events nil)
         (cl-letf
             (((symbol-function
                'ace-popup-menu--test-unrelated)
               (lambda (original &rest arguments)
                 (push
                  'unrelated
                  ace-popup-menu--test-events)
                 (apply original arguments))))
           (unwind-protect
               (progn
                 (advice-add
                  'x-popup-menu
                  :around
                  #'ace-popup-menu--test-unrelated)
                 (ace-popup-menu-mode 1)
                 (ace-popup-menu-mode -1)
                 (list
                  (and
                   (advice-member-p
                    #'ace-popup-menu
                    'x-popup-menu)
                   t)
                  (and
                   (advice-member-p
                    #'ace-popup-menu--test-unrelated
                    'x-popup-menu)
                   t)
                  ace-popup-menu-mode))
             (advice-remove
              'x-popup-menu
              #'ace-popup-menu--test-unrelated)
             (ace-popup-menu-mode -1))))"##;
    let expect = expect!["OK (nil t nil)"];
    assert_ace_popup_menu_parity(elisp_form, expect);
}
