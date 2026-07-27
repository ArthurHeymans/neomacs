use expect_test::expect;

use super::{assert_all_the_icons_ivy_autoload_parity, assert_all_the_icons_ivy_parity};

#[test]
fn all_the_icons_ivy_loads_exact_dependencies_and_complete_callable_surface() {
    let elisp_form = r##"(list
         (featurep 'all-the-icons-ivy)
         (featurep 'all-the-icons)
         (featurep 'ivy)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (help-function-arglist symbol t)
                  (commandp symbol)))
          '(all-the-icons-ivy--buffer-propertize
            all-the-icons-ivy--icon-for-mode
            all-the-icons-ivy--buffer-transformer
            all-the-icons-ivy-icon-for-file
            all-the-icons-ivy-file-transformer
            all-the-icons-ivy-buffer-transformer
            all-the-icons-ivy-setup)))"##;
    let expect = expect![
        "OK (t t t ((all-the-icons-ivy--buffer-propertize t (b s) nil) (all-the-icons-ivy--icon-for-mode t (mode) nil) (all-the-icons-ivy--buffer-transformer t (b s) nil) (all-the-icons-ivy-icon-for-file t (s) nil) (all-the-icons-ivy-file-transformer t (s) nil) (all-the-icons-ivy-buffer-transformer t (s) nil) (all-the-icons-ivy-setup t nil nil)))"
    ];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_custom_variables_publish_exact_defaults_types_and_groups() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (default-value symbol)
                 (get symbol 'custom-type)
                 (get symbol 'custom-group)
                 (get symbol 'standard-value)))
         '(all-the-icons-ivy-buffer-commands
           all-the-icons-spacer
           all-the-icons-ivy-family-fallback-for-buffer
           all-the-icons-ivy-name-fallback-for-buffer
           all-the-icons-ivy-file-commands))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-ivy-buffer-commands (ivy-switch-buffer ivy-switch-buffer-other-window counsel-projectile-switch-to-buffer) (repeat function) nil ((funcall #'#[nil ('(ivy-switch-buffer ivy-switch-buffer-other-window counsel-projectile-switch-to-buffer)) #1=(t)]))) (all-the-icons-spacer "\11" string nil ((funcall #'#[nil ("\11") #1#]))) (all-the-icons-ivy-family-fallback-for-buffer all-the-icons-faicon function nil ((funcall #'#[nil ('all-the-icons-faicon) #1#]))) (all-the-icons-ivy-name-fallback-for-buffer "sticky-note-o" string nil ((funcall #'#[nil ("sticky-note-o") #1#]))) (all-the-icons-ivy-file-commands (counsel-find-file counsel-file-jump counsel-recentf counsel-projectile counsel-projectile-find-file counsel-projectile-find-dir counsel-git) (repeat function) nil ((funcall #'#[nil ('(counsel-find-file counsel-file-jump counsel-recentf counsel-projectile counsel-projectile-find-file counsel-projectile-find-dir counsel-git)) #1#]))))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_face_and_groups_keep_their_declared_contract() {
    let elisp_form = r##"(list
         (facep 'all-the-icons-ivy-dir-face)
         (get 'all-the-icons-ivy-dir-face 'face-documentation)
         (get 'all-the-icons-ivy-dir-face 'face-defface-spec)
         (get 'all-the-icons-ivy-dir-face 'custom-group)
         (get 'all-the-icons-ivy 'group-documentation)
         (get 'all-the-icons-ivy 'custom-group)
         (get 'all-the-icons-ivy 'custom-version))"##;
    let expect = expect![[
        r#"OK ([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "Face for the dir icons used in ivy" ((((background dark)) :foreground "white") (((background light)) :foreground "black")) nil "Shows icons while using ivy and counsel." ((all-the-icons-ivy-buffer-commands custom-variable) (all-the-icons-spacer custom-variable) (all-the-icons-ivy-family-fallback-for-buffer custom-variable) (all-the-icons-ivy-name-fallback-for-buffer custom-variable) (all-the-icons-ivy-file-commands custom-variable)) nil)"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_autoload_exposes_setup_without_loading_runtime() {
    let elisp_form = r##"(list
         (featurep 'all-the-icons-ivy)
         (fboundp 'all-the-icons-ivy-setup)
         (autoloadp
          (and (fboundp 'all-the-icons-ivy-setup)
               (symbol-function 'all-the-icons-ivy-setup)))
         (commandp 'all-the-icons-ivy-setup)
         (help-function-arglist 'all-the-icons-ivy-setup t)
         (file-name-nondirectory
          (getenv "NEOMACS_PACKAGE_SOURCE")))"##;
    let expect = expect![[
        r#"OK (nil t t nil "[Arg list not available until function definition is loaded.]" "all-the-icons-ivy-autoloads.el")"#
    ]];
    assert_all_the_icons_ivy_autoload_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_package_descriptor_records_pinned_version_and_dependencies() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq
                            'all-the-icons-ivy
                            package-alist))))
         (list
          (package-version-join
           (package-desc-version description))
          (package-desc-reqs description)
          (file-name-nondirectory
           (directory-file-name
            (package-desc-dir description)))))"##;
    let expect = expect![[
        r#"OK ("20190508.1803" ((emacs (24 4)) (all-the-icons (2 4 0)) (ivy (0 8 0))) "all-the-icons-ivy-20190508.1803")"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}
