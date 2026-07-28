use expect_test::expect;

use super::assert_android_env_parity;

#[test]
fn compiling_runs_gradlew_from_the_project_root_it_finds_above_the_open_file() {
    let elisp_form = r##"
(progn
  (aenv-test-write
   "checkout/gradlew"
   "#!/bin/sh\necho \"argv: $*\"\necho \"cwd: $(pwd)\"\necho BUILD SUCCESSFUL\n"
   t)
  (aenv-test-write "checkout/app/src/main/java/com/example/Main.java"
                   "package com.example;\nclass Main {}\n")
  (aenv-test-write "loose/Notes.java" "class Notes {}\n")
  (find-file (aenv-test-path "checkout/app/src/main/java/com/example/Main.java"))
  (let ((setup (list :compilation-mode (and (fboundp 'android-env-compile-mode) t)
                     :registered-regexps
                     (mapcar (lambda (name) (assq name
                                                  compilation-error-regexp-alist-alist))
                             '(android-java android-java-2 android-kotlin))
                     :hydra-option android-env-hydra
                     :entry-command-with-hydra-off (android-env)
                     :project-root (locate-dominating-file "." "gradlew"))))
    (android-env-compile "assembleDevDebug")
    (let ((first (list :wait (aenv-test-await "*android-env-compile*")
                       :buffer (aenv-test-compilation-text
                                "*android-env-compile*")
                       :mode (with-current-buffer "*android-env-compile*"
                               major-mode)
                       :error-regexps
                       (with-current-buffer "*android-env-compile*"
                         compilation-error-regexp-alist))))
      ;; The two task-specific commands go through the same route with the
      ;; configured gradle tasks.
      (android-env-unit-test)
      (aenv-test-await "*android-env-compile*")
      (let ((unit (aenv-test-compilation-text "*android-env-compile*")))
        (android-env-test)
        (aenv-test-await "*android-env-compile*")
        (let ((instrumented (aenv-test-compilation-text
                             "*android-env-compile*")))
          ;; A file with no gradlew anywhere above it is reported, not run.
          (find-file (aenv-test-path "loose/Notes.java"))
          (list :setup setup
                :compile first
                :unit-test-command android-env-unit-test-command
                :unit-test unit
                :instrumented-test-command android-env-test-command
                :instrumented-test instrumented
                :outside-a-project
                (list :root (locate-dominating-file "." "gradlew")
                      :result (android-env-gradle "assembleRelease"))))))))
"##;

    let expect = expect![[
        r#"OK (:setup (:compilation-mode t :registered-regexps ((android-java ":compile.*?\\(/.*?\\):\\([0-9]+\\): " 1 2) (android-java-2 "^\\(/.[^:]*\\):\\([0-9]*\\): +error:" 1 2) (android-kotlin "^e: \\(.[^:]*\\): (\\([0-9]*\\), \\([0-9]*\\)" 1 2 3)) :hydra-option nil :entry-command-with-hydra-off nil :project-root "[ORACLE-SANDBOX]/checkout/") :compile (:wait :finished :buffer "-*- mode: android-env-compile; default-directory: \"[ORACLE-SANDBOX]/checkout/\" -*-\nAndroid Compile started at <TIME>\n\ncd [ORACLE-SANDBOX]/checkout/; ./gradlew assembleDevDebug\nargv: assembleDevDebug\ncwd: [ORACLE-SANDBOX]/checkout\nBUILD SUCCESSFUL\n\nAndroid Compile finished at <TIME>\n" :mode android-env-compile-mode :error-regexps (android-java android-java-2 android-kotlin)) :unit-test-command "testDevDebug" :unit-test "-*- mode: android-env-compile; default-directory: \"[ORACLE-SANDBOX]/checkout/\" -*-\nAndroid Compile started at <TIME>\n\ncd [ORACLE-SANDBOX]/checkout/; ./gradlew testDevDebug\nargv: testDevDebug\ncwd: [ORACLE-SANDBOX]/checkout\nBUILD SUCCESSFUL\n\nAndroid Compile finished at <TIME>\n" :instrumented-test-command "testDev" :instrumented-test "-*- mode: android-env-compile; default-directory: \"[ORACLE-SANDBOX]/checkout/\" -*-\nAndroid Compile started at <TIME>\n\ncd [ORACLE-SANDBOX]/checkout/; ./gradlew testDev\nargv: testDev\ncwd: [ORACLE-SANDBOX]/checkout\nBUILD SUCCESSFUL\n\nAndroid Compile finished at <TIME>\n" :outside-a-project (:root nil :result "Couldn’t find a gradle project in ancestors directories"))"#
    ]];

    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn javac_and_kotlinc_errors_in_the_build_output_become_navigable_locations() {
    let elisp_form = r##"
(progn
  (aenv-test-write "app/app/src/main/java/com/example/Checkout.java"
                   "package com.example;\nclass Checkout {}\n")
  (aenv-test-write "app/app/src/main/kotlin/com/example/Gateway.kt"
                   "package com.example\nclass Gateway\n")
  (aenv-test-write
   "app/gradlew"
   (concat "#!/bin/sh\n"
           "echo '> Task :app:compileDevDebugJavaWithJavac'\n"
           "echo \":compileDevDebugJavaWithJavac $(pwd)"
           "/app/src/main/java/com/example/Checkout.java:17: \""
           "'cannot find symbol'\n"
           "echo \"$(pwd)/app/src/main/java/com/example/Checkout.java:42:"
           " error: incompatible types: String cannot be converted to int\"\n"
           "echo \"e: $(pwd)/app/src/main/kotlin/com/example/Gateway.kt:"
           " (23, 9): unresolved reference: charge\"\n"
           "echo 'BUILD FAILED in 1s'\n"
           "exit 1\n")
   t)
  (find-file (aenv-test-path "app/app/src/main/java/com/example/Checkout.java"))
  (android-env-compile "assembleDevDebug")
  (let ((wait (aenv-test-await "*android-env-compile*")))
    (list :wait wait
          :buffer (aenv-test-compilation-text "*android-env-compile*")
          ;; One location per registered regexp: the `:compile' form, the
          ;; plain javac form, and the kotlin `e:' form with a column.
          :locations (aenv-test-locations "*android-env-compile*"))))
"##;

    let expect = expect![[
        r#"OK (:wait :finished :buffer "-*- mode: android-env-compile; default-directory: \"[ORACLE-SANDBOX]/app/\" -*-\nAndroid Compile started at <TIME>\n\ncd [ORACLE-SANDBOX]/app/; ./gradlew assembleDevDebug\n> Task :app:compileDevDebugJavaWithJavac\n:compileDevDebugJavaWithJavac [ORACLE-SANDBOX]/app/app/src/main/java/com/example/Checkout.java:17: cannot find symbol\n[ORACLE-SANDBOX]/app/app/src/main/java/com/example/Checkout.java:42: error: incompatible types: String cannot be converted to int\ne: [ORACLE-SANDBOX]/app/app/src/main/kotlin/com/example/Gateway.kt: (23, 9): unresolved reference: charge\nBUILD FAILED in 1s\n\nAndroid Compile exited abnormally with code 1 at <TIME>\n" :locations ((:file "[ORACLE-SANDBOX]/app/app/src/main/java/com/example/Checkout.java" :line 17 :column nil) (:file "[ORACLE-SANDBOX]/app/app/src/main/java/com/example/Checkout.java" :line 42 :column nil) (:file "[ORACLE-SANDBOX]/app/app/src/main/kotlin/com/example/Gateway.kt" :line 23 :column 9)))"#
    ]];

    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn adb_commands_are_built_from_the_sdk_root_and_quoted_for_the_shell() {
    let elisp_form = r##"
(progn
  (aenv-test-install-sdk)
  (let ((adb (android-env-adb)))
    (android-env-logcat-clear)
    (let ((cleared (with-current-buffer "*Shell Command Output*"
                     (buffer-substring-no-properties (point-min) (point-max)))))
      (android-env-uninstall-app "com.example.checkout")
      (let ((uninstalled (with-current-buffer android-env-adb-buffer-name
                           (buffer-substring-no-properties
                            (point-min) (point-max)))))
        ;; A deeplink with a space and a query string: the whole `am start'
        ;; line is handed to `adb shell' as one argument, so the space has to
        ;; survive `shell-quote-argument'.
        (android-env-deeplink "myapp://item/42?ref=spring sale")
        (list :adb-path adb
              :sdk-root (getenv "ANDROID_SDK_ROOT")
              :adb-buffer-name android-env-adb-buffer-name
              :logcat-clear-output cleared
              :uninstall-output uninstalled
              :deeplink-output (with-current-buffer android-env-adb-buffer-name
                                 (buffer-substring-no-properties
                                  (point-min) (point-max)))
              :recorded (aenv-test-argv))))))
"##;

    let expect = expect![[
        r#"OK (:adb-path "[ORACLE-SANDBOX]/sdk/platform-tools/adb" :sdk-root "[ORACLE-SANDBOX]/sdk" :adb-buffer-name "*android-adb*" :logcat-clear-output "logcat cleared\n" :uninstall-output "ok\n" :deeplink-output "ok\n" :recorded ("adb [logcat] [-c]" "adb [shell] [pm] [uninstall] [com.example.checkout]" "adb [shell] [am start -a android.intent.action.VIEW -d \"myapp://item/42?ref=spring sale\"]"))"#
    ]];

    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn logcat_streams_from_a_real_adb_process_and_a_tag_restarts_it_filtered() {
    let elisp_form = r##"
(progn
  (aenv-test-install-sdk)
  (android-env-logcat "")
  ;; Waiting only for the process to die captures a buffer that is about to
  ;; change: its last output and its sentinel line are still queued, and if
  ;; the next command runs first the sentinel lands in the erased buffer.
  (let ((settled-one (aenv-test-settle "*Android Logcat*"))
        (unfiltered
         (with-current-buffer "*Android Logcat*"
           (list :text (buffer-substring-no-properties (point-min) (point-max))
                 :view-mode view-mode
                 :read-only buffer-read-only
                 :process (and (get-buffer-process "*Android Logcat*") t)))))
    (android-env-logcat "Checkout")
    (aenv-test-settle "*Android Logcat*")
    (let ((filtered
           (with-current-buffer "*Android Logcat*"
             (buffer-substring-no-properties (point-min) (point-max)))))
      (android-env-logcat-crash)
      (aenv-test-settle "*Android Logcat*")
      (list :settled settled-one
            :unfiltered unfiltered
            :filtered filtered
            :crash (with-current-buffer "*Android Logcat*"
                     (buffer-substring-no-properties (point-min) (point-max)))
            ;; One buffer throughout: each call kills the previous process
            ;; and erases what it had printed.
            :logcat-buffers
            (seq-filter (lambda (name) (string-match-p "Logcat" name))
                        (mapcar #'buffer-name (buffer-list)))
            :recorded (aenv-test-argv)))))
"##;

    let expect = expect![[
        r#"OK (:settled :settled :unfiltered (:text "I/Checkout( 911): charge accepted\nD/Gateway( 911): retrying\nI/Sync( 1024): idle\n\nProcess Android Logcat finished\n" :view-mode t :read-only t :process nil) :filtered "I/Checkout( 911): charge accepted\n\nProcess Android Logcat finished\n" :crash "F/libc( 911): Fatal signal 11 in tid 911\n\nProcess Android Logcat finished\n" :logcat-buffers ("*Android Logcat*") :recorded ("adb [logcat]" "adb [logcat] [*:S] [Checkout]" "adb [logcat] [-b] [crash]"))"#
    ]];

    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn listing_avds_goes_through_the_sdk_avdmanager_and_loses_the_first_device() {
    let elisp_form = r##"
(progn
  (aenv-test-install-sdk)
  ;; avdmanager prints three names, NUL separated, exactly as
  ;; `list avd --compact -0' does.
  (let* ((raw (shell-command-to-string
               (concat (getenv "ANDROID_SDK_ROOT")
                       "/tools/bin/avdmanager list avd --compact -0")))
         (printed (split-string raw "\0" t))
         (listed (android-env-avd-list)))
    ;; `android-env-avd-list' pops the first element as though it were a
    ;; header and then deletes any repeat of it.  `--compact' prints no
    ;; header, so the first real device is dropped -- a user with a single
    ;; AVD is offered an empty list.
    (cl-letf (((symbol-function 'completing-read)
               (lambda (&rest _ignored) "Nexus_5X_API_29")))
      (android-env-avd))
    (let ((buffer (car (seq-filter
                        (lambda (name) (string-match-p "emulator" name))
                        (mapcar #'buffer-name (buffer-list))))))
      (aenv-test-settle buffer)
      (list :emulator-command android-env-emulator-command
            :avdmanager-printed printed
            :avd-list listed
            :dropped (seq-remove (lambda (name) (member name listed)) printed)
            ;; The package's own format string leaves the buffer name
            ;; unbalanced -- it writes "*android-emulator-%s" with no
            ;; closing star.
            :launch-buffer buffer
            :launch-output (with-current-buffer buffer
                             (buffer-substring-no-properties
                              (point-min) (point-max)))
            :recorded (aenv-test-argv)))))
"##;

    let expect = expect![[
        r#"OK (:emulator-command "[ORACLE-SANDBOX]/sdk/emulator/emulator" :avdmanager-printed ("Pixel_6_API_33" "Pixel_Tablet_API_34" "Nexus_5X_API_29") :avd-list ("Pixel_Tablet_API_34" "Nexus_5X_API_29") :dropped ("Pixel_6_API_33") :launch-buffer "*android-emulator-Nexus_5X_API_29" :launch-output "boot completed: @Nexus_5X_API_29\n" :recorded ("avdmanager [list] [avd] [--compact] [-0]" "avdmanager [list] [avd] [--compact] [-0]" "avdmanager [list] [avd] [--compact] [-0]" "emulator [@Nexus_5X_API_29]"))"#
    ]];

    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn refactoring_a_source_tree_rewrites_every_file_and_treats_the_mapping_as_regexps() {
    let elisp_form = r##"
(progn
  ;; Three rows of a real androidx-class-mapping.csv.
  (aenv-test-write
   "migrate/androidx-class-mapping.csv"
   (concat "android.support.v4.app.Fragment,androidx.fragment.app.Fragment\n"
           "android.support.v7.widget.RecyclerView,"
           "androidx.recyclerview.widget.RecyclerView\n"
           "android.arch.lifecycle.ViewModel,androidx.lifecycle.ViewModel\n"))
  (aenv-test-write
   "migrate/src/Checkout.java"
   (concat "import android.support.v4.app.Fragment;\n"
           "import android.support.v7.widget.RecyclerView;\n"
           "class Checkout extends Fragment {\n"
           "  RecyclerView items;\n"
           "}\n"))
  (aenv-test-write
   "migrate/src/Gateway.kt"
   (concat "import android.arch.lifecycle.ViewModel\n"
           "class Gateway : ViewModel()\n"))
  ;; A class whose name only matches because the mapping's dots are regexp
  ;; wildcards.  Nothing in this file should be migrated.
  (aenv-test-write
   "migrate/src/Untouched.java"
   (concat "import androidXsupportXv4XappXFragment;\n"
           "import com.example.Fragment;\n"))
  (aenv-test-write "migrate/src/README.md" "android.support.v4.app.Fragment\n")
  (setq android-env-refactor-file
        (aenv-test-path "migrate/androidx-class-mapping.csv"))
  (let ((mapping (android-env-refactor-map android-env-refactor-file))
        (default-directory (aenv-test-path "migrate/src/")))
    (android-env-recursive-refactor "\\.\\(java\\|kt\\)\\'")
    (list :mapping mapping
          :refactor-file-configured (and android-env-refactor-file t)
          :files
          (mapcar (lambda (name)
                    (cons name (aenv-test-read (aenv-test-path
                                                (concat "migrate/src/" name)))))
                  '("Checkout.java" "Gateway.kt" "Untouched.java" "README.md"))
          ;; A second pass has nothing left to replace.
          :second-pass-replacements
          (with-current-buffer (find-file-noselect
                                (aenv-test-path "migrate/src/Checkout.java"))
            (android-env-refactor))
          ;; The docstring says this returns an alist of pid and process; it
          ;; splits on the literal "d " and returns neither.
          :pid-assoc (android-env-logcat-pid-assoc "911 com.example.checkout"))))
"##;

    let expect = expect![[
        r#"OK (:mapping (("android.arch.lifecycle.ViewModel" "androidx.lifecycle.ViewModel") ("android.support.v7.widget.RecyclerView" "androidx.recyclerview.widget.RecyclerView") ("android.support.v4.app.Fragment" "androidx.fragment.app.Fragment")) :refactor-file-configured t :files (("Checkout.java" . "import androidx.fragment.app.Fragment;\nimport androidx.recyclerview.widget.RecyclerView;\nclass Checkout extends Fragment {\n  RecyclerView items;\n}\n") ("Gateway.kt" . "import androidx.lifecycle.ViewModel\nclass Gateway : ViewModel()\n") ("Untouched.java" . "import androidx.fragment.app.Fragment;\nimport com.example.Fragment;\n") ("README.md" . "android.support.v4.app.Fragment\n")) :second-pass-replacements 0 :pid-assoc ("911 com.example.checkout"))"#
    ]];

    assert_android_env_parity(elisp_form, expect);
}
