use expect_test::expect;

use super::assert_android_env_parity;

#[test]
fn adb_path_uses_the_configured_android_sdk_root_without_normalizing_spaces() {
    let elisp_form = r##"(let ((process-environment
       (cons "ANDROID_SDK_ROOT=/opt/Android SDK"
             process-environment)))
  (android-env-adb))"##;
    let expect = expect![[r#"OK "/opt/Android SDK/platform-tools/adb""#]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn avd_list_invokes_the_sdk_manager_and_removes_header_and_empty_nul_records() {
    let elisp_form = r##"(let ((process-environment
       (cons "ANDROID_SDK_ROOT=/opt/android-sdk"
             process-environment))
      calls)
  (cl-letf (((symbol-function 'shell-command-to-string)
             (lambda (command)
               (push command calls)
               "Available Android Virtual Devices:\0Pixel_8_API_35\0Foldable_API_34\0")))
    (list (android-env-avd-list)
          (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("Pixel_8_API_35" "Foldable_API_34") ("/opt/android-sdk/tools/bin/avdmanager list avd --compact -0"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn avd_command_prompts_from_the_live_device_list_and_launches_the_selected_emulator_buffer() {
    let elisp_form = r##"(let ((android-env-emulator-command
       "/opt/Android SDK/emulator/emulator")
      events)
  (cl-letf (((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (let ((text (apply #'format format-string arguments)))
                 (push (list 'message text) events)
                 text)))
            ((symbol-function 'android-env-avd-list)
             (lambda () '("Pixel 8 API 35" "Foldable_API_34")))
            ((symbol-function 'completing-read)
             (lambda (prompt collection &rest arguments)
               (push (list 'complete prompt collection arguments) events)
               "Pixel 8 API 35"))
            ((symbol-function 'async-shell-command)
             (lambda (command buffer)
               (push (list 'async command buffer) events)
               'emulator-process)))
    (list (android-env-avd)
          (nreverse events))))"##;
    let expect = expect![[
        r#"OK (emulator-process ((message "Getting avd list...") (complete "Select avd: " ("Pixel 8 API 35" "Foldable_API_34") nil) (async "/opt/Android SDK/emulator/emulator @Pixel\\ 8\\ API\\ 35" "*android-emulator-Pixel 8 API 35")))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn auto_desktop_head_unit_forwards_adb_then_head_unit_as_one_async_shell_pipeline() {
    let elisp_form = r##"(let (calls)
  (cl-letf (((symbol-function 'async-shell-command)
             (lambda (command buffer)
               (push (list command buffer) calls)
               'dhu-process)))
    (list
     (android-env-auto-dhu)
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (dhu-process (("$ANDROID_SDK_ROOT/platform-tools/adb forward tcp:5277 tcp:5277 && $ANDROID_SDK_ROOT/extras/google/auto/desktop-head-unit" "*android-auto-dhu*")))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn logcat_clear_uses_the_resolved_adb_binary_and_clear_subcommand_synchronously() {
    let elisp_form = r##"(let (calls)
  (cl-letf (((symbol-function 'android-env-adb)
             (lambda () "/opt/android/platform-tools/adb"))
            ((symbol-function 'shell-command)
             (lambda (&rest arguments)
               (push arguments calls)
               0)))
    (list
     (android-env-logcat-clear)
     (nreverse calls))))"##;
    let expect = expect![[r#"OK (0 (("/opt/android/platform-tools/adb logcat -c")))"#]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn uninstall_app_targets_the_configured_output_buffer_and_preserves_android_package_name() {
    let elisp_form = r##"(let ((android-env-adb-buffer-name
       "*project adb output*")
      calls)
  (cl-letf (((symbol-function 'android-env-adb)
             (lambda () "/sdk/platform-tools/adb"))
            ((symbol-function 'shell-command)
             (lambda (&rest arguments)
               (push arguments calls)
               0)))
    (list
     (android-env-uninstall-app "com.example.checkout.debug")
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (0 (("/sdk/platform-tools/adb shell pm uninstall 'com.example.checkout.debug'" "*project adb output*")))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn deeplink_quotes_the_entire_android_activity_request_and_uses_the_adb_output_buffer() {
    let elisp_form = r##"(let ((android-env-adb-buffer-name
       "*deeplink adb*")
      calls)
  (cl-letf (((symbol-function 'android-env-adb)
             (lambda () "/opt/Android SDK/platform-tools/adb"))
            ((symbol-function 'shell-command)
             (lambda (&rest arguments)
               (push arguments calls)
               0)))
    (list
     (android-env-deeplink
      "myapp://checkout/open?name=Jane Doe&campaign=\"summer sale\"")
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (0 (("/opt/Android\\ SDK/platform-tools/adb shell am\\ start\\ -a\\ android.intent.action.VIEW\\ -d\\ \\\"myapp\\://checkout/open\\?name\\=Jane\\ Doe\\&campaign\\=\\\"summer\\ sale\\\"\\\"" "*deeplink adb*")))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}
