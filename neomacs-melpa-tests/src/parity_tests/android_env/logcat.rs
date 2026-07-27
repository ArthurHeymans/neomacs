use expect_test::expect;

use super::assert_android_env_parity;

#[test]
fn logcat_buffer_replaces_stale_output_stops_the_old_process_and_starts_a_read_only_view() {
    let elisp_form = r##"(save-window-excursion
  (let ((buffer (get-buffer-create "*Android Logcat*"))
        (process-lookups 0)
        events)
    (unwind-protect
        (progn
          (with-current-buffer buffer
            (insert "stale logcat output\n"))
          (cl-letf (((symbol-function 'get-buffer-process)
                     (lambda (queried-buffer)
                       (setq process-lookups (1+ process-lookups))
                       (push (list 'get-process
                                   (buffer-name queried-buffer))
                             events)
                       (and (= process-lookups 1) 'old-logcat)))
                    ((symbol-function 'delete-process)
                     (lambda (process)
                       (push (list 'delete process) events)))
                    ((symbol-function 'android-env-adb)
                     (lambda () "/sdk/platform-tools/adb"))
                    ((symbol-function 'start-process)
                     (lambda (&rest arguments)
                       (push (cons 'start arguments) events)
                       'new-logcat)))
            (let ((return-value
                   (android-env-logcat-buffer
                    '("--pid" "321" "*:S" "Checkout"))))
              (list
               return-value
               process-lookups
               (nreverse events)
               (with-current-buffer buffer
                 (list
                  (buffer-string)
                  view-mode
                  buffer-read-only
                  face-remapping-alist))))))
      (when (buffer-live-p buffer)
        (with-current-buffer buffer
          (setq buffer-read-only nil))
        (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (t 2 ((get-process "*Android Logcat*") (delete old-logcat) (get-process "*Android Logcat*") (start "Android Logcat" "*Android Logcat*" "/sdk/platform-tools/adb" "logcat" "--pid" "321" "*:S" "Checkout")) ("" t t ((default (:height 105) default))))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn logcat_tag_filter_distinguishes_nil_empty_and_named_tags_and_orders_suppression_first() {
    let elisp_form = r##"(let (calls)
  (cl-letf (((symbol-function 'android-env-logcat-buffer)
             (lambda (arguments)
               (push (copy-tree arguments) calls)
               arguments)))
    (list
     (android-env-logcat nil)
     (android-env-logcat "")
     (android-env-logcat "Checkout Flow")
     (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (nil nil ("*:S" "Checkout Flow") (nil nil ("*:S" "Checkout Flow")))"#]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn crash_logcat_wrapper_selects_only_the_android_crash_buffer() {
    let elisp_form = r##"(let (calls)
  (cl-letf (((symbol-function 'android-env-logcat-buffer)
             (lambda (arguments)
               (push arguments calls)
               'shown)))
    (list
     (android-env-logcat-crash)
     (nreverse calls))))"##;
    let expect = expect![[r#"OK (shown (("-b" "crash")))"#]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn pid_assoc_helper_preserves_its_actual_d_space_split_semantics_on_process_text() {
    let elisp_form = r##"(mapcar
 (lambda (text)
   (list text (android-env-logcat-pid-assoc text)))
 '("123 com.example.Main"
   "123 daemon-worker"
   "42 android system"
   "7d child"
   "no delimiter"))"##;
    let expect = expect![[
        r#"OK (("123 com.example.Main" ("123 com.example.Main")) ("123 daemon-worker" ("123 daemon-worker")) ("42 android system" ("42 android system")) ("7d child" ("7d child")) ("no delimiter" ("no delimiter")))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn pid_logcat_builds_a_process_map_prompts_by_command_name_and_forwards_the_selected_pid() {
    let elisp_form = r##"(let (events)
  (cl-letf (((symbol-function 'android-env-adb)
             (lambda () "/sdk/platform-tools/adb"))
            ((symbol-function 'shell-command-to-string)
             (lambda (command)
               (push (list 'shell command) events)
               "101 com.alpha.Worker\n205 com.beta.Main\n333 com.gamma.Sync\n"))
            ((symbol-function 'completing-read)
             (lambda (prompt collection &rest arguments)
               (push (list 'complete
                           prompt
                           (copy-sequence collection)
                           arguments)
                     events)
               "com.beta.Main"))
            ((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (let ((text (apply #'format format-string arguments)))
                 (push (list 'message text) events)
                 text)))
            ((symbol-function 'android-env-logcat-buffer)
             (lambda (arguments)
               (push (list 'logcat arguments) events)
               'shown)))
    (list
     (android-env-logcat-pid)
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK (shown ((shell "/sdk/platform-tools/adb shell ps -A -o PID,ARGS=CMD") (complete "Select process: " ("com.gamma.Sync" "com.beta.Main" "com.alpha.Worker") nil) (message "pid: 205") (logcat ("--pid" "205"))))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}
