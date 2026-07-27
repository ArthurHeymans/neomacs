use expect_test::expect;

use super::{assert_arch_packer_autoload_parity, assert_arch_packer_parity};

#[test]
fn installed_descriptor_source_dependencies_and_files_identify_the_exact_melpa_build() {
    let elisp_form = r##"(let* ((descriptor
                          (cadr (assq 'arch-packer package-alist)))
                         (source (getenv "NEOMACS_PACKAGE_SOURCE"))
                         (directory (file-name-directory source)))
                    (list
                     (featurep 'arch-packer)
                     (package-desc-name descriptor)
                     (package-version-join
                      (package-desc-version descriptor))
                     (package-desc-reqs descriptor)
                     (package-desc-summary descriptor)
                     (file-name-nondirectory source)
                     (file-name-nondirectory
                      (symbol-file 'arch-packer-menu-entry 'defun))
                     (sort
                      (mapcar
                       #'file-name-nondirectory
                       (directory-files
                        directory t
                        "\\`arch-packer.*\\.el\\'"))
                      #'string<)))"##;
    let expect = expect![[
        r#"OK (t arch-packer "20170730.1321" ((emacs (25 1)) (s (1 11 0)) (async (1 9 2)) (dash (2 12 0))) "Arch Linux package management frontend." "arch-packer.el" "arch-packer.el" ("arch-packer-autoloads.el" "arch-packer-pkg.el" "arch-packer.el"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_file_exposes_exactly_the_three_public_entry_commands() {
    let elisp_form = r##"(list
                    (featurep 'arch-packer)
                    (featurep 'arch-packer-autoloads)
                    (mapcar
                     (lambda (symbol)
                       (list
                        symbol
                        (fboundp symbol)
                        (and
                         (fboundp symbol)
                         (autoloadp (symbol-function symbol)))
                        (commandp symbol)))
                     '(arch-packer-search-package
                       arch-packer-install-package
                       arch-packer-list-packages
                       arch-packer-menu-entry
                       arch-packer-package-menu-mode))
                    (boundp 'arch-packer-default-command))"##;
    let expect = expect![
        "OK (nil t ((arch-packer-search-package t t t) (arch-packer-install-package t t t) (arch-packer-list-packages t t t) (arch-packer-menu-entry nil nil nil) (arch-packer-package-menu-mode nil nil nil)) nil)"
    ];
    assert_arch_packer_autoload_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_has_exact_arglists_and_command_status() {
    let elisp_form = r##"(mapcar
                    (lambda (symbol)
                      (list
                       symbol
                       (copy-tree
                        (help-function-arglist symbol t))
                       (commandp symbol)))
                    '(arch-packer-package-menu-mode
                      arch-packer-menu-entry
                      arch-packer-generate-menu
                      arch-packer-get-package-alist
                      arch-packer-search-mode
                      arch-packer-search-entry
                      arch-packer-generate-search-menu
                      arch-packer-get-search-alist
                      arch-packer-search-pkg
                      arch-packer-pkg-menu-async
                      arch-packer-output-mode
                      arch-packer-open-shell-process
                      arch-packer-process-filter
                      arch-packer-process-sentinel
                      arch-packer-call-shell-process
                      arch-packer-send-root
                      arch-packer-shell-process-live-p
                      arch-packer-wait-shell-subprocess
                      arch-packer-get-exit-status
                      arch-packer-disable-status-reporter
                      arch-packer-enable-status-reporter
                      arch-packer-status-reporter
                      arch-packer-get-output-buffer-create
                      arch-packer-shell-command
                      arch-packer-refresh-database
                      arch-packer-delete-package
                      arch-packer-upgrade-package
                      arch-packer-get-info
                      arch-packer-get-outdated
                      arch-packer-display-output-buffer
                      arch-packer-menu-mark-upgrade
                      arch-packer-menu-mark-delete
                      arch-packer-menu-mark-unmark
                      arch-packer-menu-mark-all-upgrades
                      arch-packer-menu-visit-homepage
                      arch-packer-pkg-info
                      arch-packer-menu-execute
                      arch-packer-search-package
                      arch-packer-install-package
                      arch-packer-list-packages))"##;
    let expect = expect![
        "OK ((arch-packer-package-menu-mode nil t) (arch-packer-menu-entry (pkg) nil) (arch-packer-generate-menu (packages) nil) (arch-packer-get-package-alist nil nil) (arch-packer-search-mode nil t) (arch-packer-search-entry (pkg) nil) (arch-packer-generate-search-menu nil t) (arch-packer-get-search-alist nil nil) (arch-packer-search-pkg (search-string) nil) (arch-packer-pkg-menu-async nil nil) (arch-packer-output-mode nil t) (arch-packer-open-shell-process nil nil) (arch-packer-process-filter (proc output) nil) (arch-packer-process-sentinel (_proc _output) nil) (arch-packer-call-shell-process (proc string) nil) (arch-packer-send-root nil nil) (arch-packer-shell-process-live-p nil nil) (arch-packer-wait-shell-subprocess nil nil) (arch-packer-get-exit-status nil nil) (arch-packer-disable-status-reporter nil nil) (arch-packer-enable-status-reporter nil nil) (arch-packer-status-reporter nil nil) (arch-packer-get-output-buffer-create nil nil) (arch-packer-shell-command nil nil) (arch-packer-refresh-database nil nil) (arch-packer-delete-package (packages) nil) (arch-packer-upgrade-package (packages) nil) (arch-packer-get-info (&optional package) nil) (arch-packer-get-outdated nil nil) (arch-packer-display-output-buffer nil t) (arch-packer-menu-mark-upgrade nil t) (arch-packer-menu-mark-delete nil t) (arch-packer-menu-mark-unmark nil t) (arch-packer-menu-mark-all-upgrades nil t) (arch-packer-menu-visit-homepage nil t) (arch-packer-pkg-info nil t) (arch-packer-menu-execute nil t) (arch-packer-search-package nil t) (arch-packer-install-package nil t) (arch-packer-list-packages nil t))"
    ];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn custom_options_have_exact_defaults_types_groups_and_documentation() {
    let elisp_form = r##"(mapcar
                    (lambda (symbol)
                      (list
                       symbol
                       (default-value symbol)
                       (get symbol 'custom-type)
                       (get symbol 'custom-group)
                       (documentation-property
                        symbol 'variable-documentation t)))
                    '(arch-packer-default-command
                      arch-packer-column-width-package
                      arch-packer-column-width-version
                      arch-packer-repository-column-width-version
                      arch-packer-highlight-aur-packages
                      arch-packer-query-options
                      arch-packer-display-status-reporter
                      arch-packer-highlight-search-string
                      arch-packer-menu-latest-face
                      arch-packer-menu-aur-face
                      arch-packer-info-attribute-face
                      arch-packer-info-dependencies-face
                      arch-packer-search-string-highlight-face))"##;
    let expect = expect![[
        r##"OK ((arch-packer-default-command "pacman" (choice (const :tag "pacman" "pacman") (const :tag "pacaur" "pacaur")) nil "Default package manager.") (arch-packer-column-width-package 18 integer nil "Width of the Package column.") (arch-packer-column-width-version 20 integer nil "Width of the Version and Latest columns.") (arch-packer-repository-column-width-version 15 integer nil "Width of the repository column in `arch-packer-search-mode'.") (arch-packer-highlight-aur-packages t boolean nil "Highlight AUR packages.") (arch-packer-query-options t boolean nil "Restrict or filter output to explicitly installed packages.") (arch-packer-display-status-reporter t boolean nil "Display progress-reporter.") (arch-packer-highlight-search-string t boolean nil "Highlight search string in `arch-packer-search-mode' buffer.") (arch-packer-menu-latest-face "firebrick" face nil "Face for latest version when newer than installed version.") (arch-packer-menu-aur-face "#1793d0" face nil "Face for AUR packages.") (arch-packer-info-attribute-face "#6e8b3d" face nil "Package attribute face for pacman-package-info buffer.") (arch-packer-info-dependencies-face "#b0e0e6" face nil "Package dependencies face for pacman-package-info buffer.") (arch-packer-search-string-highlight-face "orange" face nil "Face for highlighted search string in `arch-packer-search-mode'."))"##
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn runtime_state_defaults_and_mode_lineage_are_exact() {
    let elisp_form = r##"(list
                    arch-packer-search-string
                    arch-packer-process-name
                    arch-packer-process-buffer
                    arch-packer-process-output
                    arch-packer-process-output-buffer
                    arch-packer-no-shell-history
                    (get 'arch-packer-package-menu-mode
                         'derived-mode-parent)
                    (get 'arch-packer-search-mode
                         'derived-mode-parent)
                    (get 'arch-packer-output-mode
                         'derived-mode-parent)
                    (get 'arch-packer 'custom-group))"##;
    let expect = expect![[
        r#"OK (nil "arch-packer-process" "*Pacman-Packages*" nil "*arch-packer-output*" "; history -d $((HISTCMD-1))" tabulated-list-mode tabulated-list-mode special-mode ((arch-packer-default-command custom-variable) (arch-packer-column-width-package custom-variable) (arch-packer-column-width-version custom-variable) (arch-packer-repository-column-width-version custom-variable) (arch-packer-highlight-aur-packages custom-variable) (arch-packer-query-options custom-variable) (arch-packer-display-status-reporter custom-variable) (arch-packer-highlight-search-string custom-variable) (arch-packer-menu-latest-face custom-variable) (arch-packer-menu-aur-face custom-variable) (arch-packer-info-attribute-face custom-variable) (arch-packer-info-dependencies-face custom-variable) (arch-packer-search-string-highlight-face custom-variable)))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn all_three_keymaps_inherit_expected_parents_and_bind_every_documented_operation() {
    let elisp_form = r##"(list
                    (eq
                     (keymap-parent arch-packer-package-menu-mode-map)
                     tabulated-list-mode-map)
                    (mapcar
                     (lambda (key)
                       (cons
                        key
                        (lookup-key
                         arch-packer-package-menu-mode-map
                         (kbd key))))
                     '("m" "d" "U" "u" "r" "s" "i" "x"
                       "b" "o" "RET" "q"))
                    (eq
                     (keymap-parent arch-packer-search-mode-map)
                     tabulated-list-mode-map)
                    (mapcar
                     (lambda (key)
                       (cons
                        key
                        (lookup-key
                         arch-packer-search-mode-map
                         (kbd key))))
                     '("i" "r" "s" "RET" "q"))
                    (eq
                     (keymap-parent arch-packer-output-mode-map)
                     special-mode-map))"##;
    let expect = expect![[
        r#"OK (t (("m" . arch-packer-menu-mark-unmark) ("d" . arch-packer-menu-mark-delete) ("U" . arch-packer-menu-mark-all-upgrades) ("u" . arch-packer-menu-mark-upgrade) ("r" . arch-packer-list-packages) ("s" . arch-packer-search-package) ("i" . arch-packer-install-package) ("x" . arch-packer-menu-execute) ("b" . arch-packer-menu-visit-homepage) ("o" . arch-packer-display-output-buffer) ("RET" . arch-packer-pkg-info) ("q" . quit-window)) t (("i" . arch-packer-install-package) ("r" . arch-packer-list-packages) ("s" . arch-packer-search-package) ("RET" . arch-packer-pkg-info) ("q" . quit-window)) nil)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn each_derived_mode_sets_practical_tabulated_columns_padding_and_buffer_behavior() {
    let elisp_form = r##"(list
                    (with-temp-buffer
                      (arch-packer-package-menu-mode)
                      (list
                       major-mode mode-name
                       buffer-read-only truncate-lines
                       (append tabulated-list-format nil)
                       tabulated-list-padding))
                    (with-temp-buffer
                      (arch-packer-search-mode)
                      (list
                       major-mode mode-name
                       buffer-read-only truncate-lines
                       (append tabulated-list-format nil)
                       tabulated-list-padding))
                    (with-temp-buffer
                      (arch-packer-output-mode)
                      (list
                       major-mode mode-name
                       buffer-read-only truncate-lines
                       (eq
                        (current-local-map)
                        arch-packer-output-mode-map))))"##;
    let expect = expect![[
        r#"OK ((arch-packer-package-menu-mode "Package Menu" nil t (("Package" 18 nil) ("Version" 20 nil) ("Latest" 20 nil) ("Description" 0 nil)) 2) (arch-packer-search-mode "Search Menu" nil t (("Package" 18 nil) ("Version" 20 nil) ("Repository" 15 nil) ("Description" 0 nil)) 2) (arch-packer-output-mode "Process output" nil t t))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}
