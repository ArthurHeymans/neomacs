use expect_test::expect;

use super::assert_arch_packer_parity;

#[test]
fn official_current_package_entry_preserves_link_and_plain_latest_columns() {
    let elisp_form = r##"(let* ((name (copy-sequence "ripgrep"))
                         (pkg
                          `((Name . ,name)
                            (Version . "14.1.0")
                            (Latest . "14.1.0")
                            (Description . "Recursive search")
                            (URL . "https://github.com/BurntSushi/ripgrep")))
                         (entry (arch-packer-menu-entry pkg))
                         (columns (append (cadr entry) nil)))
                    (list
                     entry
                     (get-text-property 0 'link (nth 0 columns))
                     (get-text-property 0 'AUR (nth 0 columns))
                     (get-text-property
                      0 'font-lock-face (nth 0 columns))
                     (get-text-property
                      0 'font-lock-face (nth 2 columns))
                     (equal name (nth 0 columns))))"##;
    let expect = expect![[
        r#"OK ((#("ripgrep" 0 7 (link #1="https://github.com/BurntSushi/ripgrep")) [#("ripgrep" 0 7 (link #1#)) "14.1.0" "14.1.0" "Recursive search"]) "https://github.com/BurntSushi/ripgrep" nil nil nil t)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn official_outdated_package_entry_highlights_latest_but_not_package_name() {
    let elisp_form = r##"(let* ((pkg
                          '((Name . "linux")
                            (Version . "6.8.1")
                            (Latest . "6.9.1")
                            (Description . "The Linux kernel")
                            (URL . "https://archlinux.org/packages/linux")))
                         (arch-packer-menu-latest-face "upgrade-red")
                         (entry (arch-packer-menu-entry pkg))
                         (columns (append (cadr entry) nil)))
                    (list
                     entry
                     (get-text-property 0 'link (nth 0 columns))
                     (get-text-property
                      0 'font-lock-face (nth 0 columns))
                     (get-text-property
                      0 'font-lock-face (nth 2 columns))))"##;
    let expect = expect![[
        r#"OK ((#("linux" 0 5 (link #1="https://archlinux.org/packages/linux")) [#("linux" 0 5 (link #1#)) "6.8.1" #("6.9.1" 0 5 (font-lock-face (:foreground "upgrade-red"))) "The Linux kernel"]) "https://archlinux.org/packages/linux" nil (:foreground "upgrade-red"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn aur_package_entry_marks_name_and_applies_backend_specific_latest_semantics() {
    let elisp_form = r##"(let ((make-package
                         (lambda ()
                           (list
                            (cons 'Name
                                  (copy-sequence "yay"))
                            '(Version . "12.3.5")
                            '(Latest . "12.3.5")
                            '(Description . "Yet another yogurt")
                            '(URL . "https://aur.archlinux.org/packages/yay")
                            '(Validated . None)))))
                    (mapcar
                     (lambda (backend)
                       (let* ((arch-packer-default-command backend)
                              (arch-packer-menu-aur-face "arch-blue")
                              (entry
                               (arch-packer-menu-entry
                                (funcall make-package)))
                              (columns (append (cadr entry) nil))
                              (name (nth 0 columns)))
                         (list
                          backend entry
                          (get-text-property 0 'AUR name)
                          (get-text-property 0 'link name)
                          (get-text-property
                           0 'font-lock-face name))))
                     '("pacman" "pacaur")))"##;
    let expect = expect![[
        r#"OK (("pacman" (#("yay" 0 3 (AUR #1="12.3.5" link #2="https://aur.archlinux.org/packages/yay")) [#("yay" 0 3 (font-lock-face (:foreground #3="arch-blue") AUR #1# link #2#)) "12.3.5" "N/A" "Yet another yogurt"]) "12.3.5" "https://aur.archlinux.org/packages/yay" (:foreground "arch-blue")) ("pacaur" (#("yay" 0 3 (AUR #1# link #2#)) [#("yay" 0 3 (font-lock-face (:foreground #3#) AUR #1# link #2#)) "12.3.5" "12.3.5" "Yet another yogurt"]) "12.3.5" "https://aur.archlinux.org/packages/yay" (:foreground "arch-blue")))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn package_menu_generation_orders_upgrades_first_then_current_packages_and_renders_rows() {
    let elisp_form = r##"(let* ((arch-packer-process-buffer
                          "*arch-packer-menu-contract*")
                         (packages
                          '(((Name . "current-one")
                             (Version . "1.0")
                             (Latest . "1.0")
                             (Description . "current first")
                             (URL . "https://one"))
                            ((Name . "upgrade-one")
                             (Version . "1.0")
                             (Latest . "2.0")
                             (Description . "upgrade first")
                             (URL . "https://upgrade-one"))
                            ((Name . "current-two")
                             (Version . "3.0")
                             (Latest . "3.0")
                             (Description . "current second")
                             (URL . "https://two"))
                            ((Name . "upgrade-two")
                             (Version . "4.0")
                             (Latest . "5.0")
                             (Description . "upgrade second")
                             (URL . "https://upgrade-two"))))
                         (buffer
                          (get-buffer-create
                           arch-packer-process-buffer))
                         displayed)
                    (unwind-protect
                        (cl-letf
                            (((symbol-function 'display-buffer)
                              (lambda (target)
                                (setq displayed target)
                                target)))
                          (arch-packer-generate-menu packages)
                          (with-current-buffer buffer
                            (list
                             (mapcar
                              (lambda (entry)
                                (substring-no-properties
                                 (car entry)))
                              tabulated-list-entries)
                             (buffer-substring-no-properties
                              (point-min) (point-max))
                             buffer-read-only
                             major-mode
                             (eq displayed buffer))))
                      (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (("upgrade-two" "upgrade-one" "current-two" "current-one") "  upgrade-two        4.0                  5.0                  upgrade second\n  upgrade-one        1.0                  2.0                  upgrade first\n  current-two        3.0                  3.0                  current second\n  current-one        1.0                  1.0                  current first\n" nil arch-packer-package-menu-mode t)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn search_entries_highlight_aur_repository_only_and_preserve_all_columns() {
    let elisp_form = r##"(let ((arch-packer-menu-aur-face "aur-blue"))
                    (mapcar
                     (lambda (pkg)
                       (let* ((entry
                               (arch-packer-search-entry pkg))
                              (columns (append (cadr entry) nil)))
                         (list
                          entry
                          (get-text-property
                           0 'font-lock-face (nth 2 columns)))))
                     '(((Name . "pacman")
                        (Version . "6.0.2")
                        (Repository . "core")
                        (Description . "Package manager"))
                       ((Name . "yay")
                        (Version . "12.3.5")
                        (Repository . "aur")
                        (Description . "AUR helper")))))"##;
    let expect = expect![[
        r#"OK ((("pacman" ["pacman" "6.0.2" "core" "Package manager"]) nil) (("yay" ["yay" "12.3.5" #("aur" 0 3 (font-lock-face (:foreground "aur-blue"))) "AUR helper"]) (:foreground "aur-blue")))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn search_menu_generation_reverses_parser_order_renders_and_highlights_every_match() {
    let elisp_form = r##"(let* ((arch-packer-process-buffer
                          "*arch-packer-search-contract*")
                         (arch-packer-search-string "pacman")
                         (arch-packer-search-string-highlight-face
                          "search-orange")
                         (buffer
                          (get-buffer-create
                           arch-packer-process-buffer))
                         displayed)
                    (unwind-protect
                        (cl-letf
                            (((symbol-function
                               'arch-packer-get-search-alist)
                              (lambda ()
                                '(((Name . "pacmanlogviewer")
                                   (Version . "1.3")
                                   (Repository . "aur")
                                   (Description . "Inspect pacman logs"))
                                  ((Name . "pacman-contrib")
                                   (Version . "1.10")
                                   (Repository . "extra")
                                   (Description . "pacman tools"))
                                  ((Name . "pacman")
                                   (Version . "6.0")
                                   (Repository . "core")
                                   (Description . "package manager")))))
                             ((symbol-function 'display-buffer)
                              (lambda (target)
                                (setq displayed target)
                                target)))
                          (arch-packer-generate-search-menu)
                          (with-current-buffer buffer
                            (let ((matches nil))
                              (goto-char (point-min))
                              (while
                                  (search-forward
                                   "pacman" nil t)
                                (push
                                 (get-text-property
                                  (match-beginning 0)
                                  'face)
                                 matches))
                              (list
                               (mapcar
                                (lambda (entry)
                                  (substring-no-properties
                                   (car entry)))
                                tabulated-list-entries)
                               (buffer-substring-no-properties
                                (point-min) (point-max))
                               (nreverse matches)
                               buffer-read-only
                               major-mode
                               (eq displayed buffer)))))
                      (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (("pacman" "pacman-contrib" "pacmanlogviewer") "  pacman             6.0                  core            package manager\n  pacman-contrib     1.10                 extra           pacman tools\n  pacmanlogviewer    1.3                  aur             Inspect pacman logs\n" (nil nil nil nil nil) nil arch-packer-search-mode t)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn package_entry_with_aur_highlighting_disabled_keeps_metadata_but_no_visual_face() {
    let elisp_form = r##"(let* ((arch-packer-highlight-aur-packages nil)
                         (pkg
                          '((Name . "paru")
                            (Version . "2.0")
                            (Latest . "2.1")
                            (Description . "AUR helper")
                            (URL . "https://aur.archlinux.org/packages/paru")
                            (Validated . None)))
                         (entry (arch-packer-menu-entry pkg))
                         (columns (append (cadr entry) nil))
                         (name (nth 0 columns)))
                    (list
                     entry
                     (get-text-property 0 'link name)
                     (get-text-property 0 'AUR name)
                     (get-text-property 0 'font-lock-face name)
                     (get-text-property
                      0 'font-lock-face (nth 2 columns))))"##;
    let expect = expect![[
        r#"OK ((#("paru" 0 4 (link #1="https://aur.archlinux.org/packages/paru")) [#("paru" 0 4 (link #1#)) "2.0" #("2.1" 0 3 (font-lock-face (:foreground "firebrick"))) "AUR helper"]) "https://aur.archlinux.org/packages/paru" nil nil (:foreground "firebrick"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}
