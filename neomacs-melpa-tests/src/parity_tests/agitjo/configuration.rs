use expect_test::expect;

use super::assert_agitjo_parity;

#[test]
fn agitjo_pull_request_type_validation_accepts_forgejo_variants_and_rejects_other_values() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (list value (agitjo--valid-pullreq-type? value)))
         '("for" "draft" "for-review" "normal" for nil 42))"##;
    let expect = expect![[
        r#"OK (("for" ("for" . #1=("draft" . #2=("for-review")))) ("draft" #1#) ("for-review" #2#) ("normal" nil) (for nil) (nil nil) (42 nil))"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_configuration_extracts_remote_target_and_builds_default_refspec() {
    let elisp_form = r##"(let* ((default-directory
                  (file-name-as-directory
                   (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                 (config
                (agitjo--pullreq-configuration
                 :type "for"
                 :source "@"
                 :target "origin/main"
                 :args nil)))
         (cl-letf (((symbol-function 'project-current) (lambda (&rest _) nil)))
           (list
            (agitjo--pullreq-target-name config)
            (agitjo--pullreq-target-remote config)
            (agitjo--pullreq-refspec config)
            (oref config type)
            (oref config source)
            (oref config target)
            (oref config args))))"##;
    let expect =
        expect![[r#"OK ("main" "origin" "@:refs/for/main/@" "for" "@" "origin/main" nil)"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_configuration_refspec_uses_project_topic_with_slashes_verbatim() {
    let elisp_form = r##"(let* ((default-directory
                  (file-name-as-directory
                   (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                 (config
                (agitjo--pullreq-configuration
                 :type "for"
                 :source "@"
                 :target "origin/main"
                 :args nil))
               (agitjo--current-topics '(("/workspace/project/" . "team/session/42"))))
         (cl-letf (((symbol-function 'project-current)
                    (lambda (&rest _) 'project))
                   ((symbol-function 'project-root)
                    (lambda (_project) "/workspace/project/")))
           (list
            (agitjo--pullreq-refspec config)
            (agitjo--pullreq-target-name config)
            (agitjo--pullreq-target-remote config))))"##;
    let expect = expect![[r#"OK ("@:refs/for/main/team/session/42" "main" "origin")"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_push_args_filters_non_git_options_and_preserves_existing_title() {
    let elisp_form = r##"(let* ((default-directory
                  (file-name-as-directory
                   (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                 (config
                (agitjo--pullreq-configuration
                 :type "for"
                 :source "@"
                 :target "origin/main"
                 :args '("--force-with-lease"
                         "--push-option=title=Ship it"
                         "draft"
                         "local-only"))))
         (agitjo--push-args config))"##;
    let expect = expect![[r#"OK ("--force-with-lease" "--push-option=title=WIP: Ship it")"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_draft_push_args_prefix_existing_title_without_mutating_configuration() {
    let elisp_form = r##"(let* ((default-directory
                  (file-name-as-directory
                   (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                (args '("draft"
                 "--push-option=title=Ship it"
                 "--push-option=force-push=true"))
                (config
                 (agitjo--pullreq-configuration
                  :type "for"
                  :source "@"
                  :target "origin/main"
                  :args args))
                (result (agitjo--push-args config)))
         (list result (oref config args) args (eq result args)))"##;
    let expect = expect![[
        r#"OK (("--push-option=title=WIP: Ship it" "--push-option=force-push=true") #1=("draft" "--push-option=title=Ship it" "--push-option=force-push=true") #1# nil)"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_draft_push_args_synthesizes_title_from_real_source_subject() {
    let elisp_form = r##"(let* ((default-directory
                  (file-name-as-directory
                   (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                 (config
                (agitjo--pullreq-configuration
                 :type "for"
                 :source "@"
                 :target "origin/main"
                 :args '("draft" "--atomic")))
               rev-format-call)
         (cl-letf (((symbol-function 'magit-rev-format)
                    (lambda (format revision)
                      (setq rev-format-call
                            (list format revision))
                      "Implement widgets")))
           (list
            (agitjo--push-args config)
            rev-format-call)))"##;
    let expect =
        expect![[r#"OK (("--push-option=title=WIP: Implement widgets" "--atomic") ("%s" "@"))"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_push_orchestration_builds_debug_sync_and_async_git_commands() {
    let elisp_form = r##"(let* ((default-directory
                  (file-name-as-directory
                   (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                 (config
                (agitjo--pullreq-configuration
                 :type "for"
                 :source "@"
                 :target "origin/main"
                 :args '("--atomic")))
               calls)
         (cl-letf (((symbol-function 'project-current) (lambda (&rest _) nil))
                   ((symbol-function 'magit-run-git)
                    (lambda (&rest args)
                      (push (cons 'sync args) calls)
                      17))
                   ((symbol-function 'magit-run-git-async)
                    (lambda (&rest args)
                      (push (cons 'async args) calls)
                      'process)))
           (let ((agitjo--push-pullreq-debug? t))
             (list
              (agitjo--push-pullreq config)
              (let ((agitjo--push-pullreq-debug? nil))
                (agitjo--push-pullreq config t))
              (let ((agitjo--push-pullreq-debug? nil))
                (agitjo--push-pullreq config))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (0 17 process ((sync "push" "-v" "origin" "@:refs/for/main/@" ("--atomic")) (async "push" "-v" "origin" "@:refs/for/main/@" ("--atomic"))))"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}
