use expect_test::expect;

use super::assert_aangit_parity;

#[test]
fn aangit_ng_new_missing_project_stops_before_every_side_effect() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'dired-read-dir-and-switches)
                     (lambda (&rest _)
                       '("/" "")))
                    ((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push (list 'message rendered) events)
                         rendered)))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)))
                    ((symbol-function 'dired)
                     (lambda (directory)
                       (push (list 'dired directory) events)))
                    ((symbol-function 'delete-other-windows)
                     (lambda ()
                       (push '(delete-other-windows) events)))
                    ((symbol-function 'aangit-menu--generate-submenu)
                     (lambda ()
                       (push '(submenu) events))))
                 (list
                  (aangit-menu--ng-new
                   '("--routing" "--style=scss"))
                  (nreverse events))))"##;
    let expect = expect![[r#"OK ("missing project name" ((message "missing project name")))"#]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_ng_new_uses_final_path_component_and_performs_side_effects_in_exact_order() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'dired-read-dir-and-switches)
                     (lambda (prompt)
                       (push (list 'read prompt) events)
                       '("/workspace/apps/my-app" "-al")))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'shell-result))
                    ((symbol-function 'dired)
                     (lambda (directory)
                       (push (list 'dired directory) events)
                       'dired-result))
                    ((symbol-function 'delete-other-windows)
                     (lambda ()
                       (push '(delete-other-windows) events)
                       'window-result))
                    ((symbol-function 'aangit-menu--generate-submenu)
                     (lambda ()
                       (push '(submenu) events)
                       'submenu-result)))
                 (list
                  (aangit-menu--ng-new
                   '("--routing" "--style=scss"))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (submenu-result ((read "") (shell "ng new --defaults my-app --routing --style=scss") (dired "my-app") (delete-other-windows) (shell "ng add --defaults --skip-confirmation @angular-eslint/schematics") (submenu)))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_component_command_handles_empty_and_strips_default_directory_from_all_args() {
    let elisp_form = r##"(let ((default-directory "/workspace/app/")
                    (answers
                     '("" "hero-card"))
                    events)
               (cl-letf
                   (((symbol-function 'read-string)
                     (lambda (prompt)
                       (push (list 'read prompt) events)
                       (pop answers)))
                    ((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push (list 'message rendered) events)
                         rendered)))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'shell-result)))
                 (list
                  (aangit-menu--ng-generate-component-command
                   '("--flat"))
                  (aangit-menu--ng-generate-component-command
                   '("--path=/workspace/app/src/app"
                     "--module=/workspace/app/admin"
                     "--flat"))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("missing component name" shell-result ((read "component name: ") (message "missing component name") (read "component name: ") (shell "ng generate component hero-card --defaults --path=src/app --module=admin --flat")))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_service_command_handles_empty_and_intentionally_ignores_transient_args() {
    let elisp_form = r##"(let ((answers
                     '("" "auth" "billing"))
                    events)
               (cl-letf
                   (((symbol-function 'read-string)
                     (lambda (prompt)
                       (push (list 'read prompt) events)
                       (pop answers)))
                    ((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push (list 'message rendered) events)
                         rendered)))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'shell-result)))
                 (list
                  (aangit-menu--ng-generate-service-command
                   '("--skip-tests"))
                  (aangit-menu--ng-generate-service-command
                   '("--skip-tests"))
                  (aangit-menu--ng-generate-service-command nil)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("missing service name" shell-result shell-result ((read "service name: ") (message "missing service name") (read "service name: ") (shell "ng generate service auth") (read "service name: ") (shell "ng generate service billing")))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_interface_command_handles_empty_and_uses_only_the_read_name() {
    let elisp_form = r##"(let ((answers
                     '("" "account"))
                    events)
               (cl-letf
                   (((symbol-function 'read-string)
                     (lambda (prompt)
                       (push (list 'read prompt) events)
                       (pop answers)))
                    ((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push (list 'message rendered) events)
                         rendered)))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'shell-result)))
                 (list
                  (aangit-menu--ng-generate-interface-command
                   '("--ignored"))
                  (aangit-menu--ng-generate-interface-command
                   '("--also-ignored"))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("missing interface name" shell-result ((read "interface name: ") (message "missing interface name") (read "interface name: ") (shell "ng generate interface account")))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_module_command_handles_empty_and_joins_transient_args_verbatim() {
    let elisp_form = r##"(let ((answers
                     '("" "admin"))
                    events)
               (cl-letf
                   (((symbol-function 'read-string)
                     (lambda (prompt)
                       (push (list 'read prompt) events)
                       (pop answers)))
                    ((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push (list 'message rendered) events)
                         rendered)))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'shell-result)))
                 (list
                  (aangit-menu--ng-generate-module-command
                   '("--flat"))
                  (aangit-menu--ng-generate-module-command
                   '("--force" "--routing" "--route=accounts"))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("missing module name" shell-result ((read "module name: ") (message "missing module name") (read "module name: ") (shell "ng generate module admin --defaults --force --routing --route=accounts")))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_npm_command_handles_empty_multiple_packages_and_joined_args() {
    let elisp_form = r##"(let ((answers
                     '("" "rxjs lodash"))
                    events)
               (cl-letf
                   (((symbol-function 'read-string)
                     (lambda (prompt)
                       (push (list 'read prompt) events)
                       (pop answers)))
                    ((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push (list 'message rendered) events)
                         rendered)))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'shell-result)))
                 (list
                  (aangit-menu--npm-install-command
                   '("--save-dev"))
                  (aangit-menu--npm-install-command
                   '("--save-dev" "--legacy-peer-deps"))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("missing package name" shell-result ((read "package name(s): ") (message "missing package name") (read "package name(s): ") (shell "npm install rxjs lodash --save-dev --legacy-peer-deps")))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}
