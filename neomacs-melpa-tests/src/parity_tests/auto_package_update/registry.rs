use expect_test::expect;

use super::{assert_auto_package_update_autoload_parity, assert_auto_package_update_parity};

#[test]
fn auto_package_update_exact_descriptor_dependencies_and_archive_payload_match() {
    let elisp_form = r##"(let*
                             ((descriptor
                               (cadr
                                (assq
                                 'auto-package-update
                                 package-alist)))
                              (directory
                               (package-desc-dir descriptor)))
                           (list
                            (package-desc-name descriptor)
                            (package-version-join
                             (package-desc-version descriptor))
                            (package-desc-summary descriptor)
                            (package-desc-reqs descriptor)
                            (package-desc-kind descriptor)
                            (package-desc-extras descriptor)
                            (mapcar
                             (lambda (name)
                               (let ((file
                                      (expand-file-name
                                       name
                                       directory)))
                                 (list
                                  name
                                  (file-attribute-size
                                   (file-attributes file))
                                  (with-temp-buffer
                                    (set-buffer-multibyte nil)
                                    (insert-file-contents-literally file)
                                    (secure-hash
                                     'sha256
                                     (current-buffer))))))
                             '("auto-package-update-pkg.el"
                               "auto-package-update.el"))
                            (let ((dash
                                   (cadr
                                    (assq
                                     'dash
                                     package-alist))))
                              (list
                               (package-desc-name dash)
                               (package-version-join
                                (package-desc-version dash))
                               (package-desc-reqs dash)))))"##;
    let expect = expect![[
        r#"OK (auto-package-update "20260601.1804" "Automatically update Emacs packages." ((emacs (24 4)) (dash (2 1 0))) nil ((:keywords "package" "update") (:revdesc . "e966c6c95de1") (:commit . "e966c6c95de1742d867250dc15b1c6bd570b6ea5") (:url . "https://github.com/rranelli/auto-package-update.el")) (("auto-package-update-pkg.el" 361 "137a90e8c3931ce94db0eb3a5880d756566cd5fc84db75cba0323b4e0934fc2d") ("auto-package-update.el" 15624 "bfdf1377656ce5d47445734eafd4db1353e87816110a5e7a0a4e78691c012745")) (dash "20260221.1346" ((emacs (24)))))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_complete_source_owned_symbol_inventory_matches() {
    let elisp_form = r##"(let (symbols)
                           (mapatoms
                            (lambda (symbol)
                              (let
                                  ((origin
                                    (or
                                     (symbol-file symbol 'defun)
                                     (symbol-file symbol 'defvar))))
                                (when
                                    (and
                                     origin
                                     (string=
                                      (file-name-base origin)
                                      "auto-package-update"))
                                  (push
                                   (list
                                    symbol
                                    (fboundp symbol)
                                    (boundp symbol)
                                    (and (commandp symbol) t)
                                    (local-variable-if-set-p symbol))
                                   symbols)))))
                           (sort
                            symbols
                            (lambda (left right)
                              (string<
                               (symbol-name (car left))
                               (symbol-name (car right))))))"##;
    let expect = expect![
        "OK ((apu--add-to-old-versions-dirs-list t nil nil nil) (apu--delete-old-versions-dirs-list t nil nil nil) (apu--filter-quelpa-packages t nil nil nil) (apu--get-permission-to-update-p t nil nil nil) (apu--hide-preview t nil nil nil) (apu--old-versions-dirs-list nil t nil nil) (apu--package-out-of-date-p t nil nil nil) (apu--package-up-to-date-p t nil nil nil) (apu--packages-to-install t nil nil nil) (apu--read-file-as-string t nil nil nil) (apu--read-last-update-day t nil nil nil) (apu--safe-install-packages t nil nil nil) (apu--safe-package-install t nil nil nil) (apu--should-update-packages-p t nil nil nil) (apu--show-preview t nil nil nil) (apu--today-day t nil nil nil) (apu--update-thread nil t nil nil) (apu--write-buffer t nil nil nil) (apu--write-current-day t nil nil nil) (apu--write-preview-buffer t nil nil nil) (apu--write-results-buffer t nil nil nil) (apu--write-string-to-file t nil nil nil) (auto-package-preview-buffer-name nil t nil nil) (auto-package-update-after-hook nil t nil nil) (auto-package-update-at-time t nil nil nil) (auto-package-update-before-hook nil t nil nil) (auto-package-update-buffer-name nil t nil nil) (auto-package-update-delete-old-versions nil t nil nil) (auto-package-update-excluded-packages nil t nil nil) (auto-package-update-hide-results nil t nil nil) (auto-package-update-interval nil t nil nil) (auto-package-update-last-update-day-filename nil t nil nil) (auto-package-update-last-update-day-path nil t nil nil) (auto-package-update-maybe t nil nil nil) (auto-package-update-minor-mode t t t t) (auto-package-update-minor-mode-hook nil t nil nil) (auto-package-update-minor-mode-map nil t nil nil) (auto-package-update-now t nil t nil) (auto-package-update-now-async t nil t nil) (auto-package-update-prompt-before-update nil t nil nil) (auto-package-update-show-preview nil t nil nil))"
    ];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_callable_signatures_docs_interactivity_and_origins_match() {
    let elisp_form = r##"(mapcar
                           (lambda (symbol)
                             (list
                              symbol
                              (help-function-arglist symbol t)
                              (and (interactive-form symbol) t)
                              (and (commandp symbol) t)
                              (documentation symbol t)
                              (file-name-nondirectory
                               (or
                                (symbol-file symbol 'defun)
                                ""))))
                           '(apu--read-file-as-string
                             apu--write-string-to-file
                             apu--today-day
                             apu--write-current-day
                             apu--read-last-update-day
                             apu--should-update-packages-p
                             apu--get-permission-to-update-p
                             apu--package-up-to-date-p
                             apu--package-out-of-date-p
                             apu--packages-to-install
                             apu--add-to-old-versions-dirs-list
                             apu--delete-old-versions-dirs-list
                             apu--safe-package-install
                             apu--safe-install-packages
                             apu--write-buffer
                             apu--write-results-buffer
                             apu--write-preview-buffer
                             apu--filter-quelpa-packages
                             apu--show-preview
                             apu--hide-preview
                             auto-package-update-now
                             auto-package-update-now-async
                             auto-package-update-at-time
                             auto-package-update-maybe
                             auto-package-update-minor-mode))"##;
    let expect = expect![[
        r#"OK ((apu--read-file-as-string (file) nil nil "Read FILE contents." "auto-package-update.el") (apu--write-string-to-file (file string) nil nil "Substitute FILE contents with STRING." "auto-package-update.el") (apu--today-day nil nil nil nil "auto-package-update.el") (apu--write-current-day nil nil nil "Store current day." "auto-package-update.el") (apu--read-last-update-day nil nil nil "Read last update day." "auto-package-update.el") (apu--should-update-packages-p nil nil nil "Return non-nil when an update is due." "auto-package-update.el") (apu--get-permission-to-update-p nil nil nil "(Optionally) Prompt permission to perform update and display preview" "auto-package-update.el") (apu--package-up-to-date-p (package) nil nil nil "auto-package-update.el") (apu--package-out-of-date-p (package) nil nil nil "auto-package-update.el") (apu--packages-to-install nil nil nil nil "auto-package-update.el") (apu--add-to-old-versions-dirs-list (package) nil nil "Add package old version dir to apu--old-versions-dirs-list" "auto-package-update.el") (apu--delete-old-versions-dirs-list nil nil nil "Delete package old version dirs saved in variable apu--old-versions-dirs-list" "auto-package-update.el") (apu--safe-package-install (package) nil nil nil "auto-package-update.el") (apu--safe-install-packages (packages) nil nil nil "auto-package-update.el") (apu--write-buffer (contents buffer-name &optional hide-buffer) nil nil nil "auto-package-update.el") (apu--write-results-buffer (contents) nil nil nil "auto-package-update.el") (apu--write-preview-buffer (contents) nil nil nil "auto-package-update.el") (apu--filter-quelpa-packages (package-list) nil nil "Return PACKAGE-LIST without quelpa packages." "auto-package-update.el") (apu--show-preview nil nil nil nil "auto-package-update.el") (apu--hide-preview nil nil nil nil "auto-package-update.el") (auto-package-update-now (&optional async) t t "Update installed Emacs packages." "auto-package-update.el") (auto-package-update-now-async (&optional force) t t "Update installed Emacs packages with an async manner.\nIf FORCE is non-nil, kill the update thread anyway." "auto-package-update.el") (auto-package-update-at-time (time) nil nil "Try to update every day at the specified TIME." "auto-package-update.el") (auto-package-update-maybe nil nil nil "Update installed Emacs packages if at least\n`auto-package-update-interval' days have passed since the last\nupdate." "auto-package-update.el") (auto-package-update-minor-mode (&optional arg) t t "Minor mode for displaying package update results.\n\nThis is a minor mode.  If called interactively, toggle the\n`Auto-Package-Update minor mode' mode.  If the prefix argument is\npositive, enable the mode, and if it is zero or negative, disable the\nmode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `auto-package-update-minor-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled." "auto-package-update.el"))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_options_have_exact_defaults_docs_types_groups_and_locality() {
    let elisp_form = r##"(mapcar
                           (lambda (symbol)
                             (list
                              symbol
                              (if
                                  (boundp symbol)
                                  (if
                                      (eq
                                       symbol
                                       'auto-package-update-last-update-day-path)
                                      (file-name-nondirectory
                                       (default-value symbol))
                                    (default-value symbol))
                                :unbound)
                              (documentation-property
                               symbol
                               'variable-documentation
                               t)
                              (get symbol 'custom-type)
                              (get symbol 'custom-group)
                              (local-variable-if-set-p symbol)
                              (file-name-nondirectory
                               (or
                                (symbol-file symbol 'defvar)
                                ""))))
                           '(auto-package-update-interval
                             auto-package-update-before-hook
                             auto-package-update-after-hook
                             auto-package-update-last-update-day-filename
                             auto-package-update-buffer-name
                             auto-package-preview-buffer-name
                             auto-package-update-delete-old-versions
                             auto-package-update-prompt-before-update
                             auto-package-update-show-preview
                             auto-package-update-hide-results
                             auto-package-update-excluded-packages
                             auto-package-update-last-update-day-path
                             apu--old-versions-dirs-list
                             apu--update-thread
                             quelpa-cache))"##;
    let expect = expect![[
        r#"OK ((auto-package-update-interval 7 "Interval in DAYS for automatic package update." integer nil nil "auto-package-update.el") (auto-package-update-before-hook nil "List of functions to be called before running an automatic package update." hook nil nil "auto-package-update.el") (auto-package-update-after-hook nil "List of functions to be called after running an automatic package update." hook nil nil "auto-package-update.el") (auto-package-update-last-update-day-filename ".last-package-update-day" "Name of the file in which the last update day is going to be stored." string nil nil "auto-package-update.el") (auto-package-update-buffer-name "*package update results*" "Name of the buffer that shows updated packages and error after execution." string nil nil "auto-package-update.el") (auto-package-preview-buffer-name "*package update preview*" "Name of the buffer that shows a preview of the packages to be updated." string nil nil "auto-package-update.el") (auto-package-update-delete-old-versions nil "If not nil, delete old versions directories." boolean nil nil "auto-package-update.el") (auto-package-update-prompt-before-update nil "Prompt user (y/n) before running auto-package-update-maybe" boolean nil nil "auto-package-update.el") (auto-package-update-show-preview nil "If not nil, show the list of packages to be updated when\nprompting before running auto-package-update-maybe" boolean nil nil "auto-package-update.el") (auto-package-update-hide-results nil "If not nil, the result of auto package update in buffer\n`auto-package-update-buffer-name' will not be shown." boolean nil nil "auto-package-update.el") (auto-package-update-excluded-packages nil "List of packages to exclude from automatic package update." (repeat symbol) nil nil "auto-package-update.el") (auto-package-update-last-update-day-path ".last-package-update-day" "Path to the file that will hold the day in which the last update was run." nil nil nil "auto-package-update.el") (apu--old-versions-dirs-list nil "List with old versions directories to delete." nil nil nil "auto-package-update.el") (apu--update-thread nil "The update thread." nil nil nil "auto-package-update.el") (quelpa-cache :unbound nil nil nil nil ""))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_feature_requirements_group_and_load_history_match() {
    let elisp_form = r##"(let*
                             ((history
                               (seq-find
                                (lambda (entry)
                                  (and
                                   (stringp (car entry))
                                   (string=
                                    (file-name-base
                                     (car entry))
                                    "auto-package-update")))
                                load-history))
                              (events
                               (seq-filter
                                (lambda (event)
                                  (memq
                                   (car-safe event)
                                   '(require defun provide)))
                                (cdr history))))
                           (list
                            (featurep 'auto-package-update)
                            (featurep 'dash)
                            (featurep 'package)
                            package--initialized
                            (get 'auto-package-update
                                 'group-documentation)
                            events))"##;
    let expect = expect![[
        r#"OK (t t t t "Automatically update Emacs packages." ((require . dash) (require . cl-lib) (require . package) (defun . apu--read-file-as-string) (defun . apu--write-string-to-file) (defun . apu--today-day) (defun . apu--write-current-day) (defun . apu--read-last-update-day) (defun . apu--should-update-packages-p) (defun . apu--get-permission-to-update-p) (defun . apu--package-up-to-date-p) (defun . apu--package-out-of-date-p) (defun . apu--packages-to-install) (defun . apu--add-to-old-versions-dirs-list) (defun . apu--delete-old-versions-dirs-list) (defun . apu--safe-package-install) (defun . apu--safe-install-packages) (defun . apu--write-buffer) (defun . apu--write-results-buffer) (defun . apu--write-preview-buffer) (defun . auto-package-update-minor-mode) (defun . apu--filter-quelpa-packages) (defun . apu--show-preview) (defun . apu--hide-preview) (defun . auto-package-update-now) (defun . auto-package-update-now-async) (defun . auto-package-update-at-time) (defun . auto-package-update-maybe) (provide . auto-package-update)))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_minor_mode_has_exact_buffer_local_keymap_lifecycle() {
    let elisp_form = r##"(with-temp-buffer
                           (let ((before
                                  (list
                                   auto-package-update-minor-mode
                                   (local-variable-p
                                    'auto-package-update-minor-mode)
                                   (key-binding (kbd "q")))))
                             (auto-package-update-minor-mode 1)
                             (let ((enabled
                                    (list
                                     auto-package-update-minor-mode
                                     (local-variable-p
                                      'auto-package-update-minor-mode)
                                     (key-binding (kbd "q"))
                                     (commandp
                                      (key-binding (kbd "q"))))))
                               (auto-package-update-minor-mode -1)
                               (list
                                before
                                enabled
                                auto-package-update-minor-mode
                                (key-binding (kbd "q"))
                                (get
                                 'auto-package-update-minor-mode
                                 'custom-type)))))"##;
    let expect = expect![
        "OK ((nil nil self-insert-command) (t t quit-window t) nil self-insert-command nil)"
    ];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_generated_autoload_exposes_only_public_commands() {
    let elisp_form = r##"(let*
                             ((history
                               (seq-find
                                (lambda (entry)
                                  (and
                                   (stringp (car entry))
                                   (string-suffix-p
                                    "auto-package-update-autoloads.el"
                                    (car entry))))
                                load-history))
                              (symbols
                               '(auto-package-update-now
                                 auto-package-update-now-async
                                 auto-package-update-at-time
                                 auto-package-update-maybe)))
                           (list
                            (featurep
                             'auto-package-update-autoloads)
                            (featurep 'auto-package-update)
                            (mapcar
                             (lambda (symbol)
                               (list
                                symbol
                                (fboundp symbol)
                                (autoloadp
                                 (symbol-function symbol))
                                (and (commandp symbol) t)
                                (file-name-nondirectory
                                 (or
                                  (symbol-file symbol 'defun)
                                  ""))))
                             symbols)
                            (seq-filter
                             (lambda (event)
                               (memq
                                (car-safe event)
                                '(defun provide)))
                             (cdr history))))"##;
    let expect = expect![[
        r#"OK (t nil ((auto-package-update-now t t t "auto-package-update.el") (auto-package-update-now-async t t t "auto-package-update.el") (auto-package-update-at-time t t nil "auto-package-update.el") (auto-package-update-maybe t t nil "auto-package-update.el")) ((defun . auto-package-update-now) (defun . auto-package-update-now-async) (defun . auto-package-update-at-time) (defun . auto-package-update-maybe) (provide . auto-package-update-autoloads)))"#
    ]];

    assert_auto_package_update_autoload_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_reload_preserves_options_and_initializes_package_only_when_needed() {
    let elisp_form = r##"(let
                             ((source
                               (getenv
                                "NEOMACS_PACKAGE_SOURCE"))
                              calls)
                           (setq
                            auto-package-update-interval 31
                            auto-package-update-hide-results t
                            package--initialized nil)
                           (cl-letf
                               (((symbol-function
                                  'package-initialize)
                                 (lambda (&rest arguments)
                                   (push arguments calls)
                                   (setq
                                    package--initialized
                                    t)
                                   :initialized)))
                             (load source nil t t)
                             (load source nil t t)
                             (list
                              auto-package-update-interval
                              auto-package-update-hide-results
                              package--initialized
                              (nreverse calls)
                              (featurep
                               'auto-package-update))))"##;
    let expect = expect!["OK (31 t t (nil) t)"];

    assert_auto_package_update_parity(elisp_form, expect);
}
