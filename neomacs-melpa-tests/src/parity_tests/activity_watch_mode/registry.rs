use expect_test::expect;

use super::{assert_activity_watch_mode_autoload_parity, assert_activity_watch_mode_parity};

#[test]
fn activity_watch_mode_exact_pin_metadata_versions_features_group_and_dependencies_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq
                  'activity-watch-mode
                  package-alist))))
         (list
          (package-desc-name
           descriptor)
          (package-version-join
           (package-desc-version
            descriptor))
          (package-desc-reqs
           descriptor)
          (package-desc-summary
           descriptor)
          (copy-tree
           (package-desc-extras
            descriptor))
          activity-watch-version
          activity-watch-user-agent
          (featurep
           'activity-watch-mode)
          (mapcar
           #'featurep
           '(ert
             request
             json
             cl-lib
             subr-x))
          (get
           'activity-watch
           'group-documentation)
          (copy-tree
           (get
            'activity-watch
            'custom-group))))"##;
    let expect = expect![[
        r#"OK (activity-watch-mode "20260311.835" ((emacs (25)) (request (0)) (json (0)) (cl-lib (0))) "Automatic time tracking extension." ((:maintainers ("Paul d'Hubert" . "paul.dhubert@ya.ru")) (:authors ("Gabor Torok" . "gabor@20y.hu") ("Alan Hamlett" . "alan@wakatime.com")) (:keywords "calendar" "comm") (:revdesc . "1a950ad310cb") (:commit . "1a950ad310cbd0511bb01744b20c7012a1c0b0e8") (:url . "https://github.com/pauldub/activity-watch-mode")) "1.0.0" "emacs-activity-watch" t (t t t t t) "Customizations for Activity-Watch" ((activity-watch-api-host custom-variable) (activity-watch-project-name-default custom-variable) (activity-watch-org-clock-active custom-variable) (activity-watch-org-clock-property custom-variable) (activity-watch-project-name-resolvers custom-variable) (global-activity-watch-mode custom-variable)))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_explicit_variable_values_docs_locality_and_sources_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (boundp symbol)
            (default-boundp symbol)
            (default-value symbol)
            (local-variable-if-set-p
             symbol)
            (documentation-property
             symbol
             'variable-documentation
             t)
            (let ((file
                   (symbol-file
                    symbol
                    'defvar)))
              (and file
                   (file-name-nondirectory
                    file)))))
         '(activity-watch-version
           activity-watch-user-agent
           activity-watch-noprompt
           activity-watch-timer
           activity-watch-idle-timer
           activity-watch-init-started
           activity-watch-init-finished
           activity-watch-bucket-created
           activity-watch-last-file-path
           activity-watch-pulse-time
           activity-watch-max-heartbeat-per-sec
           activity-watch-last-heartbeat-time
           activity-watch-project-name))"##;
    let expect = expect![[
        r#"OK ((activity-watch-version t t "1.0.0" nil nil "activity-watch-mode.el") (activity-watch-user-agent t t "emacs-activity-watch" nil nil "activity-watch-mode.el") (activity-watch-noprompt t t nil nil nil "activity-watch-mode.el") (activity-watch-timer t t nil nil nil "activity-watch-mode.el") (activity-watch-idle-timer t t nil nil nil "activity-watch-mode.el") (activity-watch-init-started t t nil nil nil "activity-watch-mode.el") (activity-watch-init-finished t t nil nil nil "activity-watch-mode.el") (activity-watch-bucket-created t t nil nil nil "activity-watch-mode.el") (activity-watch-last-file-path t t nil nil nil "activity-watch-mode.el") (activity-watch-pulse-time t t 30 nil nil "activity-watch-mode.el") (activity-watch-max-heartbeat-per-sec t t 1 nil nil "activity-watch-mode.el") (activity-watch-last-heartbeat-time t t nil nil nil "activity-watch-mode.el") (activity-watch-project-name t t nil t "Cached value of the project this file belongs to" "activity-watch-mode.el"))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_custom_options_defaults_types_docs_and_standard_forms_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((standard
                  (get
                   symbol
                   'standard-value)))
             (list
              symbol
              (default-value
               symbol)
              (and standard
                   (eval
                    (car standard)
                    t))
              (get
               symbol
               'custom-type)
              (documentation-property
               symbol
               'variable-documentation
               t)
              (let ((file
                     (symbol-file
                      symbol
                      'defvar)))
                (and file
                     (file-name-nondirectory
                      file))))))
         '(activity-watch-api-host
           activity-watch-project-name-default
           activity-watch-org-clock-active
           activity-watch-org-clock-property
           activity-watch-project-name-resolvers))"##;
    let expect = expect![[
        r#"OK ((activity-watch-api-host "http://localhost:5600" "http://localhost:5600" string "API host for Activity-Watch." "activity-watch-mode.el") (activity-watch-project-name-default "unknown" "unknown" string "Default name for a non-identifiable project." "activity-watch-mode.el") (activity-watch-org-clock-active nil nil boolean "When non-nil, inject the active Org clock property into the payload.\nThis allows ActivityWatch to track time spent on specific Org tasks." "activity-watch-mode.el") (activity-watch-org-clock-property "TICKET_ID" "TICKET_ID" string "The Org mode property to extract when the clock is active.\nThe property name will be converted to lowercase and used as the JSON key." "activity-watch-mode.el") (activity-watch-project-name-resolvers #1=(projectile project magit-dir-force magit-origin) #1# (list symbol) "List of resolvers used to find the project name.\n\nWhen determining the name of a project, the watcher will go down the list\nand for each name tries to call the function `activity-watch-project-name-<symbol>' with no parameters.\nIf the function returns a non-emtpy string, it will be used as the project name.\nOtherwise, the following resolver in the list will be queried.\n\nIf no resolver is able to identify the project, `activity-watch-project-name-default' is assumed.\n\nMethods provided by default are listed below.\nEvery resolver that depends on an external package has a -force version.\nThe default resolver checks if the package is loaded, and fails early if not.\nThe forced resolver tries to `require' the package.\n\nprojectile:\nprojectile-force:\n  Return the project name from `projectile-project-name'.\n\nmagit-dir:\nmagit-dir-force:\n  Return the name of the directory where the repository is located.\n\nmagit-origin:\nmagit-origin-force:\n  Return the name of the repository extracted from the 'origin' remote.\n\ncwd:\n  Return the name of the current working directory." "activity-watch-mode.el"))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_complete_macro_callable_and_command_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((doc
                  (documentation
                   symbol
                   t)))
             (list
              symbol
              (fboundp symbol)
              (and
               (fboundp symbol)
               (macrop
                (symbol-function
                 symbol)))
              (help-function-arglist
               symbol
               t)
              (commandp symbol)
              (interactive-form
               symbol)
              (and doc
                   (car
                    (split-string
                     doc
                     "\n")))
              (let ((file
                     (symbol-file
                      symbol
                      'defun)))
                (and file
                     (file-name-nondirectory
                      file))))))
         '(activity-watch--gen-feature-resolver
           activity-watch-project-name-project
           activity-watch-project-name-project-force
           activity-watch-project-name-projectile
           activity-watch-project-name-projectile-force
           activity-watch-project-name-magit-dir
           activity-watch-project-name-magit-dir-force
           activity-watch-project-name-magit-origin
           activity-watch-project-name-magit-origin-force
           activity-watch--inject-org-property
           activity-watch-project-name-cwd
           activity-watch--get-project
           activity-watch--s-blank
           activity-watch--init
           activity-watch--bucket-id
           activity-watch--create-bucket
           activity-watch--create-heartbeat
           activity-watch--send-heartbeat
           activity-watch--call
           activity-watch--save
           activity-watch--start-timer
           activity-watch--stop-timer
           activity-watch--stop-idle-timer
           activity-watch--bind-hooks
           activity-watch--unbind-hooks
           activity-watch-turn-on
           activity-watch-turn-off
           activity-watch-refresh-project-name
           activity-watch-mode
           activity-watch-mode--set-explicitly
           global-activity-watch-mode
           global-activity-watch-mode-enable-in-buffer))"##;
    let expect = expect![[
        r#"OK ((activity-watch--gen-feature-resolver t t (feature name &rest body) nil nil "Generate a pair of functions: `activity-watch-project-name-<NAME>' and `activity-watch-project-name-<NAME>-force'. The forced version will try to `require' FEATURE first." "activity-watch-mode.el") (activity-watch-project-name-project t nil nil nil nil "Check if feature `project' is provided, and when it is, use it to find the project's name." "activity-watch-mode.el") (activity-watch-project-name-project-force t nil nil nil nil "Try to require feature `project', and on success use it to find the project's name." "activity-watch-mode.el") (activity-watch-project-name-projectile t nil nil nil nil "Check if feature `projectile' is provided, and when it is, use it to find the project's name." "activity-watch-mode.el") (activity-watch-project-name-projectile-force t nil nil nil nil "Try to require feature `projectile', and on success use it to find the project's name." "activity-watch-mode.el") (activity-watch-project-name-magit-dir t nil nil nil nil "Check if feature `magit' is provided, and when it is, use it to find the project's name." "activity-watch-mode.el") (activity-watch-project-name-magit-dir-force t nil nil nil nil "Try to require feature `magit', and on success use it to find the project's name." "activity-watch-mode.el") (activity-watch-project-name-magit-origin t nil nil nil nil "Check if feature `magit' is provided, and when it is, use it to find the project's name." "activity-watch-mode.el") (activity-watch-project-name-magit-origin-force t nil nil nil nil "Try to require feature `magit', and on success use it to find the project's name." "activity-watch-mode.el") (activity-watch--inject-org-property t nil (heartbeat) nil nil "Inject the active Org clock property into the ActivityWatch HEARTBEAT payload." "activity-watch-mode.el") (activity-watch-project-name-cwd t nil nil nil nil "Return the name of the `default-directory'." "activity-watch-mode.el") (activity-watch--get-project t nil (&optional refresh) nil nil "Return the name of the project. If REFRESH is non-nil, disable cache." "activity-watch-mode.el") (activity-watch--s-blank t nil (string) nil nil "Return non-nil if the STRING is empty or nil.  Expects string." "activity-watch-mode.el") (activity-watch--init t nil nil nil nil "Initialize symbol ‘activity-watch-mode’." "activity-watch-mode.el") (activity-watch--bucket-id t nil nil nil nil "Return the bucket-id to be used when submitting heartbeats." "activity-watch-mode.el") (activity-watch--create-bucket t nil nil nil nil "Create the editor bucket." "activity-watch-mode.el") (activity-watch--create-heartbeat t nil (time) nil nil "Create heartbeart to sent to the activity watch server." "activity-watch-mode.el") (activity-watch--send-heartbeat t nil (heartbeat &rest --cl-rest--) nil nil "Send HEARTBEAT to activity watch server, calling ON-ERROR on error and ON-SUCCESS on success." "activity-watch-mode.el") (activity-watch--call t nil nil nil nil "Conditionally submit heartbeat to activity watch." "activity-watch-mode.el") (activity-watch--save t nil nil nil nil "Send save notice to Activity-Watch." "activity-watch-mode.el") (activity-watch--start-timer t nil nil nil nil "Start timers for heartbeat submission and idling." "activity-watch-mode.el") (activity-watch--stop-timer t nil nil nil nil "Stop heartbeat submission timer." "activity-watch-mode.el") (activity-watch--stop-idle-timer t nil nil nil nil "Stop idling timer." "activity-watch-mode.el") (activity-watch--bind-hooks t nil nil nil nil "Watch for activity in buffers." "activity-watch-mode.el") (activity-watch--unbind-hooks t nil nil nil nil "Stop watching for activity in buffers." "activity-watch-mode.el") (activity-watch-turn-on t nil (defer) nil nil "Turn on Activity-Watch." "activity-watch-mode.el") (activity-watch-turn-off t nil nil nil nil "Turn off Activity-Watch." "activity-watch-mode.el") (activity-watch-refresh-project-name t nil nil t (interactive nil) "Recompute the name of the project for the current file." "activity-watch-mode.el") (activity-watch-mode t nil (&optional arg) t (interactive #1=(list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Toggle Activity-Watch (Activity-Watch mode)." "activity-watch-mode.el") (activity-watch-mode--set-explicitly t nil nil nil nil nil "activity-watch-mode.el") (global-activity-watch-mode t nil (&optional arg) t (interactive #1#) "Toggle Activity-Watch mode in many buffers." "activity-watch-mode.el") (global-activity-watch-mode-enable-in-buffer t nil nil nil nil nil "activity-watch-mode.el"))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_generated_minor_mode_variables_lighters_hooks_and_sources_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (boundp symbol)
             (default-boundp symbol)
             (and
              (boundp symbol)
              (default-value symbol))
             (local-variable-if-set-p
              symbol)
             (documentation-property
              symbol
              'variable-documentation
              t)
             (let ((file
                    (symbol-file
                     symbol
                     'defvar)))
               (and file
                    (file-name-nondirectory
                     file)))))
          '(activity-watch-mode
            activity-watch-mode-hook
            activity-watch-mode-map
            activity-watch-mode--set-explicitly
            activity-watch-mode--suppress-set-explicitly
            global-activity-watch-mode
            global-activity-watch-mode-hook))
         (assq
          'activity-watch-mode
          minor-mode-alist)
         (assq
          'activity-watch-mode
          minor-mode-map-alist)
         (get
          'global-activity-watch-mode
          'globalized-minor-mode)
         (get
          'global-activity-watch-mode
          'custom-type))"##;
    let expect = expect![[
        r#"OK (((activity-watch-mode t t nil t "Non-nil if activity-watch mode is enabled.\nUse the command `activity-watch-mode' to change this variable." "activity-watch-mode.el") (activity-watch-mode-hook t t (activity-watch-mode--set-explicitly) nil "Hook run after entering or leaving `activity-watch-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "activity-watch-mode.el") (activity-watch-mode-map nil nil nil nil nil nil) (activity-watch-mode--set-explicitly t t nil t nil "activity-watch-mode.el") (activity-watch-mode--suppress-set-explicitly t t nil nil nil "activity-watch-mode.el") (global-activity-watch-mode t t nil nil "Non-nil if Global Activity-Watch mode is enabled.\nSee the `global-activity-watch-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `global-activity-watch-mode'." "activity-watch-mode.el") (global-activity-watch-mode-hook t t nil nil "Hook run after entering or leaving `global-activity-watch-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "activity-watch-mode.el")) (activity-watch-mode " activity-watch") nil t boolean)"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_installed_package_inventory_and_content_assets_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'activity-watch-mode
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor))
                 (names
                  (sort
                   (directory-files
                    directory
                    nil
                    "^[^.].*")
                   #'string<)))
         (list
          names
          (mapcar
           (lambda (name)
             (let ((path
                    (expand-file-name
                     name
                     directory)))
               (list
                name
                (file-regular-p path)
                (if
                    (string-suffix-p
                     ".elc"
                     name)
                    t
                  (with-temp-buffer
                    (insert-file-contents-literally
                     path)
                    (list
                     (buffer-size)
                     (secure-hash
                      'sha256
                      (current-buffer))))))))
           names)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" "activity-watch-mode-autoloads.el" "activity-watch-mode-pkg.el" "activity-watch-mode.el" "activity-watch-mode.elc") (("README-elpa" t (323 "baa8564eb53319ed16b35c89b5909c61e6c78b62b458d69a15f6749ce6acfbb8")) ("activity-watch-mode-autoloads.el" t (2607 "3be6b83cbaca8e93c3655a08ca3ea1257872360bd4520ed3eb81ac4f0ad9d1a7")) ("activity-watch-mode-pkg.el" t (545 "2f471411ad01b1f2f792d19970844050b6b8ef0a81771f2b33d4ff974bc85138")) ("activity-watch-mode.el" t (15349 "e8ccbcbf497cf0e4dd710da32bc1239e312a08993c90f0aca6553d4b3365f369")) ("activity-watch-mode.elc" t t)))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_generated_autoload_surface_does_not_load_runtime_until_introspection() {
    let elisp_form = r##"(let ((before
                (list
                 (featurep
                  'activity-watch-mode)
                 (featurep
                  'activity-watch-mode-autoloads)
                 (mapcar
                  (lambda (symbol)
                    (list
                     symbol
                     (fboundp symbol)
                     (autoloadp
                      (symbol-function
                       symbol))))
                  '(activity-watch-refresh-project-name
                    activity-watch-mode
                    global-activity-watch-mode))
                 (boundp
                  'activity-watch-version))))
         (list
          before
          (mapcar
           (lambda (symbol)
             (list
              symbol
              (commandp symbol)
              (interactive-form
               symbol)
              (help-function-arglist
               symbol
               t)
              (let ((file
                     (symbol-file
                      symbol
                      'defun)))
                (and file
                     (file-name-nondirectory
                      file)))))
           '(activity-watch-refresh-project-name
             activity-watch-mode
             global-activity-watch-mode))
          (list
           (featurep
            'activity-watch-mode)
           (boundp
            'activity-watch-version)
           (autoloadp
            (symbol-function
             'activity-watch-mode)))))"##;
    let expect = expect![[
        r#"OK ((nil t ((activity-watch-refresh-project-name t t) (activity-watch-mode t t) (global-activity-watch-mode t t)) nil) ((activity-watch-refresh-project-name t (interactive nil) nil "activity-watch-mode.el") (activity-watch-mode t (interactive #1=(list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) (&optional arg) "activity-watch-mode.el") (global-activity-watch-mode t (interactive #1#) (&optional arg) "activity-watch-mode.el")) (t t nil))"#
    ]];
    assert_activity_watch_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_direct_reload_preserves_defvars_custom_values_and_mode_registrations() {
    let elisp_form = r##"(let ((original-version
                activity-watch-version)
               (original-user-agent
                activity-watch-user-agent)
               (original-pulse
                activity-watch-pulse-time)
               (original-host
                activity-watch-api-host))
         (unwind-protect
             (progn
               (setq activity-watch-version
                     "sentinel-version"
                     activity-watch-user-agent
                     "sentinel-agent"
                     activity-watch-pulse-time
                     77
                     activity-watch-api-host
                     "https://sentinel.invalid")
               (load
                (getenv
                 "NEOMACS_PACKAGE_SOURCE")
                nil t t)
               (list
                activity-watch-version
                activity-watch-user-agent
                activity-watch-pulse-time
                activity-watch-api-host
                (boundp
                 'activity-watch-mode-map)
                (length
                 (seq-filter
                  (lambda (entry)
                    (eq
                     (car-safe entry)
                     'activity-watch-mode))
                  minor-mode-alist))))
           (setq activity-watch-version
                 original-version
                 activity-watch-user-agent
                 original-user-agent
                 activity-watch-pulse-time
                 original-pulse
                 activity-watch-api-host
                 original-host)))"##;
    let expect =
        expect![[r#"OK ("1.0.0" "emacs-activity-watch" 77 "https://sentinel.invalid" nil 1)"#]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}
