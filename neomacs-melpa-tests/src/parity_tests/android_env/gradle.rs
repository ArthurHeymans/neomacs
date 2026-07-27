use expect_test::expect;

use super::{assert_android_env_hydra_parity, assert_android_env_parity};

#[test]
fn entry_command_initializes_hydra_only_when_the_user_option_is_enabled() {
    let elisp_form = r##"(let (events)
  (cl-letf (((symbol-function 'android-env-hydra-setup)
             (lambda () (push 'hydra-setup events) 'configured)))
    (let ((android-env-hydra nil))
      (push (list 'disabled (android-env)) events))
    (let ((android-env-hydra t))
      (push (list 'enabled (android-env)) events))
    (nreverse events)))"##;
    let expect = expect!["OK ((disabled nil) hydra-setup (enabled configured))"];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn optional_hydra_setup_is_a_clean_noop_when_hydra_is_not_installed() {
    let elisp_form = r##"(let (requests)
  (cl-letf (((symbol-function 'require)
             (lambda (&rest arguments)
               (push arguments requests)
               nil)))
    (list
     (android-env-hydra-setup)
     (nreverse requests)
     (fboundp 'hydra-android/body)
     (boundp 'hydra-android/keymap))))"##;
    let expect = expect!["OK (nil ((hydra nil noerror)) nil nil)"];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn hydra_setup_registers_every_android_workflow_head_when_the_optional_macro_is_available() {
    let elisp_form = r##"(let ((result (android-env-hydra-setup)))
  (let* ((definition android-env-test-hydra-definition)
         (body (nth 2 definition))
         (hint (car body))
         (heads (cdr body)))
    (list
     (equal result definition)
     (car definition)
     (cadr definition)
     (secure-hash 'sha256 hint)
     (length hint)
     (mapcar
      (lambda (head)
        (list (nth 0 head)
              (nth 1 head)
              (nth 2 head)))
      heads))))"##;
    let expect = expect![[
        r#"OK (t hydra-android (:color teal :hint nil) "e3242e0811e740e918a8841f27ec76bd359db49933470ecfc8112a069da1176e" 578 (("w" android-env-compile nil) ("s" android-env-test nil) ("u" android-env-unit-test nil) ("e" android-env-avd nil) ("d" android-env-auto-dhu nil) ("l" android-env-logcat nil) ("c" android-env-logcat-crash nil) ("C" android-env-logcat-clear nil) ("p" android-env-logcat-pid nil) ("t" android-env-unit-test-single nil) ("x" android-env-crashlytics nil) ("U" android-env-uninstall-app nil) ("L" android-env-deeplink nil) ("r" android-env-refactor nil) ("R" android-env-recursive-refactor nil) ("q" nil "quit")))"#
    ]];
    assert_android_env_hydra_parity(elisp_form, expect);
}

#[test]
fn crashlytics_builds_the_module_scoped_assemble_and_distribution_tasks() {
    let elisp_form = r##"(let (calls)
  (cl-letf (((symbol-function 'android-env-gradle)
             (lambda (command) (push command calls) 'started)))
    (list
     (android-env-crashlytics ":mobile-app" "QaRelease")
     (android-env-crashlytics "feature:checkout" "Prod")
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (started started (":mobile-app:assembleQaRelease :mobile-app:crashlyticsUploadDistributionQaRelease" "feature:checkout:assembleProd feature:checkout:crashlyticsUploadDistributionProd"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn gradle_reports_a_missing_project_without_starting_compilation() {
    let elisp_form = r##"(let (events)
  (cl-letf (((symbol-function 'locate-dominating-file)
             (lambda (directory marker)
               (push (list 'locate directory marker) events)
               nil))
            ((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (let ((text (apply #'format format-string arguments)))
                 (push (list 'message text) events)
                 text)))
            ((symbol-function 'compilation-start)
             (lambda (&rest arguments)
               (push (cons 'compile arguments) events))))
    (list
     (android-env-gradle "assembleDebug")
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("Couldn't find a gradle project in ancestors directories" ((locate "." "gradlew") (message "Couldn't find a gradle project in ancestors directories")))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn gradle_quotes_a_real_project_root_and_executable_then_selects_android_compile_mode() {
    let elisp_form = r##"(let ((android-env-executable "./gradlew wrapper")
      invocation)
  (cl-letf (((symbol-function 'locate-dominating-file)
             (lambda (_directory _marker)
               "/workspace/Android Project/[demo]/"))
            ((symbol-function 'compilation-start)
             (lambda (command mode)
               (setq invocation (list command mode))
               'compilation-buffer)))
    (list
     (android-env-gradle
      ":mobile:assembleQa --scan --tests com.example.CartTest")
     invocation)))"##;
    let expect = expect![[
        r#"OK (compilation-buffer ("cd /workspace/Android\\ Project/[demo]/; ./gradlew\\ wrapper :mobile:assembleQa --scan --tests com.example.CartTest" android-env-compile-mode))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn configured_test_unit_test_and_compile_commands_forward_exact_gradle_tasks() {
    let elisp_form = r##"(let ((android-env-test-command
       "connectedQaDebugAndroidTest")
      (android-env-unit-test-command
       ":core:testQaDebugUnitTest")
      calls)
  (cl-letf (((symbol-function 'android-env-gradle)
             (lambda (command)
               (push command calls)
               (length calls))))
    (list
     (android-env-test)
     (android-env-unit-test)
     (android-env-compile
      ":app:bundleRelease --configuration-cache")
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (1 2 3 ("connectedQaDebugAndroidTest" ":core:testQaDebugUnitTest" ":app:bundleRelease --configuration-cache"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn single_unit_test_combines_build_variant_and_fully_qualified_jvm_selector() {
    let elisp_form = r##"(let (calls)
  (cl-letf (((symbol-function 'android-env-gradle)
             (lambda (command)
               (push command calls)
               'queued)))
    (list
     (android-env-unit-test-single
      "QaDebugUnitTest"
      "com.example.checkout.CartRepositoryTest.loads cached cart")
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (queued ("testQaDebugUnitTest --tests com.example.checkout.CartRepositoryTest.loads cached cart"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}
