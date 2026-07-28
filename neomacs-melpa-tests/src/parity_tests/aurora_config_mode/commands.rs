use expect_test::expect;

use super::assert_aurora_config_mode_parity;

#[test]
fn aurora_config_mode_run_aurora_builds_exact_cli_command_from_operation_job_and_basename() {
    let elisp_form = r##"(with-temp-buffer
          (setq
           buffer-file-name
           (expand-file-name
            "tmp/aurora-config/production/service.aurora"
            default-directory))
          (let (calls)
            (cl-letf
                (((symbol-function 'compile)
                  (lambda (command)
                    (push
                     (list
                      command
                      compile-command
                      (file-name-nondirectory
                       buffer-file-name)
                      (current-buffer))
                     calls)
                    :compile-result)))
              (list
               (aurora-config-run-aurora
                "inspect"
                "cluster/role/prod/service")
               (nreverse
                (mapcar
                 (lambda (call)
                   (list
                    (nth 0 call)
                    (nth 1 call)
                    (nth 2 call)
                    (bufferp
                     (nth 3 call))))
                 calls))
               compile-command))))"##;
    let expect = expect![[
        r#"OK (:compile-result (("aurora inspect cluster/role/prod/service service.aurora" "aurora inspect cluster/role/prod/service service.aurora" "service.aurora" t)) "make -k -j22 ")"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}

#[test]
fn aurora_config_mode_run_aurora_forwards_every_operation_and_jobpath_without_validation() {
    let elisp_form = r##"(with-temp-buffer
          (setq
           buffer-file-name
           (expand-file-name
            "tmp/aurora-config/job.mesos"
            default-directory))
          (let (commands)
            (cl-letf
                (((symbol-function 'compile)
                  (lambda (command)
                    (push command commands)
                    (list
                     :compiled
                     command))))
              (list
               (mapcar
                (lambda (case)
                  (list
                   case
                   (aurora-config-run-aurora
                    (nth 0 case)
                    (nth 1 case))))
                '(("inspect" "west/role/dev/job")
                  ("diff" "east/role/stage/job")
                  ("" "")
                  ("custom-command" "one/two/three/four")))
               (nreverse commands)))))"##;
    let expect = expect![[
        r#"OK (((("inspect" "west/role/dev/job") (:compiled "aurora inspect west/role/dev/job job.mesos")) (("diff" "east/role/stage/job") (:compiled "aurora diff east/role/stage/job job.mesos")) (("" "") (:compiled "aurora   job.mesos")) (("custom-command" "one/two/three/four") (:compiled "aurora custom-command one/two/three/four job.mesos"))) ("aurora inspect west/role/dev/job job.mesos" "aurora diff east/role/stage/job job.mesos" "aurora   job.mesos" "aurora custom-command one/two/three/four job.mesos"))"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}

#[test]
fn aurora_config_mode_command_construction_preserves_unquoted_spaces_and_shell_metacharacters() {
    let elisp_form = r##"(with-temp-buffer
          (setq
           buffer-file-name
           (expand-file-name
            "tmp/aurora-config/config name;echo.aurora"
            default-directory))
          (let (commands)
            (cl-letf
                (((symbol-function 'compile)
                  (lambda (command)
                    (push command commands)
                    command)))
              (list
               (aurora-config-run-aurora
                "inspect --verbose"
                "cluster/role env/job;touch-marker")
               (nreverse commands)))))"##;
    let expect = expect![[
        r#"OK ("aurora inspect --verbose cluster/role env/job;touch-marker config name;echo.aurora" ("aurora inspect --verbose cluster/role env/job;touch-marker config name;echo.aurora"))"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}

#[test]
fn aurora_config_mode_run_aurora_missing_file_and_non_string_parts_fail_before_compile() {
    let elisp_form = r##"(let (compile-calls)
          (cl-letf
              (((symbol-function 'compile)
                (lambda (command)
                  (push command compile-calls)
                  :unexpected)))
            (list
             (with-temp-buffer
               (aurora-config-test-error-data
                (lambda ()
                  (aurora-config-run-aurora
                   "inspect"
                   "cluster/role/env/job"))))
             (with-temp-buffer
               (setq
                buffer-file-name
                "fixture.aurora")
               (mapcar
                (lambda (case)
                  (list
                   case
                   (aurora-config-test-error-data
                    (lambda ()
                      (aurora-config-run-aurora
                       (nth 0 case)
                       (nth 1 case))))))
                '((inspect "valid/path")
                  ("inspect" job-symbol)
                  (42 "valid/path")
                  (nil nil)
                  (("inspect") "valid/path"))))
             (nreverse compile-calls))))"##;
    let expect = expect![[
        r#"OK ((:error wrong-type-argument (stringp nil)) (((inspect "valid/path") (:error wrong-type-argument (sequencep inspect))) (("inspect" job-symbol) (:error wrong-type-argument (sequencep job-symbol))) ((42 "valid/path") (:error wrong-type-argument (sequencep 42))) ((nil nil) (:ok :unexpected)) ((("inspect") "valid/path") (:error wrong-type-argument (characterp "inspect")))) ("aurora   fixture.aurora"))"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}

#[test]
fn aurora_config_mode_inspect_and_diff_delegate_exact_operation_jobpath_and_return_value() {
    let elisp_form = r##"(let (calls)
          (cl-letf
              (((symbol-function
                 'aurora-config-run-aurora)
                (lambda (command jobpath)
                  (push
                   (list command jobpath)
                   calls)
                  (list
                   :result
                   command
                   jobpath))))
            (list
             (aurora-config-inspect
              "cluster/role/prod/api")
             (aurora-config-diff
              "cluster/role/prod/api")
             (aurora-config-inspect "")
             (aurora-config-diff nil)
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((:result "inspect" "cluster/role/prod/api") (:result "diff" "cluster/role/prod/api") (:result "inspect" "") (:result "diff" nil) (("inspect" "cluster/role/prod/api") ("diff" "cluster/role/prod/api") ("inspect" "") ("diff" nil)))"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}

#[test]
fn aurora_config_mode_interactive_inspect_and_diff_prompt_update_history_and_dispatch() {
    let elisp_form = r##"(with-temp-buffer
          (setq
           aurora-config-last-job-path
           "initial/role/env/job")
          (let ((answers
                 '("inspect/role/env/job"
                   "diff/role/env/job"))
                prompts
                runs)
            (cl-letf
                (((symbol-function 'read-string)
                  (lambda (prompt initial)
                    (push
                     (list prompt initial)
                     prompts)
                    (pop answers)))
                 ((symbol-function
                   'aurora-config-run-aurora)
                  (lambda (command jobpath)
                    (push
                     (list command jobpath)
                     runs)
                    (list
                     :ran
                     command
                     jobpath))))
              (list
               (call-interactively
                #'aurora-config-inspect)
               (call-interactively
                #'aurora-config-diff)
               (nreverse prompts)
               (nreverse runs)
               aurora-config-last-job-path
               answers))))"##;
    let expect = expect![[
        r#"OK ((:ran "inspect" "inspect/role/env/job") (:ran "diff" "diff/role/env/job") (("Job path as 'cluster/role/env/job': " "initial/role/env/job") ("Job path as 'cluster/role/env/job': " "inspect/role/env/job")) (("inspect" "inspect/role/env/job") ("diff" "diff/role/env/job")) "diff/role/env/job" nil)"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}

#[test]
fn aurora_config_mode_interactive_prompt_failure_prevents_command_dispatch_and_preserves_history() {
    let elisp_form = r##"(with-temp-buffer
          (setq
           aurora-config-last-job-path
           "stable/role/env/job")
          (let (runs)
            (cl-letf
                (((symbol-function 'read-string)
                  (lambda (&rest _)
                    (error
                     "fixture prompt failed")))
                 ((symbol-function
                   'aurora-config-run-aurora)
                  (lambda (&rest arguments)
                    (push arguments runs)
                    :unexpected)))
              (list
               (aurora-config-test-error-data
                (lambda ()
                  (call-interactively
                   #'aurora-config-inspect)))
               (aurora-config-test-error-data
                (lambda ()
                  (call-interactively
                   #'aurora-config-diff)))
               aurora-config-last-job-path
               (nreverse runs)))))"##;
    let expect = expect![[
        r#"OK ((:error error ("fixture prompt failed")) (:error error ("fixture prompt failed")) "stable/role/env/job" nil)"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}

#[test]
fn aurora_config_mode_compile_failure_exposes_constructed_dynamic_command_then_unwinds_cleanly() {
    let elisp_form = r##"(with-temp-buffer
          (setq
           buffer-file-name
           (expand-file-name
            "tmp/aurora-config/failing.aurora"
            default-directory))
          (let ((compile-command
                 "outer command")
                observations)
            (cl-letf
                (((symbol-function 'compile)
                  (lambda (command)
                    (push
                     (list
                      command
                      compile-command)
                     observations)
                    (error
                     "fixture compile failure %s"
                     command))))
              (list
               (aurora-config-test-error-data
                (lambda ()
                  (aurora-config-run-aurora
                   "diff"
                   "cluster/role/test/failing")))
               (nreverse observations)
               compile-command))))"##;
    let expect = expect![[
        r#"OK ((:error error ("fixture compile failure aurora diff cluster/role/test/failing failing.aurora")) (("aurora diff cluster/role/test/failing failing.aurora" "aurora diff cluster/role/test/failing failing.aurora")) "outer command")"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}

#[test]
fn aurora_config_mode_command_arity_failures_are_exact_and_side_effect_free() {
    let elisp_form = r##"(let (calls)
          (cl-letf
              (((symbol-function
                 'aurora-config-run-aurora)
                (lambda (&rest arguments)
                  (push arguments calls)
                  :unexpected)))
            (list
             (aurora-config-test-error-data
              (lambda ()
                (aurora-config-inspect)))
             (aurora-config-test-error-data
              (lambda ()
                (aurora-config-inspect
                 "one"
                 "two")))
             (aurora-config-test-error-data
              (lambda ()
                (aurora-config-diff)))
             (aurora-config-test-error-data
              (lambda ()
                (aurora-config-diff
                 "one"
                 "two")))
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((:error wrong-number-of-arguments (#1=#[(jobpath) ((aurora-config-run-aurora "inspect" jobpath)) nil nil "Run `aurora inspect JOBPATH' with the config in current buffer." (list (aurora-config-read-jobpath))] 0)) (:error wrong-number-of-arguments (#1# 2)) (:error wrong-number-of-arguments (#2=#[(jobpath) ((aurora-config-run-aurora "diff" jobpath)) nil nil "Run `aurora diff JOBPATH' with the config in current buffer." (list (aurora-config-read-jobpath))] 0)) (:error wrong-number-of-arguments (#2# 2)) nil)"#
    ]];

    assert_aurora_config_mode_parity(elisp_form, expect);
}
