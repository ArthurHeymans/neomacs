use expect_test::expect;

use super::assert_android_mode_parity;

#[test]
fn ant_maven_and_gradle_builders_compile_exact_commands_from_project_root() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (root
                            (file-name-as-directory
                             (expand-file-name
                              "build-project"
                              sandbox)))
                           events)
                      (make-directory root t)
                      (cl-letf
                          (((symbol-function 'android-root)
                            (lambda () root))
                           ((symbol-function 'compile)
                            (lambda (command)
                              (push
                               (list
                                default-directory
                                command)
                               events)
                              command)))
                        (let ((android-mode-build-command-alist
                               '((ant . "ant -e")
                                 (maven . "mvn -q")
                                 (gradle . "./gradlew"))))
                          (list
                           (android-ant
                            "clean debug")
                           (android-maven
                            "android:deploy")
                           (android-gradle
                            "assembleDebug --stacktrace")
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("ant -e clean debug" "mvn -q android:deploy" "./gradlew assembleDebug --stacktrace" (("[ORACLE-SANDBOX]/build-project/" "ant -e clean debug") ("[ORACLE-SANDBOX]/build-project/" "mvn -q android:deploy") ("[ORACLE-SANDBOX]/build-project/" "./gradlew assembleDebug --stacktrace")))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn every_generated_task_wrapper_delegates_the_exact_ant_maven_or_gradle_goal() {
    let elisp_form = r##"(let (events)
                      (cl-letf
                          (((symbol-function 'android-ant)
                            (lambda (task)
                              (push
                               (list 'ant task)
                               events)
                              task))
                           ((symbol-function
                             'android-maven)
                            (lambda (task)
                              (push
                               (list 'maven task)
                               events)
                              task))
                           ((symbol-function
                             'android-gradle)
                            (lambda (task)
                              (push
                               (list 'gradle task)
                               events)
                              task)))
                        (list
                         (mapcar
                          #'funcall
                          '(android-ant-clean
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
                            android-gradle-uninstallDebug))
                         (nreverse events))))"##;
    let expect = expect![[
        r#"OK (("clean" "test" "debug" "installd" "uninstall" "clean" "test" "install" "android:deploy" "android:redeploy" "android:undeploy" "clean" "test" "assembleDebug" "assembleRelease" "installDebug" "uninstallDebug") ((ant "clean") (ant "test") (ant "debug") (ant "installd") (ant "uninstall") (maven "clean") (maven "test") (maven "install") (maven "android:deploy") (maven "android:redeploy") (maven "android:undeploy") (gradle "clean") (gradle "test") (gradle "assembleDebug") (gradle "assembleRelease") (gradle "installDebug") (gradle "uninstallDebug")))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn common_build_commands_dispatch_all_builder_matrices_and_reinstall_rejects_non_maven() {
    let elisp_form = r##"(let (events)
                      (cl-labels
                          ((record
                             (symbol)
                             (push symbol events)
                             symbol))
                        (cl-letf
                            (((symbol-function
                               'android-ant-clean)
                              (lambda ()
                                (record 'ant-clean)))
                             ((symbol-function
                               'android-ant-test)
                              (lambda ()
                                (record 'ant-test)))
                             ((symbol-function
                               'android-ant-debug)
                              (lambda ()
                                (record 'ant-debug)))
                             ((symbol-function
                               'android-ant-installd)
                              (lambda ()
                                (record 'ant-install)))
                             ((symbol-function
                               'android-ant-uninstall)
                              (lambda ()
                                (record 'ant-uninstall)))
                             ((symbol-function
                               'android-gradle-clean)
                              (lambda ()
                                (record 'gradle-clean)))
                             ((symbol-function
                               'android-gradle-test)
                              (lambda ()
                                (record 'gradle-test)))
                             ((symbol-function
                               'android-gradle-assembleDebug)
                              (lambda ()
                                (record 'gradle-debug)))
                             ((symbol-function
                               'android-gradle-installDebug)
                              (lambda ()
                                (record 'gradle-install)))
                             ((symbol-function
                               'android-gradle-uninstallDebug)
                              (lambda ()
                                (record
                                 'gradle-uninstall)))
                             ((symbol-function
                               'android-maven-clean)
                              (lambda ()
                                (record 'maven-clean)))
                             ((symbol-function
                               'android-maven-test)
                              (lambda ()
                                (record 'maven-test)))
                             ((symbol-function
                               'android-maven-install)
                              (lambda ()
                                (record 'maven-debug)))
                             ((symbol-function
                               'android-maven-android-deploy)
                              (lambda ()
                                (record 'maven-install)))
                             ((symbol-function
                               'android-maven-android-undeploy)
                              (lambda ()
                                (record
                                 'maven-uninstall))))
                          (let (results)
                            (dolist
                                (builder
                                 '(ant gradle maven))
                              (let ((android-mode-builder
                                     builder))
                                (push
                                 (list
                                  builder
                                  (android-build-clean)
                                  (android-build-test)
                                  (android-build-debug)
                                  (android-build-install)
                                  (android-build-uninstall))
                                 results)))
                            (let ((android-mode-builder
                                   'maven))
                              (push
                               (list
                                'maven-reinstall
                                (android-build-reinstall))
                               results))
                            (let ((android-mode-builder
                                   'ant))
                              (push
                               (list
                                'ant-reinstall
                                (condition-case
                                    error-data
                                    (android-build-reinstall)
                                  (error error-data)))
                               results))
                            (list
                             (nreverse results)
                             (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (((ant ant-clean ant-test ant-debug ant-install ant-uninstall) (gradle gradle-clean gradle-test gradle-debug gradle-install gradle-uninstall) (maven maven-clean maven-test maven-debug maven-install maven-uninstall) (maven-reinstall maven-install) (ant-reinstall (error "ant builder does not support reinstall"))) (ant-clean ant-test ant-debug ant-install ant-uninstall gradle-clean gradle-test gradle-debug gradle-install gradle-uninstall maven-clean maven-test maven-debug maven-install maven-uninstall maven-install))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}
