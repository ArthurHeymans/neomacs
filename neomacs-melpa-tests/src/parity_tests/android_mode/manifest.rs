use expect_test::expect;

use super::assert_android_mode_parity;

#[test]
fn manifest_package_and_main_activity_resolution_expand_relative_capitalized_and_qualified_names() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (root
                            (file-name-as-directory
                             (expand-file-name
                              "manifest-project"
                              sandbox)))
                           (manifest
                            (expand-file-name
                             "AndroidManifest.xml"
                             root)))
                      (make-directory root t)
                      (with-temp-file manifest
                        (insert
                         "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\" package=\"com.example.demo\">\n\
  <application>\n\
    <activity android:name=\".MainActivity\">\n\
      <intent-filter>\n\
        <action android:name=\"android.intent.action.MAIN\" />\n\
        <category android:name=\"android.intent.category.LAUNCHER\" />\n\
      </intent-filter>\n\
    </activity>\n\
    <activity android:name=\"SettingsActivity\">\n\
      <intent-filter>\n\
        <action android:name=\"android.intent.action.MAIN\" />\n\
        <category android:name=\"android.intent.category.DEFAULT\" />\n\
      </intent-filter>\n\
    </activity>\n\
    <activity android:name=\"com.vendor.ExternalActivity\">\n\
      <intent-filter>\n\
        <action android:name=\"android.intent.action.MAIN\" />\n\
        <category android:name=\"android.intent.category.LAUNCHER\" />\n\
      </intent-filter>\n\
    </activity>\n\
    <activity android:name=\".NotMain\">\n\
      <intent-filter>\n\
        <action android:name=\"android.intent.action.VIEW\" />\n\
        <category android:name=\"android.intent.category.DEFAULT\" />\n\
      </intent-filter>\n\
    </activity>\n\
  </application>\n\
</manifest>\n"))
                      (cl-letf
                          (((symbol-function
                             'android-manifest-dir)
                            (lambda () root)))
                        (list
                         (android-project-package)
                         (android-project-main-activities)
                         (android-project-main-activities
                          "LAUNCHER")
                         (android-project-main-activities
                          "DEFAULT")
                         (android-project-main-activities
                          "LEANBACK_LAUNCHER"))))"##;
    let expect = expect![[
        r#"OK ("com.example.demo" ("com.example.demo.MainActivity" "com.example.demo.SettingsActivity" "com.vendor.ExternalActivity" "com.example.demo.NotMain") ("com.example.demo.MainActivity" "com.example.demo.SettingsActivity" "com.vendor.ExternalActivity" "com.example.demo.NotMain") ("com.example.demo.SettingsActivity" "com.vendor.ExternalActivity" "com.example.demo.NotMain") nil)"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn start_app_prefers_current_main_then_launcher_fallback_and_preserves_adb_errors() {
    let elisp_form = r##"(let ((current
                           "com.example.demo.MainActivity")
                          response
                          commands
                          messages)
                      (cl-letf
                          (((symbol-function
                             'android-project-package)
                            (lambda ()
                              "com.example.demo"))
                           ((symbol-function
                             'android-current-buffer-class-name)
                            (lambda () current))
                           ((symbol-function
                             'android-project-main-activities)
                            (lambda (&optional category)
                              (if category
                                  '("com.example.demo.Launcher")
                                '("com.example.demo.MainActivity"
                                  "com.example.demo.Launcher"))))
                           ((symbol-function
                             'android-tool-path)
                            (lambda (_name)
                              "/sdk/platform-tools/adb"))
                           ((symbol-function
                             'shell-command-to-string)
                            (lambda (command)
                              (push command commands)
                              response))
                           ((symbol-function 'message)
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
                        (setq response
                              "Starting: Intent")
                        (android-start-app)
                        (setq current
                              "com.example.demo.Other")
                        (android-start-app)
                        (setq response
                              "Error: Activity not found")
                        (list
                         (condition-case error-data
                             (android-start-app)
                           (error error-data))
                         (nreverse commands)
                         (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ((error "/sdk/platform-tools/adb shell am start -n com.example.demo/com.example.demo.Launcher\nError: Activity not found") ("/sdk/platform-tools/adb shell am start -n com.example.demo/com.example.demo.MainActivity" "/sdk/platform-tools/adb shell am start -n com.example.demo/com.example.demo.Launcher" "/sdk/platform-tools/adb shell am start -n com.example.demo/com.example.demo.Launcher") ("Starting activity: com.example.demo.MainActivity" "Starting activity: com.example.demo.Launcher" "Starting activity: com.example.demo.Launcher"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn start_app_signals_missing_activity_after_resolving_package_and_tool() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function
                             'android-project-package)
                            (lambda ()
                              (push 'package calls)
                              "com.example.empty"))
                           ((symbol-function
                             'android-current-buffer-class-name)
                            (lambda ()
                              (push 'current calls)
                              nil))
                           ((symbol-function
                             'android-project-main-activities)
                            (lambda (&optional category)
                              (push
                               (list 'activities category)
                               calls)
                              nil))
                           ((symbol-function
                             'android-tool-path)
                            (lambda (name)
                              (push
                               (list 'tool name)
                               calls)
                              "/sdk/adb")))
                        (list
                         (condition-case error-data
                             (android-start-app)
                           (error error-data))
                         (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((error "No main activity found in manifest") (package current (activities nil) (activities "LAUNCHER") (tool "adb")))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn logcat_source_link_opens_real_project_file_and_moves_to_exact_line_property() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (root
                            (file-name-as-directory
                             (expand-file-name
                              "linked-project"
                              sandbox)))
                           (relative
                            "com/example/MainActivity.java")
                           (source
                            (expand-file-name
                             (concat "src/" relative)
                             root))
                           (log-buffer
                            (generate-new-buffer
                             " *android-link-log*"))
                           source-buffer)
                      (make-directory
                       (file-name-directory source)
                       t)
                      (with-temp-file source
                        (insert
                         "line one\nline two\nline three\nline four\n"))
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'android-root)
                                (lambda () root)))
                            (with-current-buffer log-buffer
                              (insert
                               (propertize
                                "stack frame"
                                'filename relative
                                'linenr 3))
                              (goto-char (point-min))
                              (android-logcat-find-file)
                              (setq source-buffer
                                    (current-buffer))
                              (list
                               buffer-file-name
                               (line-number-at-pos)
                               (buffer-substring-no-properties
                                (line-beginning-position)
                                (line-end-position)))))
                        (when
                            (buffer-live-p log-buffer)
                          (kill-buffer log-buffer))
                        (when
                            (buffer-live-p source-buffer)
                          (kill-buffer source-buffer))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/linked-project/src/com/example/MainActivity.java" 3 "line three")"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn mouse_logcat_navigation_resolves_event_window_position_and_delegates_at_point() {
    let elisp_form = r##"(let ((target
                           (generate-new-buffer
                            " *android-mouse-target*"))
                          calls)
                      (unwind-protect
                          (progn
                            (with-current-buffer target
                              (insert "0123456789"))
                            (cl-letf
                                (((symbol-function 'event-end)
                                  (lambda (event)
                                    (push
                                     (list
                                      'event-end event)
                                     calls)
                                    'fake-position))
                                 ((symbol-function 'posn-window)
                                  (lambda (position)
                                    (push
                                     (list
                                      'window position)
                                     calls)
                                    'fake-window))
                                 ((symbol-function 'posn-point)
                                  (lambda (position)
                                    (push
                                     (list
                                      'point position)
                                     calls)
                                    6))
                                 ((symbol-function 'window-buffer)
                                  (lambda (window)
                                    (push
                                     (list
                                      'buffer window)
                                     calls)
                                    target))
                                 ((symbol-function
                                   'android-logcat-find-file)
                                  (lambda ()
                                    (push
                                     (list
                                      'delegate
                                      (buffer-name)
                                      (point))
                                     calls)
                                    'opened)))
                              (list
                               (android-logcat-find-file-mouse
                                'mouse-event)
                               (nreverse calls))))
                        (kill-buffer target)))"##;
    let expect = expect![[
        r#"OK (opened ((event-end mouse-event) (window fake-position) (event-end mouse-event) (point fake-position) (buffer fake-window) (delegate " *android-mouse-target*" 6)))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}
