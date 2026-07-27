use expect_test::expect;

use super::assert_anybar_parity;

#[test]
fn set_routes_builtin_and_discovered_styles_to_default_and_explicit_ports() {
    let elisp_form = r##"(let ((anybar-images
                         '("company-logo"
                           "deploying"))
                        (events nil))
                     (cl-letf
                         (((symbol-function
                            'anybar-send)
                           (lambda
                             (command
                              &optional port)
                             (push
                              (list
                               command port)
                              events)
                             (list
                              'sent
                              command
                              port)))
                          ((symbol-function
                            'display-warning)
                           (lambda
                             (&rest arguments)
                             (push
                              (cons
                               'unexpected-warning
                               arguments)
                              events))))
                       (list
                        (anybar-set "green")
                        (anybar-set
                         "company-logo"
                         4242)
                        (anybar-set
                         "deploying"
                         0)
                        (nreverse events))))"##;
    let expect = expect![[
        r#"OK ((sent "green" 1738) (sent "company-logo" 4242) (sent "deploying" 0) (("green" 1738) ("company-logo" 4242) ("deploying" 0)))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn set_rejects_unknown_empty_and_case_mismatched_styles_with_exact_warnings() {
    let elisp_form = r##"(let ((anybar-images
                         '("custom"))
                        (events nil))
                     (cl-letf
                         (((symbol-function
                            'anybar-send)
                           (lambda
                             (&rest arguments)
                             (push
                              (cons
                               'unexpected-send
                               arguments)
                              events)))
                          ((symbol-function
                            'display-warning)
                           (lambda
                             (type message
                                   &optional level buffer-name)
                             (push
                              (list
                               type
                               message
                               level
                               buffer-name)
                              events)
                             (list
                              'warned
                              message))))
                       (list
                        (anybar-set "Green")
                        (anybar-set "" 9999)
                        (anybar-set "missing")
                        (nreverse events))))"##;
    let expect = expect![[
        r#"OK ((warned "Not a style: Green") (warned "Not a style: ") (warned "Not a style: missing") (("AnyBar" "Not a style: Green" nil nil) ("AnyBar" "Not a style: " nil nil) ("AnyBar" "Not a style: missing" nil nil)))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn set_builds_available_styles_without_mutating_public_or_discovered_lists() {
    let elisp_form = r##"(let* ((anybar-images
                          (list
                           "custom"
                           "green"))
                         (styles-before
                          (copy-tree
                           anybar-styles))
                         (images-before
                          (copy-tree
                           anybar-images))
                         (events nil))
                     (cl-letf
                         (((symbol-function
                            'anybar-send)
                           (lambda
                             (style
                              &optional port)
                             (push
                              (list style port)
                              events)))
                          ((symbol-function
                            'display-warning)
                           (lambda
                             (&rest arguments)
                             (push arguments events))))
                       (anybar-set "green")
                       (anybar-set "custom")
                       (list
                        (equal
                         anybar-styles
                         styles-before)
                        (equal
                         anybar-images
                         images-before)
                        anybar-styles
                        anybar-images
                        (nreverse events))))"##;
    let expect = expect![[
        r#"OK (t t ("white" "red" "orange" "yellow" "green" "cyan" "blue" "purple" "black" "question" "exclamation") ("custom" "green") (("green" 1738) ("custom" 1738)))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn quit_routes_exact_command_to_default_explicit_and_zero_ports() {
    let elisp_form = r##"(let ((events nil))
                     (cl-letf
                         (((symbol-function
                            'anybar-send)
                           (lambda
                             (command
                              &optional port)
                             (push
                              (list
                               command port)
                              events)
                             (length events))))
                       (list
                        (anybar-quit)
                        (anybar-quit 8080)
                        (anybar-quit 0)
                        (nreverse events))))"##;
    let expect = expect![[r#"OK (1 2 3 (("quit" 1738) ("quit" 8080) ("quit" 0)))"#]];
    assert_anybar_parity(elisp_form, expect);
}

#[test]
fn start_constructs_documented_environment_and_open_command_for_realistic_app_paths() {
    let elisp_form = r##"(let ((events nil))
                     (cl-letf
                         (((symbol-function
                            'shell-command)
                           (lambda
                             (command
                              &optional output-buffer
                              error-buffer)
                             (push
                              (list
                               command
                               output-buffer
                               error-buffer)
                              events)
                             (list
                              'launched
                              command))))
                       (let ((anybar-executable-location
                              "/Applications/AnyBar.app"))
                         (anybar-start))
                       (let ((anybar-executable-location
                              "/Users/demo/Applications/Any Bar Preview.app"))
                         (anybar-start 4242))
                       (let ((anybar-executable-location
                              "~/Applications/AnyBar.app"))
                         (anybar-start 0))
                       (nreverse events)))"##;
    let expect = expect![[
        r#"OK (("ANYBAR_PORT=1738 open -n /Applications/AnyBar.app" nil nil) ("ANYBAR_PORT=4242 open -n /Users/demo/Applications/Any Bar Preview.app" nil nil) ("ANYBAR_PORT=0 open -n ~/Applications/AnyBar.app" nil nil))"#
    ]];
    assert_anybar_parity(elisp_form, expect);
}
