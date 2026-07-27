use expect_test::expect;

use super::{assert_arch_packer_parity, assert_arch_packer_signal_parity};

#[test]
fn row_mark_commands_upgrade_only_outdated_entries_and_delete_unmark_move_to_next_rows() {
    let elisp_form = r##"(with-temp-buffer
                    (arch-packer-package-menu-mode)
                    (setq tabulated-list-entries
                          '(("linux"
                             ["linux" "6.8" "6.9" "kernel"])
                            ("ripgrep"
                             ["ripgrep" "14.1" "14.1" "search"])
                            ("old"
                             ["old" "1.0" "1.0" "unused"])))
                    (tabulated-list-print t)
                    (goto-char (point-min))
                    (arch-packer-menu-mark-upgrade)
                    (let ((after-upgrade
                           (buffer-substring-no-properties
                            (point-min) (point-max)))
                          (line-after-upgrade
                           (line-number-at-pos)))
                      (arch-packer-menu-mark-upgrade)
                      (let ((after-current
                             (buffer-substring-no-properties
                              (point-min) (point-max)))
                            (line-after-current
                             (line-number-at-pos)))
                        (forward-line)
                        (arch-packer-menu-mark-delete)
                        (forward-line -1)
                        (arch-packer-menu-mark-unmark)
                        (list
                         after-upgrade line-after-upgrade
                         after-current line-after-current
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         (line-number-at-pos)))))"##;
    let expect = expect![[
        r#"OK ("U linux              6.8                  6.9                  kernel\n  ripgrep            14.1                 14.1                 search\n  old                1.0                  1.0                  unused\n" 1 "U linux              6.8                  6.9                  kernel\n  ripgrep            14.1                 14.1                 search\n  old                1.0                  1.0                  unused\n" 1 "U linux              6.8                  6.9                  kernel\n  ripgrep            14.1                 14.1                 search\n  old                1.0                  1.0                  unused\n" 3)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn mark_all_upgrades_scans_real_tabulated_rows_and_leaves_current_versions_unmarked() {
    let elisp_form = r##"(with-temp-buffer
                    (arch-packer-package-menu-mode)
                    (setq tabulated-list-entries
                          '(("linux"
                             ["linux" "6.8" "6.9" "kernel"])
                            ("ripgrep"
                             ["ripgrep" "14.1" "14.1" "search"])
                            ("mesa"
                             ["mesa" "24.0" "24.1" "graphics"])))
                    (tabulated-list-print t)
                    (arch-packer-menu-mark-all-upgrades)
                    (list
                     (buffer-substring-no-properties
                      (point-min) (point-max))
                     (line-number-at-pos)
                     (point)))"##;
    let expect = expect![[
        r#"OK ("U linux              6.8                  6.9                  kernel\n  ripgrep            14.1                 14.1                 search\nU mesa               24.0                 24.1                 graphics\n" 1 1)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn homepage_command_reads_link_property_from_rendered_package_name() {
    let elisp_form = r##"(with-temp-buffer
                    (arch-packer-package-menu-mode)
                    (let* ((name
                            (propertize
                             "linux"
                             'link
                             "https://archlinux.org/packages/linux"))
                           visited)
                      (setq tabulated-list-entries
                            `(("linux"
                               [,name "6.9" "6.9" "kernel"])))
                      (tabulated-list-print t)
                      (goto-char (point-min))
                      (cl-letf
                          (((symbol-function 'browse-url)
                            (lambda (url &rest args)
                              (setq visited
                                    (list url args)))))
                        (arch-packer-menu-visit-homepage)
                        visited)))"##;
    let expect = expect![[r#"OK ("https://archlinux.org/packages/linux" nil)"#]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn menu_execute_collects_real_marks_confirms_summary_and_orders_upgrade_before_delete() {
    let elisp_form = r##"(with-temp-buffer
                    (arch-packer-package-menu-mode)
                    (setq tabulated-list-entries
                          '(("linux"
                             ["linux" "6.8" "6.9" "kernel"])
                            ("mesa"
                             ["mesa" "24.0" "24.1" "graphics"])
                            ("old-one"
                             ["old-one" "1.0" "1.0" "unused"])
                            ("old-two"
                             ["old-two" "2.0" "2.0" "unused"])))
                    (tabulated-list-print t)
                    (goto-char (point-min))
                    (tabulated-list-put-tag "U" t)
                    (tabulated-list-put-tag "U" t)
                    (tabulated-list-put-tag "D" t)
                    (tabulated-list-put-tag "D" t)
                    (let (events)
                      (cl-letf
                          (((symbol-function
                             'process-running-child-p)
                            (lambda (_process) nil))
                           ((symbol-function 'yes-or-no-p)
                            (lambda (prompt)
                              (push
                               (list
                                :prompt
                                (substring-no-properties prompt))
                               events)
                              t))
                           ((symbol-function
                             'arch-packer-upgrade-package)
                            (lambda (packages)
                              (push
                               (list :upgrade packages)
                               events)))
                           ((symbol-function
                             'arch-packer-delete-package)
                            (lambda (packages)
                              (push
                               (list :delete packages)
                               events)))
                           ((symbol-function
                             'arch-packer-wait-shell-subprocess)
                            (lambda ()
                              (push :wait events))))
                        (arch-packer-menu-execute)
                        (nreverse events))))"##;
    let expect = expect![[
        r#"OK ((:prompt "Delete 2 packages (old-two, old-one) and Upgrade 2 packages (mesa, linux)") (:upgrade "mesa linux") :wait (:delete "old-two old-one"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn menu_execute_without_marks_surfaces_exact_user_error() {
    let elisp_form = r##"(with-temp-buffer
                    (arch-packer-package-menu-mode)
                    (setq tabulated-list-entries
                          '(("linux"
                             ["linux" "6.9" "6.9" "kernel"])))
                    (tabulated-list-print t)
                    (cl-letf
                        (((symbol-function
                           'process-running-child-p)
                          (lambda (_process) nil)))
                      (arch-packer-menu-execute)))"##;
    let expect = expect![[r#"ERR (user-error "No operations specified")"#]];
    assert_arch_packer_signal_parity(elisp_form, expect);
}

#[test]
fn busy_child_process_makes_menu_execute_a_strict_noop() {
    let elisp_form = r##"(let (events)
                    (cl-letf
                        (((symbol-function
                           'process-running-child-p)
                          (lambda (_process) t))
                         ((symbol-function 'yes-or-no-p)
                          (lambda (_prompt)
                            (push :prompt events)))
                         ((symbol-function
                           'arch-packer-upgrade-package)
                          (lambda (_packages)
                            (push :upgrade events))))
                      (list
                       (arch-packer-menu-execute)
                       events)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn search_command_opens_shell_when_needed_trims_query_and_requests_status() {
    let elisp_form = r##"(let (events)
                    (cl-letf
                        (((symbol-function
                           'arch-packer-shell-process-live-p)
                          (lambda () nil))
                         ((symbol-function
                           'arch-packer-open-shell-process)
                          (lambda ()
                            (push :open events)
                            t))
                         ((symbol-function
                           'process-running-child-p)
                          (lambda (_process) nil))
                         ((symbol-function
                           'read-from-minibuffer)
                          (lambda (prompt)
                            (push (list :read prompt) events)
                            "  linux hardened  "))
                         ((symbol-function
                           'arch-packer-get-exit-status)
                          (lambda ()
                            (push :status events))))
                      (arch-packer-search-package)
                      (list
                       arch-packer-search-string
                       (nreverse events))))"##;
    let expect =
        expect![[r#"OK ("linux hardened" (:open (:read "Enter package name: ") :status))"#]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn normal_install_opens_shell_refreshes_database_trims_packages_and_upgrades() {
    let elisp_form = r##"(let (events)
                    (cl-letf
                        (((symbol-function
                           'arch-packer-shell-process-live-p)
                          (lambda () nil))
                         ((symbol-function
                           'arch-packer-open-shell-process)
                          (lambda ()
                            (push :open events)
                            t))
                         ((symbol-function
                           'arch-packer-refresh-database)
                          (lambda ()
                            (push :refresh events)
                            t))
                         ((symbol-function
                           'process-running-child-p)
                          (lambda (_process) nil))
                         ((symbol-function
                           'read-from-minibuffer)
                          (lambda (prompt)
                            (push (list :read prompt) events)
                            "  linux linux-headers  "))
                         ((symbol-function
                           'arch-packer-upgrade-package)
                          (lambda (packages)
                            (push
                             (list :upgrade packages)
                             events))))
                      (arch-packer-install-package)
                      (nreverse events)))"##;
    let expect = expect![[
        r#"OK (:open :refresh (:read "Enter package name: ") (:upgrade "linux linux-headers"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn search_result_install_uses_row_package_and_confirmation_without_database_refresh() {
    let elisp_form = r##"(progn
                    (require 'thingatpt)
                    (with-temp-buffer
                    (arch-packer-search-mode)
                    (setq tabulated-list-entries
                          '(("pacman-contrib"
                             ["pacman-contrib" "1.10" "extra"
                              "pacman tools"])))
                    (tabulated-list-print t)
                    (goto-char (point-min))
                    (let (events)
                      (cl-letf
                          (((symbol-function
                             'arch-packer-shell-process-live-p)
                            (lambda () t))
                           ((symbol-function
                             'process-running-child-p)
                            (lambda (_process) nil))
                           ((symbol-function 'yes-or-no-p)
                            (lambda (prompt)
                              (push
                               (list
                                :prompt
                                (substring-no-properties prompt))
                               events)
                              t))
                           ((symbol-function
                             'arch-packer-upgrade-package)
                            (lambda (package)
                              (push
                               (list
                                :upgrade
                                (substring-no-properties package))
                               events))))
                        (arch-packer-install-package)
                        (nreverse events)))))"##;
    let expect = expect![[r#"OK ((:prompt "Install package pacman ?") (:upgrade "pacman"))"#]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn search_result_install_surfaces_missing_thingatpt_dependency_in_a_fresh_session() {
    let elisp_form = r##"(with-temp-buffer
                    (arch-packer-search-mode)
                    (setq tabulated-list-entries
                          '(("pacman-contrib"
                             ["pacman-contrib" "1.10" "extra"
                              "pacman tools"])))
                    (tabulated-list-print t)
                    (goto-char (point-min))
                    (cl-letf
                        (((symbol-function
                           'arch-packer-shell-process-live-p)
                          (lambda () t))
                         ((symbol-function
                           'process-running-child-p)
                          (lambda (_process) nil)))
                      (arch-packer-install-package)))"##;
    let expect = expect!["ERR (void-function word-at-point)"];
    assert_arch_packer_signal_parity(elisp_form, expect);
}

#[test]
fn list_packages_selects_open_refresh_status_or_busy_noop_paths() {
    let elisp_form = r##"(let (events live child)
                    (cl-letf
                        (((symbol-function
                           'arch-packer-shell-process-live-p)
                          (lambda () live))
                         ((symbol-function
                           'process-running-child-p)
                          (lambda (_process) child))
                         ((symbol-function
                           'arch-packer-open-shell-process)
                          (lambda ()
                            (push :open events)
                            t))
                         ((symbol-function
                           'arch-packer-refresh-database)
                          (lambda ()
                            (push :refresh events)
                            t))
                         ((symbol-function
                           'arch-packer-get-exit-status)
                          (lambda ()
                            (push :status events)
                            t)))
                      (setq live nil child nil)
                      (arch-packer-list-packages)
                      (push :separator events)
                      (setq live t child nil)
                      (arch-packer-list-packages)
                      (push :separator events)
                      (setq live t child t)
                      (arch-packer-list-packages)
                      (nreverse events)))"##;
    let expect = expect!["OK (:open :refresh :status :separator :status :separator)"];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn output_command_displays_the_exact_factory_buffer() {
    let elisp_form = r##"(let (events)
                    (cl-letf
                        (((symbol-function
                           'arch-packer-get-output-buffer-create)
                          (lambda ()
                            (push :factory events)
                            'fixture-buffer))
                         ((symbol-function 'display-buffer)
                          (lambda (buffer)
                            (push
                             (list :display buffer)
                             events)
                            :displayed)))
                      (list
                       (arch-packer-display-output-buffer)
                       (nreverse events))))"##;
    let expect = expect!["OK (:displayed (:factory (:display fixture-buffer)))"];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn password_dispatch_installs_cancel_binding_sends_secret_and_clears_it() {
    let elisp_form = r##"(let ((minibuffer-local-map
                         (make-sparse-keymap))
                        events)
                    (cl-letf
                        (((symbol-function 'read-passwd)
                          (lambda (prompt)
                            (push (list :read prompt) events)
                            (copy-sequence "secret")))
                         ((symbol-function
                           'arch-packer-call-shell-process)
                          (lambda (process password)
                            (push
                             (list :send process password)
                             events)))
                         ((symbol-function 'clear-string)
                          (lambda (password)
                            (push
                             (list :clear password)
                             events))))
                      (arch-packer-send-root)
                      (list
                       (nreverse events)
                       (commandp
                        (lookup-key
                         minibuffer-local-map
                         (kbd "C-g"))))))"##;
    let expect = expect![[
        r#"OK (((:read "Password: ") (:send "arch-packer-process" "secret") (:clear "secret")) t)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn package_info_formats_real_pacman_attributes_with_faces_and_special_mode() {
    let elisp_form = r##"(let ((info-buffer
                         (get-buffer
                          "*pacman-package-info*")))
                    (when info-buffer
                      (kill-buffer info-buffer))
                    (unwind-protect
                        (with-temp-buffer
                          (arch-packer-package-menu-mode)
                          (setq tabulated-list-entries
                                '(("linux"
                                   ["linux" "6.9" "6.9" "kernel"])))
                          (tabulated-list-print t)
                          (goto-char (point-min))
                          (cl-letf
                              (((symbol-function
                                 'arch-packer-get-info)
                                (lambda (&optional _package)
                                  '("Name            : linux\nVersion         : 6.9\nDepends On      : coreutils kmod\nDescription     : The Linux kernel")))
                               ((symbol-function 'pop-to-buffer)
                                (lambda (buffer &rest _args)
                                  buffer)))
                            (arch-packer-pkg-info)
                            (with-current-buffer
                                "*pacman-package-info*"
                              (list
                               (buffer-string)
                               major-mode mode-name
                               buffer-read-only
                               (let ((properties nil))
                                 (goto-char (point-min))
                                 (while (not (eobp))
                                   (push
                                    (list
                                     (buffer-substring-no-properties
                                      (line-beginning-position)
                                      (line-end-position))
                                     (get-text-property
                                      (line-beginning-position)
                                      'font-lock-face))
                                    properties)
                                   (forward-line))
                                 (nreverse properties))))))
                      (when
                          (get-buffer
                           "*pacman-package-info*")
                        (kill-buffer
                         "*pacman-package-info*"))))"##;
    let expect = expect![[
        r##"OK (#("Name            : linux\nVersion         : 6.9\nDepends On      : coreutils kmod\nDescription     : The Linux kernel\n" 0 15 (font-lock-face (:foreground #1="#6e8b3d")) 24 39 (font-lock-face (:foreground #1#)) 46 61 (font-lock-face (:foreground #1#)) 64 78 (font-lock-face (:foreground "#b0e0e6")) 79 94 (font-lock-face (:foreground #1#))) special-mode "Special" t (("Name            : linux" (:foreground "#6e8b3d")) ("Version         : 6.9" (:foreground "#6e8b3d")) ("Depends On      : coreutils kmod" (:foreground "#6e8b3d")) ("Description     : The Linux kernel" (:foreground "#6e8b3d"))))"##
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}
