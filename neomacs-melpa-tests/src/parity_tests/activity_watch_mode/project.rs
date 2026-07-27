use expect_test::expect;

use super::assert_activity_watch_mode_parity;

#[test]
fn activity_watch_mode_blank_predicate_handles_nil_empty_whitespace_and_invalid_values() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (list
            value
            (condition-case error
                (activity-watch--s-blank
                 value)
              (error
               (list
                (car error)
                (cdr error))))))
         '(nil
           ""
           " "
           "\t\n"
           "project"
           0
           symbol))"##;
    let expect = expect![[
        r#"OK ((nil t) ("" t) (" " nil) ("\11\n" nil) ("project" nil) (0 (wrong-type-argument (sequencep 0))) (symbol (wrong-type-argument (sequencep symbol))))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_cwd_resolver_normalizes_directory_names_and_nil() {
    let elisp_form = r##"(mapcar
         (lambda (directory)
           (let ((default-directory
                  directory))
             (list
              directory
              (condition-case error
                  (activity-watch-project-name-cwd)
                (error
                 (list
                  (car error)
                  (cdr error)))))))
         '("/workspace/project/"
           "/workspace/project"
           "/"
           nil))"##;
    let expect = expect![[
        r#"OK (("/workspace/project/" "project") ("/workspace/project" "project") ("/" "") (nil nil))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_project_resolution_order_cache_refresh_and_default_match() {
    let elisp_form = r##"(let ((activity-watch-project-name-resolvers
                '(empty missing winner later))
               (activity-watch-project-name-default
                "fallback")
               calls)
         (cl-letf
           (((symbol-function
                'activity-watch-project-name-empty)
               (lambda ()
                 (push
                  'empty
                  calls)
                 ""))
              ((symbol-function
                'activity-watch-project-name-winner)
               (lambda ()
                 (push
                  'winner
                  calls)
                 "chosen"))
              ((symbol-function
                'activity-watch-project-name-later)
               (lambda ()
                 (push
                  'later
                  calls)
                 "too-late")))
           (list
            (with-temp-buffer
              (let ((first
                     (activity-watch--get-project))
                    first-calls
                    second
                    second-calls
                    refreshed
                    refreshed-calls)
                (setq first-calls
                      (nreverse calls)
                      calls nil
                      second
                      (activity-watch--get-project)
                      second-calls
                      (nreverse calls)
                      calls nil
                      refreshed
                      (activity-watch--get-project
                       t)
                      refreshed-calls
                      (nreverse calls))
                (list
                 first
                 first-calls
                 second
                 second-calls
                 refreshed
                 refreshed-calls
                 activity-watch-project-name
                 (local-variable-p
                  'activity-watch-project-name))))
            (let ((activity-watch-project-name-resolvers
                   '(missing empty)))
              (with-temp-buffer
                (list
                 (activity-watch--get-project)
                 activity-watch-project-name))))))"##;
    let expect = expect![[
        r#"OK (("chosen" (empty winner) "chosen" nil "chosen" (empty winner) "chosen" t) ("fallback" "fallback"))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_feature_resolver_macro_expands_symbol_quoted_and_dynamic_features() {
    let elisp_form = r##"(mapcar
         (lambda (form)
           (list
            form
            (macroexpand-1
             form)))
         '((activity-watch--gen-feature-resolver demo sample
             "Sample resolver."
             (demo-name))
           (activity-watch--gen-feature-resolver 'quoted quoted-name
             (quoted-name))
           (activity-watch--gen-feature-resolver
               (if condition feature-a feature-b)
               dynamic
             (dynamic-name))))"##;
    let expect = expect![[
        r#"OK (((activity-watch--gen-feature-resolver demo sample "Sample resolver." #1=(demo-name)) (progn (defun activity-watch-project-name-sample nil "Check if feature `demo' is provided, and when it is, use it to find the project's name.\n\nSample resolver." (when (featurep demo) . #2=(#1#))) (defun activity-watch-project-name-sample-force nil "Try to require feature `demo', and on success use it to find the project's name.\n\nSample resolver." (when (require demo . #5=(nil t)) . #2#)))) ((activity-watch--gen-feature-resolver #3='quoted quoted-name #4=(quoted-name)) (progn (defun activity-watch-project-name-quoted-name nil "Check if feature `quoted' is provided, and when it is, use it to find the project's name." (when (featurep #3#) . #6=(#4#))) (defun activity-watch-project-name-quoted-name-force nil "Try to require feature `quoted', and on success use it to find the project's name." (when (require #3# . #5#) . #6#)))) ((activity-watch--gen-feature-resolver #7=(if condition feature-a feature-b) dynamic #8=(dynamic-name)) (progn (defun activity-watch-project-name-dynamic nil "Check if feature `<feature>' is provided, and when it is, use it to find the project's name." (when (featurep #7#) . #9=(#8#))) (defun activity-watch-project-name-dynamic-force nil "Try to require feature `<feature>', and on success use it to find the project's name." (when (require #7# . #5#) . #9#)))))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_project_and_projectile_resolvers_cover_loaded_absent_and_forced_paths() {
    let elisp_form = r##"(let ((available-features nil)
               (require-succeeds nil)
               (current-project
                'project-object)
               (projectile-present t)
               calls)
         (cl-letf
             (((symbol-function
                'featurep)
               (let ((real-featurep
                      (symbol-function
                       'featurep)))
                 (lambda (feature)
                   (if
                       (memq feature
                             '(project projectile))
                       (and
                        (memq feature
                              available-features)
                        t)
                     (funcall
                      real-featurep
                      feature)))))
              ((symbol-function
                'project-current)
               (lambda ()
                 (push
                  'project-current
                  calls)
                 current-project))
              ((symbol-function
                'project-name)
               (lambda (project)
                 (push
                  (list
                   'project-name
                   project)
                  calls)
                 "named-project"))
              ((symbol-function
                'project-roots)
               (lambda (project)
                 (push
                  (list
                   'project-roots
                   project)
                  calls)
                 '("/workspace/fallback/")))
              ((symbol-function
                'projectile-project-p)
               (lambda ()
                 (push
                  'projectile-project-p
                  calls)
                 projectile-present))
              ((symbol-function
                'projectile-project-name)
               (lambda ()
                 (push
                  'projectile-project-name
                  calls)
                 "projectile-name"))
              ((symbol-function
                'require)
               (let ((real-require
                      (symbol-function
                       'require)))
                 (lambda
                   (feature
                    &optional filename noerror)
                   (if
                       (memq feature
                             '(project projectile))
                       (progn
                         (push
                          (list
                           'require
                           feature
                           filename
                           noerror)
                          calls)
                         require-succeeds)
                     (funcall
                      real-require
                      feature
                      filename
                      noerror))))))
           (let ((absent-project
                  (activity-watch-project-name-project))
                 (absent-projectile
                  (activity-watch-project-name-projectile)))
             (setq require-succeeds
                   t)
             (let ((forced-project
                    (activity-watch-project-name-project-force))
                   (forced-projectile
                    (activity-watch-project-name-projectile-force)))
               (setq require-succeeds
                     nil)
               (let ((failed-forced-project
                      (activity-watch-project-name-project-force))
                     (failed-forced-projectile
                      (activity-watch-project-name-projectile-force)))
                 (setq available-features
                       '(project projectile))
                 (let ((loaded-project
                        (activity-watch-project-name-project))
                       (loaded-projectile
                        (activity-watch-project-name-projectile)))
                   (setq current-project
                         nil
                         projectile-present
                         nil)
                 (list
                  absent-project
                  absent-projectile
                  forced-project
                  forced-projectile
                      failed-forced-project
                      failed-forced-projectile
                      loaded-project
                      loaded-projectile
                      (activity-watch-project-name-project)
                      (activity-watch-project-name-projectile)
                      (nreverse calls))))))))"##;
    let expect = expect![[
        r#"OK (nil nil "named-project" "projectile-name" nil nil "named-project" "projectile-name" nil nil ((require project nil t) project-current (project-name project-object) (require projectile nil t) projectile-project-p projectile-project-name (require project nil t) (require projectile nil t) project-current (project-name project-object) projectile-project-p projectile-project-name project-current projectile-project-p))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_project_resolver_uses_roots_fallback_when_project_name_is_unavailable() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'featurep)
               (let ((real-featurep
                      (symbol-function
                       'featurep)))
                 (lambda (feature)
                   (if
                       (eq feature
                           'project)
                       t
                     (funcall
                      real-featurep
                      feature)))))
              ((symbol-function
                'project-current)
               (lambda ()
                 (push
                  'project-current
                  calls)
                 'project-object))
              ((symbol-function
                'project-roots)
               (lambda (project)
                 (push
                  (list
                   'project-roots
                   project)
                  calls)
                 '("/workspace/fallback-name/")))
              ((symbol-function
                'project-name)
               nil))
           (list
            (fboundp
             'project-name)
            (activity-watch-project-name-project)
            (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (nil "fallback-name" (project-current (project-roots project-object)))"#]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_magit_directory_and_origin_resolvers_parse_all_remote_shapes() {
    let elisp_form = r##"(let ((available-features nil)
               (require-succeeds t)
               (toplevel
                "/workspace/local-repo/")
               (remote
                "https://github.com/owner/repository.git")
               calls)
         (cl-letf
             (((symbol-function
                'featurep)
               (let ((real-featurep
                      (symbol-function
                       'featurep)))
                 (lambda (feature)
                   (if
                       (eq feature
                           'magit)
                       (and
                        (memq
                         'magit
                         available-features)
                        t)
                     (funcall
                      real-featurep
                      feature)))))
              ((symbol-function
                'require)
               (let ((real-require
                      (symbol-function
                       'require)))
                 (lambda
                   (feature
                    &optional filename noerror)
                   (if
                       (eq feature
                           'magit)
                       (progn
                         (push
                          (list
                           'require
                           feature
                           filename
                           noerror)
                          calls)
                         require-succeeds)
                     (funcall
                      real-require
                      feature
                      filename
                      noerror)))))
              ((symbol-function
                'magit-toplevel)
               (lambda ()
                 (push
                  'magit-toplevel
                  calls)
                 toplevel))
              ((symbol-function
                'magit-git-string)
               (lambda
                 (&rest arguments)
                 (push
                  (cons
                   'magit-git-string
                   arguments)
                  calls)
                 remote)))
           (let ((absent-directory
                  (activity-watch-project-name-magit-dir))
                 (absent-origin
                  (activity-watch-project-name-magit-origin)))
             (setq available-features
                   '(magit))
             (let ((loaded-directory
                    (activity-watch-project-name-magit-dir)))
               (setq toplevel
                     nil)
               (let ((missing-directory
                      (activity-watch-project-name-magit-dir))
                     (origins
                      (mapcar
                       (lambda (value)
                         (setq remote value)
                         (list
                          value
                          (activity-watch-project-name-magit-origin)))
                       '("https://github.com/owner/repository.git"
                         "git@github.com:owner/repository.git"
                         "ssh://host/owner/repository"
                         nil))))
                 (setq toplevel
                       "/workspace/forced-repo/"
                       remote
                       "https://example.invalid/owner/forced-origin.git")
                 (let ((forced-directory
                        (activity-watch-project-name-magit-dir-force))
                       (forced-origin
                        (activity-watch-project-name-magit-origin-force)))
                   (setq require-succeeds
                         nil)
                   (list
                    absent-directory
                    absent-origin
                    loaded-directory
                    missing-directory
                    origins
                    forced-directory
                    forced-origin
                    (activity-watch-project-name-magit-dir-force)
                    (activity-watch-project-name-magit-origin-force)
                    (nreverse calls))))))))"##;
    let expect = expect![[
        r#"OK (nil nil "local-repo" nil (("https://github.com/owner/repository.git" "repository") ("git@github.com:owner/repository.git" "repository") ("ssh://host/owner/repository" "repository") (nil nil)) "forced-repo" "forced-origin" nil nil (magit-toplevel magit-toplevel (magit-git-string "remote" "get-url" "origin") (magit-git-string "remote" "get-url" "origin") (magit-git-string "remote" "get-url" "origin") (magit-git-string "remote" "get-url" "origin") (require magit nil t) magit-toplevel (require magit nil t) (magit-git-string "remote" "get-url" "origin") (require magit nil t) (require magit nil t)))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_org_property_injection_mutates_only_valid_active_heartbeats() {
    let elisp_form = r##"(progn
         (defvar org-clock-marker nil)
         (let ((clock-buffer
                (generate-new-buffer
                 " *activity-watch-clock*"))
               (calls nil))
           (unwind-protect
               (let ((marker
                      (with-current-buffer clock-buffer
                        (insert
                         "* Clock")
                        (copy-marker
                         (point-min)))))
                 (cl-letf
                     (((symbol-function
                        'org-entry-get)
                       (lambda
                         (received-marker property)
                         (push
                          (list
                           (eq received-marker
                               marker)
                           property)
                          calls)
                         (cond
                          ((equal property
                                  "TICKET_ID")
                           "ABC-123")
                          ((equal property
                                  "Mixed_Case")
                           "value")))))
                   (let ((base
                          '((timestamp . "time")
                            (data
                             (project . "project")))))
                     (let ((inactive
                            (let ((activity-watch-org-clock-active
                                   nil)
                                  (org-clock-marker
                                   marker))
                              (activity-watch--inject-org-property
                               (copy-tree base)))))
                       (setq features
                             (delq
                              'org-clock
                              features))
                       (let ((without-feature
                              (let ((activity-watch-org-clock-active
                                     t)
                                    (org-clock-marker
                                     marker))
                                (activity-watch--inject-org-property
                                 (copy-tree base)))))
                         (provide
                          'org-clock)
                         (list
                          inactive
                          without-feature
                          (let ((activity-watch-org-clock-active
                                 t)
                                (org-clock-marker
                                 nil))
                            (activity-watch--inject-org-property
                             (copy-tree base)))
                          (let ((activity-watch-org-clock-active
                                 t)
                                (org-clock-marker
                                 (make-marker)))
                            (activity-watch--inject-org-property
                             (copy-tree base)))
                          (let ((activity-watch-org-clock-active
                                 t)
                                (activity-watch-org-clock-property
                                 "TICKET_ID")
                                (org-clock-marker
                                 marker))
                            (activity-watch--inject-org-property
                             (copy-tree base)))
                          (let ((activity-watch-org-clock-active
                                 t)
                                (activity-watch-org-clock-property
                                 "EMPTY")
                                (org-clock-marker
                                 marker))
                            (activity-watch--inject-org-property
                             (copy-tree base)))
                          (let ((activity-watch-org-clock-active
                                 t)
                                (activity-watch-org-clock-property
                                 "Mixed_Case")
                                (org-clock-marker
                                 marker))
                            (activity-watch--inject-org-property
                             '((timestamp . "time"))))
                          (nreverse calls)))))))
             (when
                 (buffer-live-p clock-buffer)
               (kill-buffer clock-buffer)))))"##;
    let expect = expect![[
        r#"OK (((timestamp . "time") (data (project . "project"))) ((timestamp . "time") (data (project . "project"))) ((timestamp . "time") (data (project . "project"))) ((timestamp . "time") (data (project . "project"))) ((timestamp . "time") (data (ticket_id . "ABC-123") (project . "project"))) ((timestamp . "time") (data (project . "project"))) ((timestamp . "time")) ((t "TICKET_ID") (t "EMPTY") (t "Mixed_Case")))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}
