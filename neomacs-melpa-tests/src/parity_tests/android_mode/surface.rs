use expect_test::expect;

use super::assert_android_mode_parity;

#[test]
fn package_defaults_custom_metadata_faces_and_log_levels_match() {
    let elisp_form = r##"(list
                      (featurep 'android-mode)
                      android-mode-default-builders
                      (list
                       android-mode-sdk-dir
                       android-mode-sdk-tool-subdirs
                       android-mode-sdk-tool-extensions
                       android-mode-builder
                       android-mode-root-file-plist
                       android-mode-build-command-alist
                       android-mode-key-prefix
                       android-mode-avd
                       android-mode-gradle-plugin
                       android-logcat-buffer)
                      (mapcar
                       (lambda (symbol)
                         (list
                          symbol
                          (get symbol 'custom-type)
                          (get symbol 'custom-group)))
                       '(android-mode-sdk-dir
                         android-mode-sdk-tool-subdirs
                         android-mode-sdk-tool-extensions
                         android-mode-builder
                         android-mode-root-file-plist
                         android-mode-build-command-alist
                         android-mode-key-prefix
                         android-mode-avd
                         android-mode-gradle-plugin
                         android-logcat-buffer))
                      android-mode-log-face-alist
                      (mapcar
                       (lambda (face)
                         (list
                          face
                          (get face 'face-defface-spec)
                          (get face 'face-documentation)))
                       '(android-mode-verbose-face
                         android-mode-debug-face
                         android-mode-info-face
                         android-mode-warning-face
                         android-mode-error-face))
                      android-mode-log-filter-regexp
                      android-logcat-pending-output
                      android-exclusive-processes)"##;
    let expect = expect![[
        r#"OK (t (ant gradle maven) (nil ("emulator" "tools" "platform-tools") ("" ".bat" ".exe") gradle (ant "AndroidManifest.xml" maven "AndroidManifest.xml" gradle "gradlew") ((ant . "ant -e") (maven . "mvn") (gradle . "./gradlew")) "\3 a" "" "2.1.3" "*android-logcat*") ((android-mode-sdk-dir string nil) (android-mode-sdk-tool-subdirs (repeat string) nil) (android-mode-sdk-tool-extensions (repeat string) nil) (android-mode-builder symbol nil) (android-mode-root-file-plist (plist :key-type symbol :value-type string) nil) (android-mode-build-command-alist (alist :key-type symbol :value-type string) nil) (android-mode-key-prefix string nil) (android-mode-avd string nil) (android-mode-gradle-plugin string nil) (android-logcat-buffer string nil)) (("V" . android-mode-verbose-face) ("D" . android-mode-debug-face) ("I" . android-mode-info-face) ("W" . android-mode-warning-face) ("E" . android-mode-error-face)) ((android-mode-verbose-face ((t (:foreground "DodgerBlue"))) "Font Lock face used to highlight VERBOSE log records.") (android-mode-debug-face ((t (:foreground "ForestGreen"))) "Font Lock face used to highlight DEBUG log records.") (android-mode-info-face ((t (:foreground "Gray45"))) "Font Lock face used to highlight INFO log records.") (android-mode-warning-face ((t (:foreground "Red"))) "Font Lock face used to highlight WARN log records.") (android-mode-error-face ((t (:foreground "Red" :bold t))) "Font Lock face used to highlight ERROR log records.")) "" "" nil)"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn complete_shipped_and_generated_callable_surface_has_exact_arglists_and_command_status() {
    let elisp_form = r##"(mapcar
                      (lambda (symbol)
                        (list
                         symbol
                         (fboundp symbol)
                         (help-function-arglist symbol t)
                         (macrop symbol)
                         (commandp symbol)))
                      '(android-find-dir
                        android-root
                        android-manifest-dir
                        android-in-directory
                        android-local-sdk-dir
                        android-tool-path
                        android-start-exclusive-command
                        android-create-project
                        android-list-targets
                        android-list-avd
                        android-start-emulator
                        android-start-ddms
                        android-logcat-find-file
                        android-logcat-find-file-mouse
                        android-logcat-prepare-msg
                        android-logcat-process-filter
                        android-logcat
                        android-current-buffer-class-name
                        android-project-package
                        android-project-main-activities
                        android-start-app
                        android-logcat-set-filter
                        android-logcat-clear-filter
                        android-logcat-erase-buffer
                        android-defun-builder
                        android-defun-ant-task
                        android-defun-maven-task
                        android-defun-gradle-task
                        android-ant
                        android-maven
                        android-gradle
                        android-ant-clean
                        android-ant-test
                        android-ant-debug
                        android-ant-installd
                        android-ant-uninstall
                        android-maven-clean
                        android-maven-test
                        android-maven-install
                        android-maven-android-deploy
                        android-maven-android-redeploy
                        android-maven-android-undeploy
                        android-gradle-clean
                        android-gradle-test
                        android-gradle-assembleDebug
                        android-gradle-assembleRelease
                        android-gradle-installDebug
                        android-gradle-uninstallDebug
                        android-build-clean
                        android-build-test
                        android-build-debug
                        android-build-install
                        android-build-reinstall
                        android-build-uninstall
                        android-mode))"##;
    let expect = expect![
        "OK ((android-find-dir t (filename) nil nil) (android-root t nil nil nil) (android-manifest-dir t nil nil nil) (android-in-directory t (chosen-dir body) t nil) (android-local-sdk-dir t nil nil nil) (android-tool-path t (name) nil nil) (android-start-exclusive-command t (name command &rest args) nil nil) (android-create-project t (path package activity) nil t) (android-list-targets t nil nil nil) (android-list-avd t nil nil nil) (android-start-emulator t nil nil t) (android-start-ddms t nil nil t) (android-logcat-find-file t nil nil t) (android-logcat-find-file-mouse t (event) nil t) (android-logcat-prepare-msg t (msg) nil nil) (android-logcat-process-filter t (process output) nil nil) (android-logcat t nil nil t) (android-current-buffer-class-name t nil nil nil) (android-project-package t nil nil nil) (android-project-main-activities t (&optional category) nil nil) (android-start-app t nil nil t) (android-logcat-set-filter t (regexp-filter) nil t) (android-logcat-clear-filter t nil nil t) (android-logcat-erase-buffer t nil nil t) (android-defun-builder t (builder) t nil) (android-defun-ant-task t (task) t nil) (android-defun-maven-task t (task) t nil) (android-defun-gradle-task t (task) t nil) (android-ant t #1=(tasks-or-goals) nil t) (android-maven t #1# nil t) (android-gradle t #1# nil t) (android-ant-clean t nil nil t) (android-ant-test t nil nil t) (android-ant-debug t nil nil t) (android-ant-installd t nil nil t) (android-ant-uninstall t nil nil t) (android-maven-clean t nil nil t) (android-maven-test t nil nil t) (android-maven-install t nil nil t) (android-maven-android-deploy t nil nil t) (android-maven-android-redeploy t nil nil t) (android-maven-android-undeploy t nil nil t) (android-gradle-clean t nil nil t) (android-gradle-test t nil nil t) (android-gradle-assembleDebug t nil nil t) (android-gradle-assembleRelease t nil nil t) (android-gradle-installDebug t nil nil t) (android-gradle-uninstallDebug t nil nil t) (android-build-clean t nil nil t) (android-build-test t nil nil t) (android-build-debug t nil nil t) (android-build-install t nil nil t) (android-build-reinstall t nil nil t) (android-build-uninstall t nil nil t) (android-mode t (&optional arg) nil t))"
    ];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn enabling_minor_mode_materializes_complete_prefixed_map_and_logcat_navigation_map() {
    let elisp_form = r##"(with-temp-buffer
                      (let ((before
                             (mapcar
                              (lambda (key)
                                (lookup-key
                                 android-mode-map
                                 (kbd
                                  (concat
                                   android-mode-key-prefix
                                   " " key))))
                              (mapcar #'car
                                      android-mode-keys))))
                        (android-mode 1)
                        (list
                         before
                         android-mode
                         (assq
                          'android-mode
                          minor-mode-alist)
                         (mapcar
                          (lambda (spec)
                            (cons
                             (car spec)
                             (lookup-key
                              android-mode-map
                              (kbd
                               (concat
                                android-mode-key-prefix
                                " "
                                (car spec))))))
                          android-mode-keys)
                         (mapcar
                          (lambda (key)
                            (cons
                             key
                             (lookup-key
                              android-logcat-map
                              (kbd key))))
                          '("RET"
                            "<mouse-2>"
                            "n"
                            "p"
                            "q"
                            "f"
                            "c"
                            "C"))
                         (local-variable-if-set-p
                          'android-mode)
                         (commandp 'android-mode)
                         (list
                          (seq-count
                           (lambda (function)
                             (string-match-p
                              "android-root"
                              (prin1-to-string
                               function)))
                           find-file-hook)
                          (seq-count
                           (lambda (function)
                             (string-match-p
                              "android-root"
                              (prin1-to-string
                               function)))
                           dired-mode-hook)))))"##;
    let expect = expect![[
        r#"OK ((1 1 1 1 1 1 1 1 1 1) t (android-mode " Android") (("d" . android-start-ddms) ("e" . android-start-emulator) ("l" . android-logcat) ("C" . android-build-clean) ("t" . android-build-test) ("c" . android-build-debug) ("i" . android-build-install) ("r" . android-build-reinstall) ("u" . android-build-uninstall) ("a" . android-start-app)) (("RET" . android-logcat-find-file) ("<mouse-2>" . android-logcat-find-file-mouse) ("n" . next-logical-line) ("p" . previous-logical-line) ("q" . delete-window) ("f" . android-logcat-set-filter) ("c" . android-logcat-clear-filter) ("C" . android-logcat-erase-buffer)) t t (1 1))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}
