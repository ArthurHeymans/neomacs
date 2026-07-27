use expect_test::expect;

use super::{assert_ant_autoload_parity, assert_ant_parity};

#[test]
fn ant_registers_exact_feature_defaults_and_complete_function_surface() {
    let elisp_form = r##"(list
         (featurep 'ant)
         (mapcar
          (lambda (symbol)
            (list symbol (boundp symbol)
                  (and (boundp symbol) (symbol-value symbol))))
          '(ant-last-task ant-build-file-name ant-command
            *ant-tasks-cache* *ant-tasks-command* ant-tasks-default))
         (mapcar
          (lambda (symbol)
            (list symbol (fboundp symbol) (commandp symbol)
                  (help-function-arglist symbol t)))
          '(ant-find-tasks ant-tasks ant-get-task ant-find-root
            ant-kill-cache ant ant-last ant-compile ant-clean
            ant-test)))"##;
    let expect = expect![[
        r#"OK (t ((ant-last-task t "compile") (ant-build-file-name t "build.xml") (ant-command t "ant -emacs") (*ant-tasks-cache* t nil) (*ant-tasks-command* t "grep -e '<target.*name=\"[^-][^\"]*.*$'") (ant-tasks-default t ("compile" "test" "clean"))) ((ant-find-tasks t nil (directory)) (ant-tasks t nil (directory)) (ant-get-task t nil (directory)) (ant-find-root t nil (indicator)) (ant-kill-cache t t nil) (ant t t (&optional task)) (ant-last t t nil) (ant-compile t t nil) (ant-clean t t nil) (ant-test t t nil)))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_interactive_commands_and_documentation_are_exact() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (interactive-form symbol)
                 (documentation symbol t)))
         '(ant-kill-cache ant ant-last ant-compile ant-clean ant-test))"##;
    let expect = expect![[
        r#"OK ((ant-kill-cache (interactive nil) nil) (ant (interactive nil) "Run ant `task` in project root directory.") (ant-last (interactive nil) "Run the last ant task in project") (ant-compile (interactive nil) nil) (ant-clean (interactive nil) nil) (ant-test (interactive nil) nil))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_descriptor_records_exact_pin_and_installed_payload() {
    let elisp_form = r##"(let* ((description (cadr (assq 'ant package-alist)))
               (directory (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar
            (lambda (file)
              (let ((relative (file-relative-name file directory)))
                (list relative
                      (file-attribute-size (file-attributes file))
                      (secure-hash 'sha256 file))))
            (directory-files-recursively directory "." nil))
           (lambda (a b) (string< (car a) (car b))))))"##;
    let expect = expect![[
        r#"OK (ant "20160211.1543" nil "Helpers for compiling with ant." nil (("ant-autoloads.el" 963 "d0019c72cbce2f1d65b8ec05b1d954a5a6d9461678b69caaa3dab68ae15933e4") ("ant-pkg.el" 292 "eeccf5d50fca000f90312dcdb3bef38ace04694c2d7a722aa5fc066db7eba374") ("ant.el" 3506 "8e3e3c8114ec82d47a4b85e0afa953916ace5afec33172004586582fea0c2d7e") ("ant.elc" 2047 "92ee5b2358bcb2bfe3ffadf95800d479a51751049c6fd8017f78ea46479e9dc5")))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_autoloads_expose_only_public_commands_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'ant)
         (mapcar
          (lambda (symbol)
            (list symbol (fboundp symbol) (commandp symbol)
                  (autoloadp (symbol-function symbol))
                  (symbol-function symbol)))
          '(ant-kill-cache ant ant-last ant-compile ant-clean ant-test))
         (fboundp 'ant-find-root)
         (boundp 'ant-command)
         (boundp '*ant-tasks-cache*))"##;
    let expect = expect![[
        r#"OK (nil ((ant-kill-cache t t t (autoload "ant" nil t nil)) (ant t t t (autoload "ant" "Run ant `task` in project root directory.\n\n(fn &optional TASK)" t nil)) (ant-last t t t (autoload "ant" "Run the last ant task in project" t nil)) (ant-compile t t t (autoload "ant" nil t nil)) (ant-clean t t t (autoload "ant" nil t nil)) (ant-test t t t (autoload "ant" nil t nil))) nil nil nil)"#
    ]];
    assert_ant_autoload_parity(elisp_form, expect);
}

#[test]
fn ant_reload_preserves_runtime_configuration_and_cache_once() {
    let elisp_form = r##"(let ((ant-last-task "deploy")
               (ant-build-file-name "custom.xml")
               (ant-command "/opt/ant -emacs")
               (*ant-tasks-cache* '(("/project/" "ship")))
               (source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (list ant-last-task ant-build-file-name ant-command
               *ant-tasks-cache*
               (length
                (cl-remove-if-not
                 (lambda (feature) (eq feature 'ant))
                 features))))"##;
    let expect =
        expect![[r#"OK ("deploy" "custom.xml" "/opt/ant -emacs" (("/project/" "ship")) 1)"#]];
    assert_ant_parity(elisp_form, expect);
}
