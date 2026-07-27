use expect_test::expect;

use super::assert_android_mode_parity;

#[test]
fn project_root_and_manifest_discovery_walk_real_nested_gradle_and_ant_trees() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (root
                            (expand-file-name
                             "android-project"
                             sandbox))
                           (nested
                            (expand-file-name
                             "app/src/main/java/pkg"
                             root))
                           messages)
                      (make-directory nested t)
                      (with-temp-file
                          (expand-file-name
                           "gradlew" root)
                        (insert "#!/bin/sh\n"))
                      (with-temp-file
                          (expand-file-name
                           "AndroidManifest.xml"
                           root)
                        (insert "<manifest/>"))
                      (let ((default-directory
                             (file-name-as-directory
                              nested)))
                        (cl-letf
                            (((symbol-function 'message)
                              (lambda
                                  (format-string
                                   &rest arguments)
                                (push
                                 (apply
                                  #'format
                                  format-string
                                  arguments)
                                 messages)
                                nil)))
                          (let ((android-mode-builder
                                 'gradle))
                            (list
                             (android-root)
                             (android-manifest-dir)
                             (android-find-dir
                              "missing.file")
                             (let ((android-mode-builder
                                    'ant))
                               (android-root))
                             (let ((android-mode-builder
                                    'unknown))
                               (android-root))
                             (nreverse messages))))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/android-project/" "[ORACLE-SANDBOX]/android-project/" nil "[ORACLE-SANDBOX]/android-project/" nil ("unknown was not found in `android-mode-root-file-plist'"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn directory_macro_evaluates_root_once_executes_body_there_and_signals_when_missing() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (chosen
                            (file-name-as-directory
                             (expand-file-name
                              "chosen-project"
                              sandbox)))
                           evaluations)
                      (make-directory chosen t)
                      (list
                       (android-in-directory
                        (progn
                          (push 'evaluated evaluations)
                          chosen)
                        (list
                         default-directory
                         (file-name-nondirectory
                          (directory-file-name
                           default-directory))))
                       (nreverse evaluations)
                       (condition-case error-data
                           (android-in-directory
                            nil
                            'not-run)
                         (error error-data))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-SANDBOX]/chosen-project/" "chosen-project") (evaluated evaluated) (error "Can’t find project root"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn java_class_detection_handles_packages_package_less_classes_case_and_non_java_buffers() {
    let elisp_form = r##"(mapcar
                      (lambda (scenario)
                        (with-temp-buffer
                          (setq buffer-file-name
                                (car scenario))
                          (insert (cdr scenario))
                          (goto-char (point-max))
                          (list
                           (android-current-buffer-class-name)
                           (point))))
                      '(("src/MainActivity.java"
                         . "package com.example.app;\npublic class MainActivity {\n}\n")
                        ("src/Standalone.java"
                         . "public class Standalone {}\n")
                        ("src/lower.java"
                         . "package COM.Example;\npublic class lower {}\n")
                        ("src/Interface.java"
                         . "package com.example;\npublic interface Interface {}\n")
                        ("src/MainActivity.kt"
                         . "package com.example\nclass MainActivity\n")))"##;
    let expect = expect![[
        r#"OK (("com.example.app.MainActivity" 56) ("Standalone" 28) ("lower" 44) (nil 52) (nil 40))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn installed_find_file_and_dired_hooks_enable_mode_only_when_project_root_exists() {
    let elisp_form = r##"(let ((find-hook
                           (car find-file-hook))
                          (dired-hook
                           (car dired-mode-hook))
                          roots
                          events)
                      (cl-letf
                          (((symbol-function 'android-root)
                            (lambda ()
                              (pop roots)))
                           ((symbol-function 'android-mode)
                            (lambda (argument)
                              (push argument events)
                              argument)))
                        (setq roots
                              '("project/" nil
                                "dired-project/" nil))
                        (list
                         (funcall find-hook)
                         (funcall find-hook)
                         (funcall dired-hook)
                         (funcall dired-hook)
                         (nreverse events)
                         (string-match-p
                          "android-root"
                          (prin1-to-string
                           find-hook))
                         (string-match-p
                          "android-root"
                          (prin1-to-string
                           dired-hook)))))"##;
    let expect = expect!["OK (t nil t nil (t t) 12 12)"];
    assert_android_mode_parity(elisp_form, expect);
}
