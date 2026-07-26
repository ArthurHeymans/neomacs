use expect_test::expect;

use super::assert_aangit_parity;

#[test]
fn aangit_style_arguments_preserve_class_key_argument_reader_policy_and_choices() {
    let elisp_form = r##"(mapcar
               (lambda (symbol)
                 (let ((argument
                        (get symbol 'transient--suffix)))
                   (list
                    (eieio-object-class-name argument)
                    (oref argument description)
                    (oref argument key)
                    (oref argument argument)
                    (oref argument always-read)
                    (copy-tree (oref argument choices)))))
               '(aangit-menu--new-project-style
                 aangit-menu--new-component-style))"##;
    let expect = expect![[
        r#"OK ((transient-option "Style" "-y" "--style=" t ("css" "scss" "sass" "less")) (transient-option "Style" "-y" "--style=" t ("css" "scss" "sass" "less" "none")))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_new_project_layout_and_initial_value_match_the_source_definition() {
    let elisp_form = r##"(let ((prefix
                    (transient--init-prefix
                     'aangit-menu--new-project)))
               (list
                (transient--get-layout
                 'aangit-menu--new-project)
                (copy-tree (oref prefix value))))"##;
    let expect = expect![[
        r#"OK ([2 nil ([transient-column (:description "Switches") ((transient-switch :key "-s" :description "Standalone" :argument "--standalone" :command transient:aangit-menu--new-project:--standalone) (transient-switch :key "-r" :description "Routing" :argument "--routing" :command transient:aangit-menu--new-project:--routing) (transient-switch :key "-i" :description "Inline Style" :argument "--inline-style" :command transient:aangit-menu--new-project:--inline-style) (transient-switch :key "-t" :description "Inline Template" :argument "--inline-template" :command transient:aangit-menu--new-project:--inline-template) (transient-suffix :command aangit-menu--new-project-style) "" (transient-switch :key "-S" :description "Skip Tests" :argument "--skip-tests" :command transient:aangit-menu--new-project:--skip-tests))] [transient-column (:description "Commands") ((transient-suffix :key "n" :description "new" :command aangit-menu--ng-new))])] ("--standalone" "--routing" "--style=css"))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_component_layout_preserves_every_switch_option_reader_and_command() {
    let elisp_form = r##"(transient--get-layout
              'aangit-menu--generate-component-submenu)"##;
    let expect = expect![[
        r#"OK [2 nil ([transient-column (:description "generate component") ((transient-switch :key "-s" :description "Standalone" :argument "--standalone" :command transient:aangit-menu--generate-component-submenu:--standalone) (transient-switch :key "-i" :description "Inline Style" :argument "--inline-style" :command transient:aangit-menu--generate-component-submenu:--inline-style) (transient-switch :key "-t" :description "Inline Template" :argument "--inline-template" :command transient:aangit-menu--generate-component-submenu:--inline-template) (transient-option :key "-p" :description "Path" :argument "--path=" :command transient:aangit-menu--generate-component-submenu:--path= :always-read t :reader aangit--transient-read-directory-with-no-slash) (transient-option :key "-m" :description "Module" :argument "--module=" :command transient:aangit-menu--generate-component-submenu:--module= :always-read t) (transient-switch :key "-e" :description "Export" :argument "--export" :command transient:aangit-menu--generate-component-submenu:--export) (transient-switch :key "-f" :description "Flat" :argument "--flat" :command transient:aangit-menu--generate-component-submenu:--flat) (transient-suffix :command aangit-menu--new-component-style) "" (transient-switch :key "-S" :description "Skip Tests" :argument "--skip-tests" :command transient:aangit-menu--generate-component-submenu:--skip-tests))] [transient-column (:description "Commands") ((transient-suffix :key "n" :description "new" :command aangit-menu--ng-generate-component-command))])]"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_interface_service_and_module_layouts_match_the_source_definitions() {
    let elisp_form = r##"(list
               (transient--get-layout
                'aangit-menu--generate-interface-submenu)
               (transient--get-layout
                'aangit-menu--generate-service-submenu)
               (transient--get-layout
                'aangit-menu--generate-module-submenu))"##;
    let expect = expect![[
        r#"OK ([2 nil ([transient-column (:description "Interfaces") ((transient-suffix :key "n" :description "new" :command aangit-menu--ng-generate-interface-command))])] [2 nil ([transient-column (:description "Service") ((transient-switch :key "-S" :description "Skip Tests" :argument "--skip-tests" :command transient:aangit-menu--generate-service-submenu:--skip-tests))] [transient-column (:description "Commands") ((transient-suffix :key "n" :description "new" :command aangit-menu--ng-generate-service-command))])] [2 nil ([transient-column (:description "Module") ((transient-switch :key "-f" :description "Force" :argument "--force" :command transient:aangit-menu--generate-module-submenu:--force) (transient-switch :key "-F" :description "Flat" :argument "--flat" :command transient:aangit-menu--generate-module-submenu:--flat) (transient-switch :key "-r" :description "Routing" :argument "--routing" :command transient:aangit-menu--generate-module-submenu:--routing) (transient-option :key "-R" :description "Route" :argument "--route=" :command transient:aangit-menu--generate-module-submenu:--route= :always-read t) (transient-option :key "-m" :description "Module" :argument "--module=" :command transient:aangit-menu--generate-module-submenu:--module= :always-read t))] [transient-column (:description "Commands") ((transient-suffix :key "n" :description "new" :command aangit-menu--ng-generate-module-command))])])"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_generate_layout_and_initial_defaults_match_the_source_definition() {
    let elisp_form = r##"(let ((prefix
                    (transient--init-prefix
                     'aangit-menu--generate-submenu)))
               (list
                (transient--get-layout
                 'aangit-menu--generate-submenu)
                (copy-tree (oref prefix value))))"##;
    let expect = expect![[
        r#"OK ([2 nil ([transient-column (:description "Generate what?") ((transient-suffix :key "c" :description "Component" :command aangit-menu--generate-component-submenu) (transient-suffix :key "i" :description "Interface" :command aangit-menu--generate-interface-submenu) (transient-suffix :key "m" :description "Module" :command aangit-menu--generate-module-submenu) (transient-suffix :key "s" :description "Service" :command aangit-menu--generate-service-submenu))])] ("--defaults"))"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_external_schematic_layout_preserves_literal_package_arguments() {
    let elisp_form = r##"(transient--get-layout
              'aangit-menu--add-external-library-or-schematic-submenu)"##;
    let expect = expect![[
        r#"OK [2 nil ([transient-column (:description "Schematics") ((transient-switch :key "l" :description "@angular-eslint/schematics" :argument "@angular-eslint/schematics" :command transient:aangit-menu--add-external-library-or-schematic-submenu:@angular-eslint/schematics) (transient-switch :key "m" :description "@angular/material" :argument "@angular/material" :command transient:aangit-menu--add-external-library-or-schematic-submenu:@angular/material) (transient-switch :key "c" :description "@angular/cdk/schematics" :argument "@angular/cdk/schematics" :command transient:aangit-menu--add-external-library-or-schematic-submenu:@angular/cdk/schematics) (transient-switch :key "s" :description "@ngrx/store" :argument "@ngrx/store" :command transient:aangit-menu--add-external-library-or-schematic-submenu:@ngrx/store))] [transient-column (:description "Commands") ((transient-suffix :key "a" :description "add" :command aangit-menu--ng-add-known-schematic-command))])]"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}

#[test]
fn aangit_top_level_layout_routes_every_key_to_the_exact_submenu_or_command() {
    let elisp_form = r##"(transient--get-layout 'aangit-menu)"##;
    let expect = expect![[
        r#"OK [2 nil ([transient-columns nil ([transient-column (:description "ng") ((transient-suffix :key "n" :description "new" :command aangit-menu--new-project) (transient-suffix :key "a" :description "Add external library" :command aangit-menu--add-external-library-or-schematic-submenu) (transient-suffix :key "p" :description "Add npm package" :command aangit-menu--npm-install-command) (transient-suffix :key "g" :description "generate" :command aangit-menu--generate-submenu))])])]"#
    ]];

    assert_aangit_parity(elisp_form, expect);
}
