use expect_test::expect;

use super::{assert_angular_mode_autoload_parity, assert_angular_mode_parity};

#[test]
fn angular_mode_registers_exact_feature_group_command_and_parent() {
    let elisp_form = r##"(list
         (featurep 'angular-mode)
         (commandp 'angular-mode)
         (interactive-form 'angular-mode)
         (get 'angular 'custom-group)
         (get 'angular 'group-documentation)
         (get 'angular-mode
              'derived-mode-parent)
         (documentation 'angular-mode)
         (help-function-arglist
          'angular-mode t))"##;
    let expect = expect![[
        r#"OK (t t (interactive nil) nil "Major mode for AngularJS." javascript-mode "Major mode for AngularJS.\n\nUses keymap ‘javascript-mode-map’, which is not currently defined.\n\n\nIn addition to any hooks its parent mode ‘javascript-mode’ might have\nrun, this mode runs the hook ‘angular-mode-hook’, as the final or\npenultimate step during initialization." nil)"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_public_keyword_tables_are_complete_and_exact() {
    let elisp_form = r##"(list
         (list
          (length
           angular-controller-definition-keywords)
          angular-controller-definition-keywords)
         (list
          (length
           angular-directive-definition-keywords)
          angular-directive-definition-keywords)
         (list
          (length angular-global-api-keywords)
          (secure-hash
           'sha256
           (prin1-to-string
            angular-global-api-keywords))
          (car angular-global-api-keywords)
          (car
           (last angular-global-api-keywords)))
         (list
          (length angular-services-keywords)
          (secure-hash
           'sha256
           (prin1-to-string
            angular-services-keywords))
          (car angular-services-keywords)
          (car
           (last angular-services-keywords)))
         (list
          (length angular-mocha-keywords)
          angular-mocha-keywords)
         (length angular-font-lock-keywords)
         (secure-hash
          'sha256
          (prin1-to-string
           angular-font-lock-keywords)))"##;
    let expect = expect![[
        r#"OK ((0 nil) (6 ("controller:" "controllerAs:" "link:" "scope:" "templateUrl:" "transclude:")) (45 "547153d8b12b26a711388ad0b8f0fc179252eb5a43bc14124d36f5561c7d0a9a" "angular.bind" "$id") (24 "b9c2802e6465bbe6d70836d1c67310d8f7924c8175e62e1d7214cf36379e4881" "$anchorScroll" "$window") (5 ("describe(" "beforeEach(" "before(" "afterEach(" "it(")) 5 "29cc67207d868c397b1e225e520fb1967cdf6ed643fe0f7ea681aefd7fbe3608")"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_descriptor_records_exact_pin_and_all_payload_files() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq
                            'angular-mode
                            package-alist)))
               (directory
                (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join
           (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar
            (lambda (file)
              (file-relative-name
               file directory))
            (directory-files-recursively
             directory "." nil))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (angular-mode "20151201.2127" nil "Major mode for Angular.js." nil ("README-elpa" "angular-html-mode.el" "angular-html-mode.elc" "angular-mode-autoloads.el" "angular-mode-pkg.el" "angular-mode.el" "angular-mode.elc" "snippets/angular-html-mode/ngclick" "snippets/angular-mode/config" "snippets/angular-mode/controller" "snippets/angular-mode/module" "snippets/angular-mode/stateprovider"))"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_installed_yasnippets_preserve_exact_practical_templates() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq
                            'angular-mode
                            package-alist)))
               (directory
                (package-desc-dir description))
               (files
                '("snippets/angular-html-mode/ngclick"
                  "snippets/angular-mode/config"
                  "snippets/angular-mode/controller"
                  "snippets/angular-mode/module"
                  "snippets/angular-mode/stateprovider")))
         (mapcar
          (lambda (relative)
            (let ((absolute
                   (expand-file-name
                    relative directory)))
              (with-temp-buffer
                (insert-file-contents
                 absolute)
                (list
                 relative
                 (buffer-size)
                 (secure-hash
                  'sha256
                  (current-buffer))
                 (buffer-string)))))
          files))"##;
    let expect = expect![[
        r##"OK (("snippets/angular-html-mode/ngclick" 90 "58e5ff93788c694a9f5246ef9e5bf2f42c2f8a525105f5d7b1e18bd195b289e9" "# -*- mode: snippet -*-\n# name: ngClick\n# --\n<${1:tag} ng-click=\"${2:expression}\">$0</$1>\n") ("snippets/angular-mode/config" 97 "5419f7ea66bbf046cc035d83988d469afff89ae0687ac81419121e63fd2865f4" "# -*- mode: snippet -*-\n# name: config\n# --\n.config(function config(${1:dependencies}) {\n  $0\n})\n") ("snippets/angular-mode/controller" 128 "29ce51c1ec2e03f80d818a0e049972e5a6e4f5f970c8ae7aca346500f3e6d13b" "# -*- mode: snippet -*-\n# name: controller\n# --\n.controller('${1:name}Ctrl', function $1Controller(${2:dependencies}) {\n  $0\n})\n") ("snippets/angular-mode/module" 100 "a5d43c02f0cd90f6c4aea7c1f998bc93d65992d65ef5171ca36a1d3ffdfd2344" "# -*- mode: snippet -*-\n# name: module\n# --\nangular.module('${1:name}', [\n  ${2:dependencies}\n])\n$0\n") ("snippets/angular-mode/stateprovider" 237 "7d0675216cab6672dfd67aaf24c61ac311fa345ef629c3ccbf0fade7c28c1b58" "# -*- mode: snippet -*-\n# name: stateProvider\n# --\n$stateProvider.state('${1:name}', {\n  url: '${2:route}',\n  views: {\n    'main': {\n      controller: '${3:controller}',\n      templateUrl: '${4:templateUrl}'\n    }\n  },\n  data: {\n  }\n});\n"))"##
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_autoloads_expose_both_modes_without_loading_features() {
    let elisp_form = r##"(list
         (featurep 'angular-mode)
         (featurep 'angular-html-mode)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (commandp symbol)
             (autoloadp
              (symbol-function symbol))
             (symbol-function symbol)))
          '(angular-mode
            angular-html-mode)))"##;
    let expect = expect![[
        r#"OK (nil nil ((angular-mode t t (autoload "angular-mode" "Major mode for AngularJS.\n\\{javascript-mode-map}\n\nIn addition to any hooks its parent mode `javascript-mode' might have\nrun, this mode runs the hook `angular-mode-hook', as the final or\npenultimate step during initialization." t nil)) (angular-html-mode t t (autoload "angular-html-mode" "Major HTML mode for AngularJS.\n\\{html-mode-map}\n\nIn addition to any hooks its parent mode `html-mode' might have run,\nthis mode runs the hook `angular-html-mode-hook', as the final or\npenultimate step during initialization." t nil))))"#
    ]];
    assert_angular_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn angular_mode_loads_html_sibling_and_preserves_distinct_features() {
    let elisp_form = r##"(let* ((directory
                          (file-name-directory
                           (getenv
                            "NEOMACS_PACKAGE_SOURCE")))
               (html-source
                (expand-file-name
                 "angular-html-mode.el"
                 directory)))
         (load html-source nil t t)
         (list
          (featurep 'angular-mode)
          (featurep 'angular-html-mode)
          (commandp 'angular-mode)
          (commandp 'angular-html-mode)
          (get 'angular-mode
               'derived-mode-parent)
          (get 'angular-html-mode
               'derived-mode-parent)))"##;
    let expect = expect!["OK (t t t t javascript-mode html-mode)"];
    assert_angular_mode_parity(elisp_form, expect);
}
