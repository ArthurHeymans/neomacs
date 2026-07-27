use expect_test::expect;

use super::assert_ada_ts_mode_parity;

#[test]
fn ada_ts_mode_project_file_discovery_covers_default_root_single_and_ambiguous_roots() {
    let elisp_form = r##"(let* ((home
                 (file-name-as-directory
                  (getenv
                   "HOME")))
                (default-root
                 (expand-file-name
                  "ada-project/default-root/"
                  home))
                (default-source
                 (expand-file-name
                  "src/nested/main.adb"
                  default-root))
                (single-root
                 (expand-file-name
                  "ada-project/single-root/"
                  home))
                (single-source
                 (expand-file-name
                  "src/main.adb"
                  single-root))
                (ambiguous-root
                 (expand-file-name
                  "ada-project/ambiguous-root/"
                  home))
                (ambiguous-source
                 (expand-file-name
                  "src/main.adb"
                  ambiguous-root)))
         (dolist (directory
                  (list
                   (file-name-directory
                    default-source)
                   (file-name-directory
                    single-source)
                   (file-name-directory
                    ambiguous-source)))
           (make-directory
            directory
            t))
         (dolist (file
                  (list
                   (expand-file-name
                    "default.gpr"
                    default-root)
                   (expand-file-name
                    "only.gpr"
                    single-root)
                   (expand-file-name
                    "first.gpr"
                    ambiguous-root)
                   (expand-file-name
                    "second.gpr"
                    ambiguous-root)))
           (with-temp-file file))
         (list
          (with-temp-buffer
            (setq
             buffer-file-name
             default-source)
            (file-relative-name
             (ada-ts-mode--default-project-file)
             home))
          (cl-letf
              (((symbol-function
                 'project-current)
                (lambda (&rest _)
                  'fixture-project))
               ((symbol-function
                 'project-root)
                (lambda (_project)
                  single-root)))
            (with-temp-buffer
              (setq
               buffer-file-name
               single-source)
              (file-relative-name
               (ada-ts-mode--root-project-file)
               home)))
          (cl-letf
              (((symbol-function
                 'project-current)
                (lambda (&rest _)
                  'fixture-project))
               ((symbol-function
                 'project-root)
                (lambda (_project)
                  ambiguous-root)))
            (with-temp-buffer
              (setq
               buffer-file-name
               ambiguous-source)
              (ada-ts-mode--root-project-file)))))"##;
    let expect = expect![[
        r#"OK ("ada-project/default-root/default.gpr" "ada-project/single-root/only.gpr" nil)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_project_file_resolution_precedence_and_short_circuiting_match() {
    let elisp_form = r##"(cl-labels
         ((resolve
           (als alire root default)
           (let (events)
             (cl-letf
                 (((symbol-function
                    'ada-ts-als-project-file)
                   (lambda ()
                     (push
                      'als
                      events)
                     als))
                  ((symbol-function
                    'ada-ts-mode--alire-project-file)
                   (lambda ()
                     (push
                      'alire
                      events)
                     alire))
                  ((symbol-function
                    'ada-ts-mode--root-project-file)
                   (lambda ()
                     (push
                      'root
                      events)
                     root))
                  ((symbol-function
                    'ada-ts-mode--default-project-file)
                   (lambda ()
                     (push
                      'default
                      events)
                     default)))
               (list
                (ada-ts-mode--project-file)
                (nreverse
                 events))))))
         (list
          (resolve
           "als.gpr"
           "alire.gpr"
           "root.gpr"
           "default.gpr")
          (resolve
           nil
           "alire.gpr"
           "root.gpr"
           "default.gpr")
          (resolve
           nil
           nil
           "root.gpr"
           "default.gpr")
          (resolve
           nil
           nil
           nil
           "default.gpr")
          (resolve
           nil
           nil
           nil
           nil)))"##;
    let expect = expect![[
        r#"OK (("als.gpr" (als)) ("alire.gpr" (als alire)) ("root.gpr" (als alire root)) ("default.gpr" (als alire root default)) (nil (als alire root default)))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_project_and_other_file_commands_dispatch_and_report_fallbacks() {
    let elisp_form = r##"(progn
         (require
          'find-file)
         (let (events)
         (cl-letf
             (((symbol-function
                'find-file)
               (lambda (file)
                 (push
                  (list
                   'find-file
                   file)
                  events)
                 'visited))
              ((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (push
                  (apply
                   #'format
                   format-string
                   arguments)
                  events)
                 "message"))
              ((symbol-function
                'ff-find-other-file)
               (lambda ()
                 (push
                  'fallback-other-file
                  events)
                 'fallback)))
           (let ((found
                  (cl-letf
                      (((symbol-function
                         'ada-ts-mode--project-file)
                        (lambda ()
                          "fixture.gpr")))
                    (ada-ts-mode-find-project-file)))
                 (missing
                  (cl-letf
                      (((symbol-function
                         'ada-ts-mode--project-file)
                        (lambda ()
                          nil)))
                    (ada-ts-mode-find-project-file)))
                 (handled
                  (cl-letf
                      (((symbol-function
                         'ada-ts-als-other-file)
                        (lambda ()
                          (push
                           'als-other-file
                           events)
                          t)))
                    (ada-ts-mode-find-other-file)))
                 (fallback
                  (cl-letf
                      (((symbol-function
                         'ada-ts-als-other-file)
                        (lambda ()
                          (push
                           'als-other-file
                           events)
                          nil)))
                    (ada-ts-mode-find-other-file))))
             (list
              found
              missing
              handled
              fallback
              (nreverse
               events))))))"##;
    let expect = expect![[
        r#"OK (visited "message" nil fallback ((find-file "fixture.gpr") "Project file unknown or non-existent." als-other-file als-other-file fallback-other-file))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_user_workspace_uri_and_relative_project_paths_resolve_exactly() {
    let elisp_form = r##"(let* ((home
                 (file-name-as-directory
                  (getenv
                   "HOME")))
                (config-home
                 (expand-file-name
                  "xdg-config/"
                  home))
                (project-root
                 (expand-file-name
                  "ada-paths/project/"
                  home))
                (buffer-file-name
                 (expand-file-name
                  "src/main.adb"
                  project-root))
                (default-directory
                 (file-name-directory
                  buffer-file-name)))
         (make-directory
          default-directory
          t)
         (setenv
          "XDG_CONFIG_HOME"
          config-home)
         (cl-letf
             (((symbol-function
                'ada-ts-lspclient-current)
               (lambda ()
                 nil))
              ((symbol-function
                'project-current)
               (lambda (&rest _)
                 'fixture-project))
              ((symbol-function
                'project-root)
               (lambda (_project)
                 project-root))
              ((symbol-function
                'ada-ts-als--project-root)
               (lambda ()
                 project-root)))
           (mapcar
            (lambda (path)
              (and
               path
               (file-relative-name
                path
                home)))
            (list
             (ada-ts-als-user-config-file)
             (ada-ts-als-workspace-config-file)
             (ada-ts-als--project-file-absolute-path
              "config/project.gpr")
             (ada-ts-als--project-file-absolute-path
              (concat
               "file://"
               (expand-file-name
                "absolute.gpr"
                project-root)))
             (ada-ts-als--uri-to-path
              (concat
               "file://"
               (expand-file-name
                "source%20dir/"
                project-root)))))))"##;
    let expect = expect![[
        r#"OK ("xdg-config/als/config.json" "ada-paths/project/.als.json" "ada-paths/project/config/project.gpr" "ada-paths/project/absolute.gpr" "ada-paths/project/source dir")"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_project_file_resolver_covers_config_server_empty_and_unsupported_paths() {
    let elisp_form = r##"(let* ((home
                 (file-name-as-directory
                  (getenv
                   "HOME")))
                (root
                 (expand-file-name
                  "als-project/"
                  home)))
         (make-directory
          root
          t)
         (cl-labels
             ((resolve
               (config client supported result)
               (let (events)
                 (cl-letf
                     (((symbol-function
                        'ada-ts-als-composite-config)
                       (lambda (&rest _)
                         (push
                          'config
                          events)
                         config))
                      ((symbol-function
                        'ada-ts-lspclient-current)
                       (lambda ()
                         (push
                          'current
                          events)
                         client))
                      ((symbol-function
                        'ada-ts-lspclient-command-supported-p)
                       (lambda (actual-client command)
                         (push
                          (list
                           'supported
                           actual-client
                           command)
                          events)
                         supported))
                      ((symbol-function
                        'ada-ts-lspclient-command-execute)
                       (lambda (actual-client command &rest arguments)
                         (push
                          (list
                           'execute
                           actual-client
                           command
                           arguments)
                          events)
                         result))
                      ((symbol-function
                        'ada-ts-als--project-root)
                       (lambda ()
                         root)))
                   (let ((path
                          (ada-ts-als-project-file)))
                     (list
                      (and
                       path
                       (file-relative-name
                        path
                        home))
                      (nreverse
                       events)))))))
           (list
            (resolve
             '(:projectFile
               "configured.gpr")
             'client
             t
             "server.gpr")
            (resolve
             nil
             'client
             t
             "server.gpr")
            (resolve
             nil
             'client
             t
             "")
            (resolve
             nil
             'client
             nil
             "unreachable.gpr")
            (resolve
             nil
             nil
             t
             "unreachable.gpr"))))"##;
    let expect = expect![[
        r#"OK (("als-project/configured.gpr" (config)) ("als-project/server.gpr" (config current (supported client "als-project-file") (execute client "als-project-file" nil))) (nil (config current (supported client "als-project-file") (execute client "als-project-file" nil))) (nil (config current (supported client "als-project-file"))) (nil (config current)))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_alire_project_resolver_covers_explicit_project_crate_fallback_and_guards() {
    let elisp_form = r##"(let* ((home
                 (file-name-as-directory
                  (getenv
                   "HOME")))
                (root
                 (expand-file-name
                  "alire-project/"
                  home))
                (source
                 (expand-file-name
                  "src/main.adb"
                  root))
                (manifest
                 (expand-file-name
                  "alire.toml"
                  root)))
         (make-directory
          (file-name-directory
           source)
          t)
         (with-temp-file manifest
           (insert
            "name = \"fixture\"\n"))
         (cl-labels
             ((resolve
               (executable lines readable)
               (set-file-modes
                manifest
                (if readable
                    #o600
                  #o000))
               (let (events)
                 (cl-letf
                     (((symbol-function
                        'executable-find)
                       (lambda (program)
                         (push
                          (list
                           'executable-find
                           program)
                          events)
                         executable))
                      ((symbol-function
                        'process-lines)
                       (lambda (program &rest arguments)
                         (push
                          (list
                           'process-lines
                           program
                           arguments
                           default-directory)
                          events)
                         lines)))
                   (with-temp-buffer
                     (setq
                      buffer-file-name
                      source)
                     (let ((path
                            (ada-ts-mode--alire-project-file)))
                       (list
                        (and
                         path
                         (file-relative-name
                          path
                          home))
                        (mapcar
                         (lambda (event)
                           (if
                               (and
                                (consp event)
                                (eq
                                 (car event)
                                 'process-lines))
                               (list
                                (nth
                                 0
                                 event)
                                (nth
                                 1
                                 event)
                                (nth
                                 2
                                 event)
                                (file-relative-name
                                 (nth
                                  3
                                  event)
                                 home))
                             event))
                         (nreverse
                          events)))))))))
           (prog1
               (list
                (resolve
                 "/usr/bin/alr"
                 '("fixture=1.0.0"
                   "   Project_File: config/custom.gpr")
                 t)
                (resolve
                 "/usr/bin/alr"
                 '("crate_name=1.2.3")
                 t)
                (resolve
                 nil
                 '("unreachable=1")
                 t)
                (resolve
                 "/usr/bin/alr"
                 '("unreachable=1")
                 nil))
             (set-file-modes
              manifest
              #o600))))"##;
    let expect = expect![[
        r#"OK (("alire-project/config/custom.gpr" ((executable-find "alr") (process-lines "alr" ("--non-interactive" "--no-tty" "show") "alire-project/"))) ("alire-project/crate_name.gpr" ((executable-find "alr") (process-lines "alr" ("--non-interactive" "--no-tty" "show") "alire-project/"))) (nil ((executable-find "alr"))) (nil nil))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_client_workspace_paths_find_commands_and_session_setup_match() {
    let elisp_form = r##"(let* ((home
                 (file-name-as-directory
                  (getenv
                   "HOME")))
                (root
                 (expand-file-name
                  "client-workspace/"
                  home))
                (source-file
                 (expand-file-name
                  "src/main.adb"
                  root))
                events)
         (make-directory
          (file-name-directory
           source-file)
          t)
         (setenv
          "XDG_CONFIG_HOME"
          (expand-file-name
           "client-config/"
           home))
         (cl-letf
             (((symbol-function
                'ada-ts-lspclient-current)
               (lambda ()
                 'fixture-client))
              ((symbol-function
                'ada-ts-lspclient-workspace-root)
               (lambda (client path)
                 (push
                  (list
                   'workspace-root
                   client
                   (file-relative-name
                    path
                    home))
                  events)
                 root))
              ((symbol-function
                'ada-ts-als-source-dirs)
               (lambda ()
                 (list
                  (expand-file-name
                   "src/"
                   root)
                  (expand-file-name
                   "generated/"
                   root))))
              ((symbol-function
                'ada-ts-lspclient-workspace-dirs-add)
               (lambda (client directories)
                 (push
                  (list
                   'workspace-dirs-add
                   client
                   (mapcar
                    (lambda (directory)
                      (file-relative-name
                       directory
                       home))
                    directories))
                  events)
                 'added))
              ((symbol-function
                'find-file)
               (lambda (path)
                 (push
                  (list
                   'find-file
                   (file-relative-name
                    path
                    home))
                  events)
                 'visited)))
           (with-temp-buffer
             (setq
              buffer-file-name
              source-file
              major-mode
              'ada-ts-mode)
             (let ((workspace
                    (file-relative-name
                     (ada-ts-als-workspace-config-file)
                     home))
                   (user
                    (file-relative-name
                     (ada-ts-als-user-config-file)
                     home))
                   (find-workspace
                    (ada-ts-als-find-workspace-config-file))
                   (find-user
                    (ada-ts-als-find-user-config-file)))
               (ada-ts-als--lsp-session-setup)
               (list
                workspace
                user
                find-workspace
                find-user
                (nreverse
                 events))))))"##;
    let expect = expect![[
        r#"OK ("client-workspace/.als.json" "client-config/als/config.json" visited visited ((workspace-root fixture-client "client-workspace/src/main.adb") (workspace-root fixture-client "client-workspace/src/main.adb") (find-file "client-workspace/.als.json") (find-file "client-config/als/config.json") (workspace-dirs-add fixture-client ("client-workspace/src/" "client-workspace/generated/"))))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_show_composite_config_renders_json_read_only_view_buffer() {
    let elisp_form = r##"(let (displayed)
         (cl-letf
             (((symbol-function
                'ada-ts-als-composite-config)
               (lambda (&rest _)
                 '(:projectFile
                   "fixture.gpr"
                   :nested
                   (:enabled
                    t
                    :disabled
                    :json-false))))
              ((symbol-function
                'pop-to-buffer)
               (lambda (buffer &rest _)
                 (with-current-buffer buffer
                   (setq
                    displayed
                    (list
                     (buffer-name)
                     (buffer-substring-no-properties
                      (point-min)
                      (point-max))
                     buffer-read-only
                     view-mode
                     major-mode)))
                 buffer)))
           (ada-ts-als-show-composite-config)
           displayed))"##;
    let expect = expect![[
        r#"OK ("*ALS composite configuration*" "{\n  \"projectFile\": \"fixture.gpr\",\n  \"nested\": {\n    \"enabled\": true,\n    \"disabled\": false\n  }\n}" t t js-json-mode)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_als_remote_config_and_windows_uri_path_normalization_branches_match() {
    let elisp_form = r##"(let ((default-directory
                "/ssh:fixture:/workspace/")
               (original-expand-file-name
                (symbol-function
                 'expand-file-name)))
         (cl-letf
             (((symbol-function
                'file-remote-p)
               (lambda (&rest _)
                 "/ssh:fixture:"))
              ((symbol-function
                'expand-file-name)
               (lambda (name &optional directory)
                 (if
                     (string-prefix-p
                      "/ssh:fixture:"
                      name)
                     name
                   (funcall
                    original-expand-file-name
                    name
                    directory)))))
           (let ((relative
                  (progn
                    (setenv
                     "XDG_CONFIG_HOME"
                     "relative-config")
                    (ada-ts-als-user-config-file)))
                 (absolute
                  (progn
                    (setenv
                     "XDG_CONFIG_HOME"
                     "/remote/config")
                    (ada-ts-als-user-config-file))))
             (append
              (list
               relative
               absolute)
              (let ((system-type
                     'windows-nt))
                (cl-letf
                    (((symbol-function
                       'w32-convert-standard-filename)
                      #'identity)
                     ((symbol-function
                       'ada-ts-als--project-root)
                      (lambda ()
                        "C:/Workspace/")))
                  (list
                   (ada-ts-als--uri-to-path
                    "file:///C:/Workspace/source%20dir/")
                   (ada-ts-als--project-file-absolute-path
                    "file:///C:/Workspace/project.gpr"))))))))"##;
    let expect = expect![[
        r#"OK ("/ssh:fixture:~/.config/als/config.json" "/ssh:fixture:[ORACLE-XDG-CONFIG]/als/config.json" "C:/Workspace/source dir" "/ssh:fixture:/workspace/C:/Workspace/C:/Workspace/project.gpr")"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}
