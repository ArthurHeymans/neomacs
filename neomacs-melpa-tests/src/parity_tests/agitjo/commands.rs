use expect_test::expect;

use super::assert_agitjo_parity;

#[test]
fn agitjo_target_branch_resolution_maps_existing_local_branch_and_preserves_remote_branch() {
    let elisp_form = r##"(let (checks)
         (cl-letf (((symbol-function 'magit-local-branch-p)
                    (lambda (branch)
                      (equal branch "main")))
                   ((symbol-function 'magit-primary-remote)
                    (lambda () "origin"))
                   ((symbol-function 'magit-remote-branch-p)
                    (lambda (branch)
                      (push (substring-no-properties branch) checks)
                      (equal branch "origin/main"))))
           (list
            (substring-no-properties
             (agitjo-get-target-branch "main"))
            (agitjo-get-target-branch "upstream/release")
            (nreverse checks))))"##;
    let expect = expect![[r#"OK ("origin/main" "upstream/release" ("origin/main"))"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_target_branch_resolution_returns_nil_for_missing_remote_counterpart() {
    let elisp_form = r##"(cl-letf (((symbol-function 'magit-local-branch-p)
                  (lambda (_branch) t))
                 ((symbol-function 'magit-primary-remote)
                  (lambda () "origin"))
                 ((symbol-function 'magit-remote-branch-p)
                  (lambda (_branch) nil)))
         (agitjo-get-target-branch "topic"))"##;
    let expect = expect!["OK nil"];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_last_pull_request_search_returns_latest_link_without_properties() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer " *agitjo-process*")))
         (unwind-protect
             (with-current-buffer buffer
               (insert "remote: https://codeberg.org/team/repo/pulls/7\n")
               (insert "noise\n")
               (insert
                (propertize
                 "remote: https://codeberg.org/team/repo/pulls/42\n"
                 'face 'bold))
               (cl-letf (((symbol-function 'magit-process-buffer)
                          (lambda (&optional _create) buffer)))
                 (let ((link (agitjo--get-last-pullreq)))
                   (list link
                         (text-properties-at 0 link)))))
           (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ("https://codeberg.org/team/repo/pulls/42" nil)"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_visit_last_pull_request_routes_browser_or_user_error() {
    let elisp_form = r##"(let (visited)
         (cl-letf (((symbol-function 'browse-url)
                    (lambda (url &rest _)
                      (push url visited)
                      'opened)))
           (list
            (cl-letf (((symbol-function 'agitjo--get-last-pullreq)
                       (lambda () "https://forge.example/pulls/9")))
              (agitjo-visit-last-pushed-pullreq))
            (nreverse visited)
            (cl-letf (((symbol-function 'agitjo--get-last-pullreq)
                       (lambda () nil)))
              (condition-case error-data
                  (agitjo-visit-last-pushed-pullreq)
                (error
                 (list (car error-data)
                       (cadr error-data))))))))"##;
    let expect = expect![[
        r#"OK (opened ("https://forge.example/pulls/9") (user-error "No pull request link could be found"))"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_setup_binds_real_magit_status_key_and_appends_dispatch_suffix() {
    let elisp_form = r##"(let ((magit-status-mode-map (make-sparse-keymap))
               calls)
         (cl-letf (((symbol-function 'transient-append-suffix)
                    (lambda (&rest args)
                      (push args calls)
                      'appended)))
           (list
            (agitjo-setup "#")
            (lookup-key magit-status-mode-map (kbd "#"))
            (nreverse calls))))"##;
    let expect = expect![[
        r##"OK (appended agitjo-push ((magit-dispatch (0 -1 -1) ("#" "AGit-Flow push" agitjo-push))))"##
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_push_command_routes_new_pull_request_to_draft_and_force_push_to_git() {
    let elisp_form = r##"(let ((default-directory
                (file-name-as-directory
                 (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
               (suffix
                (agitjo-push-pullreq-suffix
                 :command 'agitjo-push-pullreq
                 :description "test"
                 :source (lambda () "@")
                 :target (lambda () "origin/main")))
               calls)
         (cl-letf (((symbol-function 'transient-suffix-object)
                    (lambda () suffix))
                   ((symbol-function 'agitjo-post--setup-buffer)
                    (lambda (config)
                      (push
                       (list 'draft
                             (oref config source)
                             (oref config target)
                             (oref config args))
                       calls)
                      'draft-buffer))
                   ((symbol-function 'agitjo--push-pullreq)
                    (lambda (config &optional _sync)
                      (push
                       (list 'push
                             (oref config source)
                             (oref config target)
                             (oref config args))
                       calls)
                      'process)))
           (list
            (agitjo-push-pullreq
             '("--push-option=title=New"))
            (agitjo-push-pullreq
             '("--push-option=force-push=true"
               "--push-option=title=Update"))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (draft-buffer process ((draft "@" "origin/main" ("--push-option=title=New")) (push "@" "origin/main" ("--push-option=force-push=true" "--push-option=title=Update"))))"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_post_confirm_encodes_real_buffer_and_replaces_stale_description_option() {
    let elisp_form = r##"(let* ((default-directory
                 (file-name-as-directory
                  (getenv "NEOMACS_TEST_WORKSPACE_ROOT")))
                (root
                 (expand-file-name
                  "agitjo-confirm"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (file (expand-file-name "draft.md" root))
                (buffer nil)
                (config
                 (agitjo--pullreq-configuration
                  :type "for"
                  :source "@"
                  :target "origin/main"
                  :args '("--atomic"
                          "--push-option=description=stale")))
                calls sentinel)
         (unwind-protect
             (progn
               (make-directory root t)
               (setq buffer (find-file-noselect file))
               (with-current-buffer buffer
                 (erase-buffer)
                 (insert "Summary\n\nλ details")
                 (setq-local agitjo-post--pullreq-config config))
               (cl-letf (((symbol-function 'agitjo-post--buffer)
                          (lambda () buffer))
                         ((symbol-function 'quit-window)
                          (lambda (&rest args)
                            (push (cons 'quit args) calls)))
                         ((symbol-function 'agitjo--push-pullreq)
                          (lambda (value &optional _sync)
                            (push
                             (list 'push (oref value args))
                             calls)
                            'fake-process))
                         ((symbol-function 'set-process-sentinel)
                          (lambda (process function)
                            (setq sentinel function)
                            (push
                             (list 'sentinel process
                                   (functionp function))
                             calls))))
                 (with-current-buffer buffer
                   (agitjo-post-confirm)))
               (list
                (oref config args)
                (nreverse calls)
                (functionp sentinel)
                (file-exists-p file)))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (#1=("--push-option=description={base64}U3VtbWFyeQoKzrsgZGV0YWlscw==" "--atomic") ((quit :kill nil) (push #1#) (sentinel fake-process t)) t t)"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_push_sentinel_preserves_failed_draft_and_clears_successful_draft() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agitjo-sentinel"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (file (expand-file-name "draft.md" root))
                (status 'exit)
                (code 1)
                calls)
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region "draft body" nil file nil 'silent)
               (cl-letf (((symbol-function 'process-status)
                          (lambda (_process) status))
                         ((symbol-function 'process-exit-status)
                          (lambda (_process) code))
                         ((symbol-function 'magit-process-sentinel)
                          (lambda (process event)
                            (push
                             (list 'magit process event)
                             calls))))
                 (let ((sentinel
                        (agitjo-post--push-sentinel file)))
                   (funcall sentinel 'proc "failed")
                   (let ((after-failure
                          (with-temp-buffer
                            (insert-file-contents file)
                            (buffer-string))))
                     (setq code 0)
                     (funcall sentinel 'proc "finished")
                     (list
                      after-failure
                      (with-temp-buffer
                        (insert-file-contents file)
                        (buffer-string))
                      (nreverse calls))))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect =
        expect![[r#"OK ("draft body" "" ((magit proc "failed") (magit proc "finished")))"#]];
    assert_agitjo_parity(elisp_form, expect);
}
