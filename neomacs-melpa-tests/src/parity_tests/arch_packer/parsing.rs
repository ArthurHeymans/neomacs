use expect_test::expect;

use super::{assert_arch_packer_parity, assert_arch_packer_signal_parity};

#[test]
fn installed_package_parser_handles_official_aur_multiline_and_colon_rich_metadata() {
    let elisp_form = r##"(let ((info
                         '("Name            : linux\nVersion         : 6.9.1-1\nDescription     : The Linux kernel : stable branch\nURL             : https://archlinux.org/packages/core/x86_64/linux/\nValidated By    : Signature"
                           "Name            : yay\nVersion         : 12.3.5-1\nDescription     : Yet another yogurt\nURL             : https://aur.archlinux.org/packages/yay\nValidated By    : None"))
                        (outdated nil))
                    (cl-letf
                        (((symbol-function 'arch-packer-get-info)
                          (lambda (&optional _package)
                            info))
                         ((symbol-function 'arch-packer-get-outdated)
                          (lambda () outdated)))
                      (arch-packer-get-package-alist)))"##;
    let expect = expect![[
        r#"OK (((Latest . "12.3.5-1") (Validated . None) (URL . "https://aur.archlinux.org/packages/yay") (Description . "Yet another yogurt") (Version . "12.3.5-1") (Name . "yay")) ((Latest . "6.9.1-1") (URL . "https://archlinux.org/packages/core/x86_64/linux/") (Description . "The Linux kernel") (Version . "6.9.1-1") (Name . "linux")))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn installed_package_parser_merges_outdated_versions_and_defaults_current_latest() {
    let elisp_form = r##"(let ((info
                         '("Name : linux\nVersion : 6.8.1\nDescription : kernel\nURL : https://linux\nValidated By : Signature"
                           "Name : ripgrep\nVersion : 14.0.3\nDescription : recursive search\nURL : https://rg\nValidated By : Signature"
                           "Name : local-tool\nVersion : 1.0\nDescription : local\nURL : https://local\nValidated By : None"))
                        (outdated
                         '(((name . "linux") (latest . "6.9.1"))
                           ((name . "uninstalled") (latest . "9.9")))))
                    (cl-letf
                        (((symbol-function 'arch-packer-get-info)
                          (lambda (&optional _package) info))
                         ((symbol-function 'arch-packer-get-outdated)
                          (lambda () outdated))
                         ((symbol-function 'return)
                          (lambda (&optional value) value)))
                      (arch-packer-get-package-alist)))"##;
    let expect = expect![[
        r#"OK (((Latest . "1.0") (Validated . None) (URL . "https://local") (Description . "local") (Version . "1.0") (Name . "local-tool")) ((Latest . "14.0.3") (URL . "https://rg") (Description . "recursive search") (Version . "14.0.3") (Name . "ripgrep")) ((Latest . "6.9.1") (URL . "https://linux") (Description . "kernel") (Version . "6.8.1") (Name . "linux")))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn matching_outdated_package_surfaces_the_source_missing_cl_return_contract() {
    let elisp_form = r##"(cl-letf
                    (((symbol-function 'arch-packer-get-info)
                      (lambda (&optional _package)
                        '("Name : linux\nVersion : 6.8.1\nDescription : kernel\nURL : https://linux\nValidated By : Signature")))
                     ((symbol-function 'arch-packer-get-outdated)
                      (lambda ()
                        '(((name . "linux") (latest . "6.9.1"))))))
                  (arch-packer-get-package-alist))"##;
    let expect = expect!["ERR (void-function return)"];
    assert_arch_packer_signal_parity(elisp_form, expect);
}

#[test]
fn outdated_parser_understands_realistic_pacman_and_pacaur_column_shapes() {
    let elisp_form = r##"(let (commands)
                    (cl-letf
                        (((symbol-function 'shell-command-to-string)
                          (lambda (command)
                            (push command commands)
                            (if (string-prefix-p "pacman" command)
                                "linux 6.8.1 -> 6.9.1\nripgrep 14.0.2 -> 14.1.0\n"
                              "aur package yay 12.2.0 -> 12.3.5\ncommunity package paru 2.0 -> 2.1\n"))))
                      (let ((arch-packer-default-command "pacman"))
                        (let ((pacman (arch-packer-get-outdated)))
                          (let ((arch-packer-default-command "pacaur"))
                            (list
                             pacman
                             (arch-packer-get-outdated)
                             (nreverse commands)))))))"##;
    let expect = expect![[
        r#"OK ((((latest . "6.9.1") (name . "linux")) ((latest . "14.1.0") (name . "ripgrep"))) (((latest . "12.3.5") (name . "yay")) ((latest . "2.1") (name . "paru"))) ("pacman -Qu" "pacaur -Qu"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn package_info_command_preserves_block_boundaries_and_query_policy() {
    let elisp_form = r##"(let (calls)
                    (cl-letf
                        (((symbol-function 'shell-command-to-string)
                          (lambda (command)
                            (push command calls)
                            "Name : one\nVersion : 1\n\nName : two\nVersion : 2\n\n")))
                      (let ((arch-packer-default-command "pacman")
                            (arch-packer-query-options t))
                        (let ((all (arch-packer-get-info)))
                          (let ((arch-packer-query-options nil))
                            (list
                             all
                             (arch-packer-get-info "linux")
                             (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (("Name : one\nVersion : 1" "Name : two\nVersion : 2") ("Name : one\nVersion : 1" "Name : two\nVersion : 2") ("pacman  -Qe --info" "pacman linux -Q --info"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn repository_search_parser_builds_structured_results_from_real_multiline_output() {
    let elisp_form = r##"(let ((arch-packer-search-string "pacman"))
                    (cl-letf
                        (((symbol-function 'arch-packer-search-pkg)
                          (lambda (_query)
                            (concat
                             "core/pacman 6.0.2-7\n"
                             "    A library-based package manager with dependency support\n"
                             "extra/pacman-contrib 1.10.6-1\n"
                             "    Contributed scripts and tools for pacman systems\n"
                             "aur/pacmanlogviewer 1.3.1-5\n"
                             "    Inspect pacman log files\n"))))
                      (arch-packer-get-search-alist)))"##;
    let expect = expect![[
        r#"OK (((Version . "1.3.1-5") (Name . "pacmanlogviewer") (Repository . "aur") (Description . "Inspect pacman log files")) ((Version . "1.10.6-1") (Name . "pacman-contrib") (Repository . "extra") (Description . "Contributed scripts and tools for pacman systems")) ((Version . "6.0.2-7") (Name . "pacman") (Repository . "core") (Description . "A library-based package manager with dependency support")))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn repository_search_parser_skips_noise_blank_lines_and_continuation_description_lines() {
    let elisp_form = r##"(let ((arch-packer-search-string "tool"))
                    (cl-letf
                        (((symbol-function 'arch-packer-search-pkg)
                          (lambda (_query)
                            (concat
                             "warning: database is stale\n"
                             "\n"
                             "extra/tool-one 2.0-1\n"
                             "    First description\n"
                             "    continuation not captured\n"
                             "community/tool-two 3.1-4 [installed]\n"
                             "    Second description with spaces\n"))))
                      (arch-packer-get-search-alist)))"##;
    let expect = expect![[
        r#"OK (((Version . "3.1-4") (Name . "tool-two") (Repository . "community") (Description . "Second description with spaces")) ((Version . "2.0-1") (Name . "tool-one") (Repository . "extra") (Description . "First description")))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn search_command_composes_selected_backend_query_exactly_and_returns_shell_output() {
    let elisp_form = r##"(let (calls)
                    (cl-letf
                        (((symbol-function 'shell-command-to-string)
                          (lambda (command)
                            (push command calls)
                            "search output")))
                      (list
                       (let ((arch-packer-default-command "pacman"))
                         (arch-packer-search-pkg
                          "linux hardened"))
                       (let ((arch-packer-default-command "pacaur"))
                         (arch-packer-search-pkg
                          "editor"))
                       (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("search output" "search output" ("pacman -Ss linux hardened" "pacaur -Ss editor"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn asynchronous_menu_pipeline_runs_package_collection_then_delivers_it_to_renderer() {
    let elisp_form = r##"(let (start-function callback-function rendered)
                    (cl-letf
                        (((symbol-function 'async-start)
                          (lambda (start callback)
                            (setq start-function start
                                  callback-function callback)
                            'fake-async))
                         ((symbol-function 'package-initialize)
                          (lambda () :initialized))
                         ((symbol-function 'arch-packer-get-package-alist)
                          (lambda ()
                            '(((Name . "linux")
                               (Version . "6.9")
                               (Latest . "6.9")))))
                         ((symbol-function 'arch-packer-generate-menu)
                          (lambda (packages)
                            (setq rendered packages))))
                      (let ((returned
                             (arch-packer-pkg-menu-async)))
                        (let ((collected (funcall start-function)))
                          (funcall callback-function collected)
                          (list returned collected rendered)))))"##;
    let expect = expect![[
        r#"OK (fake-async #1=(((Name . "linux") (Version . "6.9") (Latest . "6.9"))) #1#)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}
