use expect_test::expect;

use super::{assert_anybar_autoload_parity, assert_anybar_parity};

#[test]
fn installed_descriptor_source_and_feature_match_exact_melpa_transaction() {
    let elisp_form = r##"(let ((descriptor
                         (cadr
                          (assq
                           'anybar
                           package-alist))))
                     (list
                      (featurep 'anybar)
                      (package-desc-name descriptor)
                      (package-version-join
                       (package-desc-version descriptor))
                      (package-desc-reqs descriptor)
                      (package-desc-summary descriptor)
                      (file-name-nondirectory
                       (symbol-file
                        'anybar-send
                        'defun))
                      (and
                       (string=
                        (file-name-nondirectory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))
                        "anybar.el")
                       t)))"##;
    let expect = expect![[
        r#"OK (t anybar "20160816.1421" nil "Control AnyBar from Emacs." "anybar.el" t)"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_only_cookie_marked_commands_before_source_load() {
    let elisp_form = r##"(list
                      (featurep 'anybar)
                      (featurep
                       'anybar-autoloads)
                      (mapcar
                       (lambda (symbol)
                         (list
                          symbol
                          (fboundp symbol)
                          (and
                           (fboundp symbol)
                           (autoloadp
                            (symbol-function
                             symbol)))))
                       '(anybar-send
                         anybar-set
                         anybar-quit
                         anybar-start
                         anybar-images-reset
                         anybar--read-style
                         anybar--read-port))
                      (boundp
                       'anybar-default-port)
                      (and
                       (member
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))
                        load-path)
                       t))"##;
    let expect = expect![
        "OK (nil t ((anybar-send t t) (anybar-set t t) (anybar-quit t t) (anybar-start t t) (anybar-images-reset nil nil) (anybar--read-style nil nil) (anybar--read-port nil nil)) nil nil)"
    ];
    assert_anybar_autoload_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_first_command_call_loads_source_and_preserves_warning_result() {
    let elisp_form = r##"(let ((events nil))
                     (cl-letf
                         (((symbol-function
                            'display-warning)
                           (lambda
                             (type message
                                   &optional level buffer-name)
                             (push
                              (if
                                  (equal
                                   type
                                   "AnyBar")
                                  (list
                                   'package-warning
                                   type
                                   message
                                   level
                                   buffer-name)
                                (list
                                 'load-warning
                                 (car type)
                                 (cadr type)
                                 (file-name-nondirectory
                                  (caddr type))
                                 level
                                 buffer-name))
                              events)
                             'warned)))
                       (list
                        (featurep 'anybar)
                        (autoloadp
                         (symbol-function
                          'anybar-set))
                        (anybar-set
                         "not-a-style")
                        (featurep 'anybar)
                        (autoloadp
                         (symbol-function
                          'anybar-set))
                        anybar-default-port
                        anybar-images
                        (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil t warned t nil 1738 nil ((load-warning files missing-lexbind-cookie "anybar.el" :warning nil) (package-warning "AnyBar" "Not a style: not-a-style" nil nil)))"#
    ]];
    assert_anybar_autoload_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_has_exact_arglists_and_command_status() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list
                         function
                         (fboundp function)
                         (copy-tree
                          (help-function-arglist
                           function t))
                         (commandp function)))
                      '(anybar-images-reset
                        anybar--read-style
                        anybar--read-port
                        anybar-send
                        anybar-set
                        anybar-quit
                        anybar-start))"##;
    let expect = expect![
        "OK ((anybar-images-reset t nil t) (anybar--read-style t nil nil) (anybar--read-port t nil nil) (anybar-send t (command &optional port) t) (anybar-set t (style &optional port) t) (anybar-quit t (&optional port) t) (anybar-start t (&optional port) t))"
    ];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn constants_custom_metadata_group_and_initial_sandbox_state_match() {
    let elisp_form = r##"(list
                      anybar-default-port
                      anybar-styles
                      anybar-images
                      anybar-executable-location
                      (custom-variable-p
                       'anybar-executable-location)
                      (get
                       'anybar-executable-location
                       'custom-type)
                      (get
                       'anybar-executable-location
                       'safe-local-variable)
                      (get
                       'anybar-executable-location
                       'custom-group)
                      (get
                       'anybar
                       'custom-group)
                      (get
                       'anybar
                       'group-documentation)
                      (featurep 'anybar))"##;
    let expect = expect![[
        r#"OK (1738 ("white" "red" "orange" "yellow" "green" "cyan" "blue" "purple" "black" "question" "exclamation") nil "/Applications/AnyBar.app" ("/Applications/AnyBar.app") string stringp nil ((anybar-executable-location custom-variable)) "Control AnyBar from Emacs" t)"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn every_interactive_spec_preserves_prompt_order_and_helper_composition() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list
                         function
                         (interactive-form function)))
                      '(anybar-images-reset
                        anybar-send
                        anybar-set
                        anybar-quit
                        anybar-start
                        anybar--read-style
                        anybar--read-port))"##;
    let expect = expect![[
        r#"OK ((anybar-images-reset (interactive nil)) (anybar-send (interactive (list (read-string "Command: ") (anybar--read-port)))) (anybar-set (interactive (list (anybar--read-style) (anybar--read-port)))) (anybar-quit (interactive (list (anybar--read-port)))) (anybar-start (interactive (list (anybar--read-port)))) (anybar--read-style nil) (anybar--read-port nil))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn public_constants_are_process_global_while_discovered_images_can_be_buffer_local() {
    let elisp_form = r##"(let ((one
                          (generate-new-buffer
                           " *anybar-one*"))
                         (two
                          (generate-new-buffer
                           " *anybar-two*")))
                     (unwind-protect
                         (progn
                           (with-current-buffer one
                             (setq-local
                              anybar-images
                              '("one")))
                           (with-current-buffer two
                             (setq-local
                              anybar-images
                              '("two")))
                           (list
                            (with-current-buffer one
                              anybar-images)
                            (with-current-buffer two
                              anybar-images)
                            (default-value
                             'anybar-images)
                            (local-variable-p
                             'anybar-images
                             one)
                            (local-variable-p
                             'anybar-default-port
                             one)
                            (local-variable-p
                             'anybar-styles
                             two)))
                       (kill-buffer one)
                       (kill-buffer two)))"##;
    let expect = expect![[r#"OK (("one") ("two") nil t nil nil)"#]];
    assert_anybar_parity(elisp_form, expect);
}
