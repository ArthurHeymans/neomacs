use expect_test::expect;

use super::assert_aangit_parity;

#[test]
fn aangit_public_surface_dependencies_commands_and_descriptions_match_the_pin() {
    let elisp_form = r##"(list
               (featurep 'aangit)
               (mapcar #'featurep '(transient dired s))
               (mapcar
                #'fboundp
                '(aangit-menu--ng-new
                  aangit--ng-add-single-schematic
                  aangit--transient-read-directory-with-no-slash
                  aangit-menu--ng-add-known-schematic-command
                  aangit-menu--new-project-style
                  aangit-menu--new-component-style
                  aangit-menu--new-project
                  aangit-menu--unimplemented
                  aangit-menu--ng-generate-component-command
                  aangit-menu--generate-component-submenu
                  aangit-menu--ng-generate-service-command
                  aangit-menu--ng-generate-interface-command
                  aangit-menu--ng-generate-module-command
                  aangit-menu--npm-install-command
                  aangit-menu--generate-interface-submenu
                  aangit-menu--generate-service-submenu
                  aangit-menu--generate-module-submenu
                  aangit-menu--generate-submenu
                  aangit-menu--add-external-library-or-schematic-submenu
                  aangit-menu))
               (mapcar
                #'commandp
                '(aangit-menu--ng-new
                  aangit-menu--ng-add-known-schematic-command
                  aangit-menu--new-project
                  aangit-menu--unimplemented
                  aangit-menu--ng-generate-component-command
                  aangit-menu--generate-component-submenu
                  aangit-menu--ng-generate-service-command
                  aangit-menu--ng-generate-interface-command
                  aangit-menu--ng-generate-module-command
                  aangit-menu--npm-install-command
                  aangit-menu--generate-interface-submenu
                  aangit-menu--generate-service-submenu
                  aangit-menu--generate-module-submenu
                  aangit-menu--generate-submenu
                  aangit-menu--add-external-library-or-schematic-submenu
                  aangit-menu))
               (mapcar
                (lambda (command)
                  (let ((suffix
                         (get command 'transient--suffix)))
                    (oref suffix description)))
                '(aangit-menu--ng-new
                  aangit-menu--ng-add-known-schematic-command
                  aangit-menu--ng-generate-component-command
                  aangit-menu--ng-generate-service-command
                  aangit-menu--ng-generate-interface-command
                  aangit-menu--ng-generate-module-command
                  aangit-menu--npm-install-command)))"##;
    let expect = expect![[
        r#"OK (t (t t t) (t t t t t t t t t t t t t t t t t t t t) (t t t t t t t t t t t t t t t t) ("ng new" "ng add" "ng generate component" "ng generate service" "ng generate interface" "ng generate module" "npm install package"))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_directory_reader_forwards_prompt_relativizes_and_removes_all_trailing_slashes() {
    let elisp_form = r##"(let ((default-directory "/workspace/project/")
                    (answers
                     '("/workspace/project/src/app/"
                       "/workspace/project/"
                       "/workspace/shared///"))
                    prompts)
               (cl-letf
                   (((symbol-function 'read-directory-name)
                     (lambda (prompt &rest _)
                       (push prompt prompts)
                       (pop answers))))
                 (list
                  (aangit--transient-read-directory-with-no-slash
                   "first: " "ignored" 'ignored-history)
                  (aangit--transient-read-directory-with-no-slash
                   "second: " nil nil)
                  (aangit--transient-read-directory-with-no-slash
                   "third: " nil nil)
                  (nreverse prompts))))"##;
    let expect = expect![[r#"OK ("src/app" "." "../shared" ("first: " "second: " "third: "))"#]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_single_schematic_formats_one_exact_noninteractive_shell_command() {
    let elisp_form = r##"(let (commands)
               (cl-letf
                   (((symbol-function 'shell-command)
                     (lambda (command)
                       (push command commands)
                       'shell-result)))
                 (list
                  (aangit--ng-add-single-schematic
                   "@scope/pkg@2")
                  (nreverse commands))))"##;
    let expect =
        expect![[r#"OK (shell-result ("ng add --defaults --skip-confirmation @scope/pkg@2"))"#]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_known_schematic_rejects_empty_args_without_shelling_out() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push (list 'message rendered) events)
                         rendered)))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'unexpected-shell)))
                 (list
                  (aangit-menu--ng-add-known-schematic-command nil)
                  (nreverse events))))"##;
    let expect = expect![[r#"OK ("missing schematic name" ((message "missing schematic name")))"#]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_known_schematic_installs_every_arg_in_order_and_returns_the_original_list() {
    let elisp_form = r##"(let (commands)
               (cl-letf
                   (((symbol-function 'shell-command)
                     (lambda (command)
                       (push command commands)
                       'shell-result)))
                 (let* ((args
                         '("@angular/material"
                           "@ngrx/store"
                           "custom-schematic"))
                        (result
                         (aangit-menu--ng-add-known-schematic-command
                          args)))
                   (list
                    result
                    (eq result args)
                    (nreverse commands)))))"##;
    let expect = expect![[
        r#"OK (("@angular/material" "@ngrx/store" "custom-schematic") t ("ng add --defaults --skip-confirmation @angular/material" "ng add --defaults --skip-confirmation @ngrx/store" "ng add --defaults --skip-confirmation custom-schematic"))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_unimplemented_command_is_interactive_and_emits_its_exact_message() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push rendered events)
                         rendered))))
                 (list
                  (interactive-form 'aangit-menu--unimplemented)
                  (aangit-menu--unimplemented)
                  (nreverse events))))"##;
    let expect =
        expect![[r#"OK ((interactive nil) "not yet implemented" ("not yet implemented"))"#]];

    assert_aangit_parity(elisp_form, expect);
}
