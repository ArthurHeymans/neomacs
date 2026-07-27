use expect_test::expect;

use super::{assert_android_env_autoload_parity, assert_android_env_parity};

#[test]
fn android_env_loads_the_exact_package_dependency_graph_and_installed_source() {
    let elisp_form = r##"(let* ((description
        (cadr (assq 'android-env package-alist)))
       (directory
        (file-name-as-directory (package-desc-dir description))))
  (list
   (featurep 'android-env)
   (package-installed-p 'android-env)
   (package-version-join (package-desc-version description))
   (mapcar
    (lambda (requirement)
      (list
       (car requirement)
       (package-version-join (cadr requirement))
       (or (package-installed-p (car requirement))
           (package-built-in-p (car requirement)))))
    (package-desc-reqs description))
   (directory-files directory nil
                    "\\`android-env\\(?:-autoloads\\|-pkg\\)?\\.el\\'")))"##;
    let expect = expect![[
        r#"OK (t t "20220810.1449" ((emacs "24.3" t) (s "1.12.0" t)) ("android-env-autoloads.el" "android-env-pkg.el" "android-env.el"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn custom_options_and_refactor_state_preserve_defaults_types_groups_docs_and_locality() {
    let elisp_form = r##"(list
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (copy-tree (symbol-value symbol))
     (get symbol 'custom-type)
     (get symbol 'custom-group)
     (get symbol 'standard-value)
     (local-variable-if-set-p symbol)))
  '(android-env-executable
    android-env-test-command
    android-env-emulator-command
    android-env-unit-test-command
    android-env-adb-buffer-name
    android-env-hydra))
 (list
  android-env-refactor-file
  (get 'android-env-refactor-file 'variable-documentation)
  (local-variable-if-set-p 'android-env-refactor-file))
 (get 'android-env 'custom-group))"##;
    let expect = expect![[
        r#"OK (((android-env-executable "./gradlew" string nil ("./gradlew") nil) (android-env-test-command "testDev" string nil ("testDev") nil) (android-env-emulator-command "emulator" string nil ("emulator") nil) (android-env-unit-test-command "testDevDebug" string nil ("testDevDebug") nil) (android-env-adb-buffer-name "*android-adb*" string nil ("*android-adb*") nil) (android-env-hydra nil boolean nil (nil) nil)) (nil "Path to file to be used by android-env-refactor." nil) ((android-env-executable custom-variable) (android-env-test-command custom-variable) (android-env-emulator-command custom-variable) (android-env-unit-test-command custom-variable) (android-env-adb-buffer-name custom-variable) (android-env-hydra custom-variable)))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_preserves_commands_arglists_interactive_specs_and_docs() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (commandp symbol)
    (help-function-arglist symbol t)
    (interactive-form symbol)
    (documentation symbol t)))
 '(android-env
   android-env-crashlytics
   android-env-gradle
   android-env-test
   android-env-unit-test
   android-env-unit-test-single
   android-env-avd-list
   android-env-avd
   android-env-adb
   android-env-auto-dhu
   android-env-logcat-clear
   android-env-logcat-buffer
   android-env-logcat
   android-env-logcat-crash
   android-env-logcat-pid-assoc
   android-env-logcat-pid
   android-env-uninstall-app
   android-env-deeplink
   android-env-refactor-map
   android-env-refactor-file-ensure
   android-env-refactor
   android-env-recursive-refactor
   android-env-compile
   android-env-hydra-setup
   android-env-compile-mode))"##;
    let expect = expect![[
        r#"OK ((android-env t t nil (interactive nil) "Set compilation error regexps.") (android-env-crashlytics t t (module build) (interactive "sModule: \nsBuild: ") "Assemble and upload the MODULE and BUILD to crashlytics.") (android-env-gradle t nil (gradle-cmd) nil "Execute GRADLE-CMD.") (android-env-test t t nil (interactive nil) "Execute instrumented test.") (android-env-unit-test t t nil (interactive nil) "Execute unit test.") (android-env-unit-test-single t t (build test) (interactive "sBuild: \nsTest: ") "Execute a single unit test from BUILD.\nWhose fully qualified jvm name is TEST.") (android-env-avd-list t nil nil nil "Return shell command output as list.") (android-env-avd t t nil (interactive nil) "Prompts for avd launch based on current avds.") (android-env-adb t nil nil nil "Return adb full path based on ANDROID_SDK_ROOT env var.") (android-env-auto-dhu t t nil (interactive nil) "Launches android auto desktop head unit.") (android-env-logcat-clear t t nil (interactive nil) "Clear android logcat.") (android-env-logcat-buffer t nil (&optional logcat-args) nil "Handles buffer related tasks using LOGCAT-ARGS.") (android-env-logcat t t (&optional tag) (interactive "sTag: ") "Show logcat using TAG for filtering.") (android-env-logcat-crash t t nil (interactive nil) "Show logcat's crash buffer.") (android-env-logcat-pid-assoc t nil (str) nil "Convert STR to assoc list with pid as car and process as cdr.") (android-env-logcat-pid t t nil (interactive nil) "Start logcat but filtering for a specific pid.") (android-env-uninstall-app t t (package) (interactive "sPackage: ") "Uninstall application by PACKAGE name.") (android-env-deeplink t t (deeplink) (interactive "sDeep link: ") "Send DEEPLINK to emulator.") (android-env-refactor-map t nil (file) nil "Return a list with FILE contents.\nFILE should be a comma separated file with pairs of intended replacements.\nTake for example this androidx migration mapping file:\nhttps://developer.android.com/topic/libraries/support-library/downloads/androidx-class-mapping.csv") (android-env-refactor-file-ensure t nil nil nil "Promps for filling ANDROID-ENV-REFACTOR-FILE when not configured.") (android-env-refactor t t nil (interactive nil) "Perform refactor on current buffer based on mappings file contents.\nMappings file path is stored for further usage at ANDROID-ENV-REFACTOR-FILE.\nWhen called with prefix it will prompt again for Mappings file.\nIt will return the number of replacements performed.") (android-env-recursive-refactor t t (match) (interactive "sMatch regexp: ") "Call ANDROID-ENV-REFACTOR for every file matching MATCH recursively.") (android-env-compile t t (task) (interactive "sTask: ") "Execute gradle compilation using TASK.") (android-env-hydra-setup t nil nil nil "Hydra setup.") (android-env-compile-mode t t nil (interactive nil) "Compilation mode for android compile.\n\nIn addition to any hooks its parent mode `compilation-mode' might have\nrun, this mode runs the hook `android-env-compile-mode-hook', as the\nfinal or penultimate step during initialization.\n\n\\{android-env-compile-mode-map}"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn compilation_error_patterns_cover_gradle_java_javac_and_kotlin_diagnostics_exactly() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (copy-tree (assq symbol compilation-error-regexp-alist-alist)))
 '(android-java android-java-2 android-kotlin))"##;
    let expect = expect![[
        r#"OK ((android-java ":compile.*?\\(/.*?\\):\\([0-9]+\\): " 1 2) (android-java-2 "^\\(/.[^:]*\\):\\([0-9]*\\): +error:" 1 2) (android-kotlin "^e: \\(.[^:]*\\): (\\([0-9]*\\), \\([0-9]*\\)" 1 2 3))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn compilation_error_patterns_extract_real_gradle_java_javac_and_kotlin_locations() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (let* ((symbol (car case))
          (line (cadr case))
          (entry
           (assq symbol compilation-error-regexp-alist-alist))
          (regexp (cadr entry))
          (matched (string-match regexp line)))
     (list
      symbol
      matched
      (mapcar
       (lambda (group)
         (and matched (match-string group line)))
       (delq nil (list (nth 2 entry)
                       (nth 3 entry)
                       (nth 4 entry)))))))
 '((android-java
    ":app:compileQaDebugJavaWithJavac /workspace/app/src/main/java/com/example/Checkout.java:73: incompatible types")
   (android-java-2
    "/workspace/core/src/main/java/com/example/Cart.java:19: error: cannot find symbol")
   (android-kotlin
    "e: /workspace/feature/src/main/kotlin/Payment.kt: (24, 17): Unresolved reference")))"##;
    let expect = expect![[
        r#"OK ((android-java 4 ("/workspace/app/src/main/java/com/example/Checkout.java" "73")) (android-java-2 0 ("/workspace/core/src/main/java/com/example/Cart.java" "19")) (android-kotlin 0 ("/workspace/feature/src/main/kotlin/Payment.kt" "24" "17")))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn android_compile_mode_derives_from_compilation_mode_and_installs_only_android_matchers() {
    let elisp_form = r##"(with-temp-buffer
  (insert "stale build output")
  (android-env-compile-mode)
  (list
   major-mode
   mode-name
   (derived-mode-p 'compilation-mode)
   compilation-error-regexp-alist
   buffer-read-only
   (buffer-string)
   (local-variable-p 'compilation-error-regexp-alist)))"##;
    let expect = expect![[
        r#"OK (android-env-compile-mode "Android Compile" compilation-mode (android-java android-java-2 android-kotlin) t "stale build output" t)"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_file_registers_no_commands_because_upstream_has_no_autoload_cookies() {
    let elisp_form = r##"(list
 (featurep 'android-env)
 (featurep 'android-env-autoloads)
 (mapcar
  (lambda (symbol)
    (list symbol
          (fboundp symbol)
          (and (fboundp symbol)
               (autoloadp (symbol-function symbol)))))
  '(android-env
    android-env-gradle
    android-env-logcat
    android-env-refactor))
 (boundp 'android-env-executable)
 (boundp 'android-env-refactor-file))"##;
    let expect = expect![
        "OK (nil t ((android-env nil nil) (android-env-gradle nil nil) (android-env-logcat nil nil) (android-env-refactor nil nil)) nil nil)"
    ];
    assert_android_env_autoload_parity(elisp_form, expect);
}

#[test]
fn installed_source_matches_the_exact_frozen_revision_and_complete_function_count() {
    let elisp_form = r##"(let ((source (locate-library "android-env")))
  (with-temp-buffer
    (insert-file-contents-literally source)
    (let ((contents (buffer-string)))
      (goto-char (point-min))
      (list
       (file-name-nondirectory source)
       (secure-hash 'sha256 contents)
       (count-lines (point-min) (point-max))
       (how-many "^(defun android-env")
       (string-match-p
        "Package-Version: 20220810\\.1449"
        contents)
       (string-match-p
        "Package-Revision: d2890f1156ed"
        contents)
       (string-match-p
        "Package-Requires: ((emacs \"24\\.3\") (s \"1\\.12\\.0\"))"
        contents)))))"##;
    let expect = expect![[
        r#"OK ("android-env.el" "4b8608904eda1f9e292dc4da5c0c427524fc169c96d4bf243e45ce626801d89b" 334 24 152 186 281)"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}
