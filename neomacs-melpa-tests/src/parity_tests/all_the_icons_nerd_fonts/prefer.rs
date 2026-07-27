use expect_test::expect;

use super::assert_all_the_icons_nerd_fonts_parity;

#[test]
fn all_the_icons_nerd_fonts_prefer_rewrites_override_and_family_entries_in_place() {
    let elisp_form = r##"(let ((fixture
                '((rust all-the-icons-alltheicon "rust" :face red)
                  (generic all-the-icons-faicon "address-book")
                  (github all-the-icons-faicon "github" :height 1.2)
                  (untouched all-the-icons-fileicon "unknown")))
               (all-the-icons-nerd-fonts-advise-all-the-icons-functions
                nil))
         (list
          (all-the-icons-nerd-fonts-prefer '(fixture))
          fixture
          all-the-icons-nerd-fonts--advice-enabled
          all-the-icons-nerd-fonts--override-map))"##;
    let expect = expect![[
        r#"OK (t ((rust all-the-icons-alltheicon "rust" :face red) (generic all-the-icons-faicon "address-book") (github all-the-icons-faicon "github" :height 1.2) (untouched all-the-icons-fileicon "unknown")) nil nil)"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_prefer_handles_unbound_and_empty_requested_variables() {
    let elisp_form = r##"(let ((empty nil)
               (all-the-icons-nerd-fonts-advise-all-the-icons-functions
                nil))
         (list
          (all-the-icons-nerd-fonts-prefer
           '(missing-fixture empty))
          (boundp 'missing-fixture)
          empty))"##;
    let expect = expect!["OK (t nil nil)"];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_prefer_is_idempotent_for_rewritten_associations() {
    let elisp_form = r##"(let ((fixture
                '((rust all-the-icons-alltheicon "rust")
                  (github all-the-icons-faicon "github")
                  (material all-the-icons-material "star")))
               (all-the-icons-nerd-fonts-advise-all-the-icons-functions
                nil))
         (all-the-icons-nerd-fonts-prefer '(fixture))
         (let ((once (copy-tree fixture)))
           (all-the-icons-nerd-fonts-prefer '(fixture))
           (list once fixture (equal once fixture))))"##;
    let expect = expect![[
        r#"OK (((rust all-the-icons-alltheicon "rust") (github all-the-icons-faicon "github") (material all-the-icons-material "star")) ((rust all-the-icons-alltheicon "rust") (github all-the-icons-faicon "github") (material all-the-icons-material "star")) t)"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_advice_install_and_remove_cover_every_configured_family() {
    let elisp_form = r##"(let (added removed)
         (cl-letf
             (((symbol-function 'advice-add)
               (lambda (symbol where function &optional props)
                 (push
                  (list symbol where
                        (functionp function) props)
                  added)))
              ((symbol-function 'advice-remove)
               (lambda (symbol function)
                 (push (list symbol function) removed))))
           (all-the-icons-nerd-fonts--install-advice)
           (all-the-icons-nerd-fonts--remove-advice)
           (list (nreverse added)
                 (nreverse removed))))"##;
    let expect = expect![
        "OK (((all-the-icons-material :around t #1=((name . all-the-icons-nerd-fonts))) (all-the-icons-faicon :around t #1#) (all-the-icons-octicon :around t #1#) (all-the-icons-wicon :around t #1#)) ((all-the-icons-material all-the-icons-nerd-fonts) (all-the-icons-faicon all-the-icons-nerd-fonts) (all-the-icons-octicon all-the-icons-nerd-fonts) (all-the-icons-wicon all-the-icons-nerd-fonts)))"
    ];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_prefer_respects_advice_configuration_switch() {
    let elisp_form = r##"(mapcar
         (lambda (enabled)
           (let ((all-the-icons-nerd-fonts-advise-all-the-icons-functions
                  enabled)
                 (all-the-icons-nerd-fonts--override-map nil)
                 (all-the-icons-nerd-fonts--advice-enabled nil)
                 calls)
             (cl-letf
                 (((symbol-function
                    'all-the-icons-nerd-fonts--build-override-map)
                   (lambda ()
                     (push 'build calls)
                     'map))
                  ((symbol-function
                    'all-the-icons-nerd-fonts--install-advice)
                   (lambda ()
                     (push 'install calls))))
               (list
                enabled
                (all-the-icons-nerd-fonts-prefer '())
                all-the-icons-nerd-fonts--override-map
                all-the-icons-nerd-fonts--advice-enabled
                (nreverse calls)))))
         '(nil t))"##;
    let expect = expect!["OK ((nil t nil nil nil) (t t map t (build install)))"];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_check_configs_reports_missing_icons_data_and_variables() {
    let elisp_form = r##"(let ((all-the-icons-nerd-fonts--alist-vars
                '(valid-icons missing-icons
                  skipped-icons no-data-icons
                  entirely-unbound))
               (valid-icons
                '((ok all-the-icons-faicon "github")))
               (missing-icons
                '((bad all-the-icons-faicon
                       "definitely-missing")))
               (skipped-icons
                '((web all-the-icons--web-mode-icon
                       "anything")))
               (no-data-icons
                '((bad all-the-icons-unknown "anything")))
               warnings)
         (cl-letf
             (((symbol-function 'display-warning)
               (lambda (type message &rest arguments)
                 (push
                  (list type message arguments)
                  warnings))))
           (all-the-icons-nerd-fonts--check-configs)
           (nreverse warnings)))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-nerd-fonts "all-the-icons override variable not bound: valid-icons" nil) (all-the-icons-nerd-fonts "all-the-icons override variable not bound: missing-icons" nil) (all-the-icons-nerd-fonts "all-the-icons override variable not bound: skipped-icons" nil) (all-the-icons-nerd-fonts "all-the-icons override variable not bound: no-data-icons" nil) (all-the-icons-nerd-fonts "all-the-icons override variable not bound: entirely-unbound" nil))"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}
