use expect_test::expect;

use super::assert_android_mode_parity;

#[test]
fn local_sdk_resolution_prefers_valid_project_property_then_environment_custom_and_error() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (root
                            (file-name-as-directory
                             (expand-file-name
                              "sdk-project"
                              sandbox)))
                           (local-sdk
                            (expand-file-name
                             "local-sdk"
                             sandbox))
                           (environment-sdk
                            (expand-file-name
                             "environment-sdk"
                             sandbox))
                           (custom-sdk
                            (expand-file-name
                             "custom-sdk"
                             sandbox))
                           (properties
                            (expand-file-name
                             "local.properties"
                             root)))
                      (make-directory root t)
                      (make-directory local-sdk t)
                      (make-directory environment-sdk t)
                      (make-directory custom-sdk t)
                      (cl-letf
                          (((symbol-function 'android-root)
                            (lambda () root)))
                        (with-temp-file properties
                          (insert
                           "name=value\nsdk.dir="
                           local-sdk
                           "\n"))
                        (setenv
                         "ANDROID_HOME"
                         environment-sdk)
                        (let ((android-mode-sdk-dir
                               custom-sdk))
                          (let ((local
                                 (android-local-sdk-dir)))
                            (with-temp-file properties
                              (insert
                               "sdk.dir="
                               (expand-file-name
                                "missing-sdk"
                                sandbox)
                               "\n"))
                            (let ((environment
                                   (android-local-sdk-dir)))
                              (setenv "ANDROID_HOME" nil)
                              (let ((custom
                                     (android-local-sdk-dir)))
                                (setq
                                 android-mode-sdk-dir nil)
                                (list
                                 local
                                 environment
                                 custom
                                 (condition-case
                                     error-data
                                     (android-local-sdk-dir)
                                   (error
                                    error-data)))))))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/local-sdk" "[ORACLE-SANDBOX]/environment-sdk" "[ORACLE-SANDBOX]/custom-sdk" (error "No SDK directory found"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn tool_lookup_honors_subdirectory_and_extension_order_on_real_sdk_files() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (sdk
                            (expand-file-name
                             "sdk-tools"
                             sandbox))
                           (android-mode-sdk-tool-subdirs
                            '("emulator"
                              "tools"
                              "platform-tools"))
                           (android-mode-sdk-tool-extensions
                            '("" ".bat" ".exe")))
                      (dolist
                          (directory
                           android-mode-sdk-tool-subdirs)
                        (make-directory
                         (expand-file-name
                          directory sdk)
                         t))
                      (dolist
                          (file
                           '("emulator/emulator.exe"
                             "tools/android.bat"
                             "platform-tools/adb"))
                        (with-temp-file
                            (expand-file-name file sdk)
                          (insert "tool")))
                      (cl-letf
                          (((symbol-function
                             'android-local-sdk-dir)
                            (lambda () sdk)))
                        (list
                         (android-tool-path
                          "emulator")
                         (android-tool-path
                          "android")
                         (android-tool-path "adb")
                         (condition-case error-data
                             (android-tool-path
                              "missing")
                           (error error-data)))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/sdk-tools/emulator/emulator.exe" "[ORACLE-SANDBOX]/sdk-tools/tools/android.bat" "[ORACLE-SANDBOX]/sdk-tools/platform-tools/adb" (error "Can’t find SDK tool: missing"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn installed_target_and_avd_parsers_preserve_sdk_order_and_signal_empty_catalogs() {
    let elisp_form = r##"(let (commands empty)
                      (cl-letf
                          (((symbol-function
                             'android-tool-path)
                            (lambda (_name)
                              "/sdk/tools/android"))
                           ((symbol-function
                             'shell-command-to-string)
                            (lambda (command)
                              (push command commands)
                              (if empty
                                  "Available Android targets:\n"
                                (if
                                    (string-match-p
                                     "target$"
                                     command)
                                    "Available Android targets:\n----------\nid: 1 or \"android-28\"\n----------\nid: 2 or \"Google Inc.:Google APIs:34\"\n"
                                  "Available Android Virtual Devices:\n    Name: Pixel_API_30\n    Path: /avd/one\n---------\n    Name: Tablet API 34\n")))))
                        (let ((targets
                               (android-list-targets))
                              (avds
                               (android-list-avd)))
                          (setq empty t)
                          (list
                           targets
                           avds
                           (condition-case error-data
                               (android-list-targets)
                             (error error-data))
                           (condition-case error-data
                               (android-list-avd)
                             (error error-data))
                           (nreverse commands)))))"##;
    let expect = expect![[
        r#"OK (("android-28" "Google Inc.:Google APIs:34") ("Pixel_API_30" "Tablet API 34") (error "No Android Targets found") (error "No Android Virtual Devices found") ("/sdk/tools/android list target" "/sdk/tools/android list avd" "/sdk/tools/android list target" "/sdk/tools/android list avd"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn project_creation_builds_gradle_and_ant_commands_opens_success_and_preserves_sdk_errors() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (project-path
                            (expand-file-name
                             "created project"
                             sandbox))
                           commands
                           opened
                           response)
                      (cl-letf
                          (((symbol-function
                             'completing-read)
                            (lambda (&rest _)
                              "android-34"))
                           ((symbol-function
                             'android-list-targets)
                            (lambda ()
                              '("android-34")))
                           ((symbol-function
                             'android-tool-path)
                            (lambda (_name)
                              "/sdk/tools/android"))
                           ((symbol-function
                             'shell-command-to-string)
                            (lambda (command)
                              (push command commands)
                              response))
                           ((symbol-function 'find-file)
                            (lambda (path)
                              (push path opened)
                              'opened)))
                        (setq response
                              "Created project")
                        (let ((android-mode-builder
                               'gradle)
                              (android-mode-gradle-plugin
                               "8.2.0"))
                          (android-create-project
                           project-path
                           "com.example.demo"
                           "MainActivity"))
                        (let ((android-mode-builder
                               'ant)
                              (android-mode-gradle-plugin
                               nil))
                          (android-create-project
                           project-path
                           "com.example.legacy"
                           "LegacyActivity"))
                        (setq response
                              "Error: target missing")
                        (let ((android-mode-builder
                               'gradle))
                          (list
                           (condition-case error-data
                               (android-create-project
                                project-path
                                "com.example.bad"
                                "BadActivity")
                             (error error-data))
                           (nreverse commands)
                           (nreverse opened)))))"##;
    let expect = expect![[
        r#"OK ((error "Error: target missing") ("/sdk/tools/android create project --path \"[ORACLE-SANDBOX]/created project\" --package com.example.demo --activity MainActivity --target \"android-34\" --gradle --gradle-version 8.2.0" "/sdk/tools/android create project --path \"[ORACLE-SANDBOX]/created project\" --package com.example.legacy --activity LegacyActivity --target \"android-34\"" "/sdk/tools/android create project --path \"[ORACLE-SANDBOX]/created project\" --package com.example.bad --activity BadActivity --target \"android-34\" --gradle --gradle-version 2.1.3") ("[ORACLE-SANDBOX]/created project" "[ORACLE-SANDBOX]/created project"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}
