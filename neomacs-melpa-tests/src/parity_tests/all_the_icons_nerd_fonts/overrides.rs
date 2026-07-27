use expect_test::expect;

use super::assert_all_the_icons_nerd_fonts_parity;

#[test]
fn all_the_icons_nerd_fonts_override_map_contains_every_declared_entry() {
    let elisp_form = r##"(let ((map
                (all-the-icons-nerd-fonts--build-override-map)))
         (list
          (hash-table-count map)
          (gethash "all-the-icons-alltheicon-rust" map)
          (gethash "all-the-icons-faicon-github" map)
          (gethash "all-the-icons-fileicon-dockerfile" map)
          (gethash "all-the-icons-material-message" map)
          (gethash "all-the-icons-octicon-file-text" map)
          (gethash "missing-family-missing-icon" map)
          (secure-hash
           'sha256
           (prin1-to-string
            (sort
             (let (entries)
               (maphash
                (lambda (key value)
                  (push (cons key value) entries))
                map)
               entries)
             (lambda (left right)
               (string< (car left) (car right))))))))"##;
    let expect = expect![[
        r#"OK (53 (all-the-icons-nerd-dev . "rust") (all-the-icons-nerd-cod . "github") (all-the-icons-nerd-linux . "docker") (all-the-icons-nerd-md . "message-text") (all-the-icons-nerd-oct . "file") nil "0b871bd0dfec3829fcf50d10a72a4d70ef8d84080b18d48b72d9eb1d54dfcf28")"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_advice_uses_specific_override_before_family_conversion() {
    let elisp_form = r##"(let ((all-the-icons-nerd-fonts--advice-enabled t)
               (all-the-icons-nerd-fonts--override-map
                (all-the-icons-nerd-fonts--build-override-map))
               calls)
         (cl-letf
             (((symbol-function 'all-the-icons-nerd-cod)
               (lambda (&rest arguments)
                 (push (cons 'nerd-cod arguments) calls)
                 "NERD"))
              ((symbol-function 'all-the-icons-faicon)
               (lambda (&rest arguments)
                 (push (cons 'original arguments) calls)
                 "ORIGINAL")))
           (let ((advice
                  (all-the-icons-nerd-fonts--make-advice
                   'all-the-icons-faicon)))
             (list
              (funcall
               advice
               #'all-the-icons-faicon
               "github" :height 1.5 :face 'bold)
              (nreverse calls)))))"##;
    let expect = expect![[r#"OK ("NERD" ((nerd-cod "github" :height 1.5 :face bold)))"#]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_advice_converts_compatible_family_and_normalizes_name() {
    let elisp_form = r##"(let ((all-the-icons-nerd-fonts--advice-enabled t)
               (all-the-icons-nerd-fonts--override-map
                (make-hash-table :test 'equal))
               calls)
         (cl-letf
             (((symbol-function
                'all-the-icons-nerd-fonts--icon-exists-p)
               (lambda (family name)
                 (push (list 'exists family name) calls)
                 t))
              ((symbol-function 'all-the-icons-nerd-md)
               (lambda (&rest arguments)
                 (push (cons 'nerd-md arguments) calls)
                 "NERD"))
              ((symbol-function 'all-the-icons-material)
               (lambda (&rest arguments)
                 (push (cons 'original arguments) calls)
                 "ORIGINAL")))
           (let ((advice
                  (all-the-icons-nerd-fonts--make-advice
                   'all-the-icons-material)))
             (list
              (funcall
               advice
               #'all-the-icons-material
               "format_align_left" :v-adjust 0.1)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("NERD" ((exists all-the-icons-nerd-md "format-align-left") (nerd-md "format-align-left" :v-adjust 0.1)))"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_advice_falls_back_when_conversion_icon_is_missing() {
    let elisp_form = r##"(let ((all-the-icons-nerd-fonts--advice-enabled t)
               (all-the-icons-nerd-fonts--override-map
                (make-hash-table :test 'equal))
               calls)
         (cl-letf
             (((symbol-function
                'all-the-icons-nerd-fonts--icon-exists-p)
               (lambda (family name)
                 (push (list 'exists family name) calls)
                 nil))
              ((symbol-function 'all-the-icons-material)
               (lambda (&rest arguments)
                 (push (cons 'original arguments) calls)
                 "ORIGINAL")))
           (let ((advice
                  (all-the-icons-nerd-fonts--make-advice
                   'all-the-icons-material)))
             (list
              (funcall
               advice
               #'all-the-icons-material
               "not_real" :face 'warning)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("ORIGINAL" ((exists all-the-icons-nerd-md "not-real") (original "not_real" :face warning)))"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_disabled_advice_calls_original_without_lookup() {
    let elisp_form = r##"(let ((all-the-icons-nerd-fonts--advice-enabled nil)
               lookup-called
               calls)
         (cl-letf
             (((symbol-function 'gethash)
               (lambda (&rest _)
                 (setq lookup-called t)
                 nil))
              ((symbol-function 'all-the-icons-faicon)
               (lambda (&rest arguments)
                 (push arguments calls)
                 "ORIGINAL")))
           (let ((advice
                  (all-the-icons-nerd-fonts--make-advice
                   'all-the-icons-faicon)))
             (list
              (funcall
               advice
               #'all-the-icons-faicon
               "github" :height 2)
              lookup-called
              (nreverse calls)))))"##;
    let expect = expect![[r#"OK ("ORIGINAL" nil (("github" :height 2)))"#]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_real_advice_rewrites_direct_icon_calls_and_unprefer_restores_them() {
    let elisp_form = r##"(let* ((before
                (all-the-icons-faicon "github"))
               preferred unpreferred)
         (unwind-protect
             (progn
               (all-the-icons-nerd-fonts-prefer '())
               (setq preferred
                     (all-the-icons-faicon "github"))
               (all-the-icons-nerd-fonts-unprefer)
               (setq unpreferred
                     (all-the-icons-faicon "github"))
               (list
                (mapcar
                 #'string-to-list
                 (list before preferred unpreferred))
                (mapcar
                 #'all-the-icons-icon-family
                 (list before preferred unpreferred))
                all-the-icons-nerd-fonts--advice-enabled
                (advice-member-p
                 'all-the-icons-nerd-fonts
                 'all-the-icons-faicon)))
           (all-the-icons-nerd-fonts-unprefer)))"##;
    let expect = expect![[
        r#"OK (((61595) (60036) (61595)) ("FontAwesome" "Symbols Nerd Font" "FontAwesome") nil nil)"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}
