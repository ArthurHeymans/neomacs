use super::assert_ace_popup_menu_autoload_parity;
use expect_test::expect;

#[test]
fn ace_popup_menu_autoload_file_registers_both_entry_points_without_loading_source() {
    let elisp_form = r##"(let ((symbols
              '(ace-popup-menu-mode
                ace-popup-menu)))
         (list
          (featurep
           'ace-popup-menu-autoloads)
          (featurep 'ace-popup-menu)
          (mapcar
           (lambda (symbol)
             (let ((function
                    (symbol-function symbol)))
               (list
                symbol
                (autoloadp function)
                (nth 1 function)
                (nth 3 function)
                (nth 4 function)
                (commandp symbol))))
           symbols)
          (boundp
           'ace-popup-menu-show-pane-header)
          (bound-and-true-p
           ace-popup-menu-mode)
          (copy-sequence
           (gethash "ace-popup-menu-"
                    definition-prefixes))))"##;
    let expect = expect![[
        r#"OK (t nil ((ace-popup-menu-mode t "ace-popup-menu" t nil t) (ace-popup-menu t "ace-popup-menu" nil nil nil)) nil nil nil)"#
    ]];
    assert_ace_popup_menu_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_direct_autoload_loads_source_and_dispatches_to_avy_menu() {
    let elisp_form = r##"(progn
         (require 'avy-menu)
         (setq ace-popup-menu--test-events
               nil)
         (cl-letf
             (((symbol-function 'avy-menu)
               (lambda (buffer menu header)
                 (push
                  (list
                   'avy
                   buffer
                   menu
                   header)
                  ace-popup-menu--test-events)
                 'avy-result))
              ((symbol-function
                'ace-popup-menu--test-original)
               (lambda (position menu)
                 (push
                  (list
                   'original
                   position
                   menu)
                  ace-popup-menu--test-events)
                 'original-result)))
           (list
            (ace-popup-menu
             #'ace-popup-menu--test-original
             '(10 20)
             '("Menu"
               ("Pane"
                ("Choice" . selected))))
            (featurep 'ace-popup-menu)
            (nreverse
             ace-popup-menu--test-events))))"##;
    let expect = expect![[
        r#"OK (avy-result t ((avy "*ace-popup-menu*" ("Menu" ("Pane" ("Choice" . selected))) nil)))"#
    ]];
    assert_ace_popup_menu_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_mode_autoload_loads_source_installs_advice_and_can_remove_it() {
    let elisp_form = r##"(progn
         (require 'avy-menu)
         (setq ace-popup-menu--test-events
               nil)
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
                   t
                   '("Menu"
                     ("Pane"
                      ("Choice" . selected))))
                  (featurep 'ace-popup-menu)
                  ace-popup-menu-mode
                  (and
                   (advice-member-p
                    #'ace-popup-menu
                    'x-popup-menu)
                   t)
                  (nreverse
                   ace-popup-menu--test-events)))
             (ace-popup-menu-mode -1))))"##;
    let expect = expect![[
        r#"OK (avy-result t t t ((avy "*ace-popup-menu*" ("Menu" ("Pane" ("Choice" . selected))) nil)))"#
    ]];
    assert_ace_popup_menu_autoload_parity(elisp_form, expect);
}
