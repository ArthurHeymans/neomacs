use expect_test::expect;

use super::assert_anti_zenburn_theme_parity;

#[test]
fn anti_zenburn_theme_all_variable_settings_match_exact_palette_contracts() {
    let elisp_form = r##"(let ((settings
                    (get
                     'anti-zenburn
                     'theme-settings)))
         (mapcar
          (lambda (name)
            (let ((setting
                   (seq-find
                    (lambda (candidate)
                      (and
                       (eq
                        (car candidate)
                        'theme-value)
                       (eq
                        (cadr candidate)
                        name)))
                    settings)))
              (list
               name
               (copy-tree
                (nth 3 setting)))))
          '(ansi-color-names-vector
            company-quickhelp-color-background
            company-quickhelp-color-foreground
            fci-rule-color
            nrepl-message-colors
            pdf-view-midnight-colors
            vc-annotate-color-map
            vc-annotate-very-old-color
            vc-annotate-background)))"##;
    let expect = expect![[
        r##"OK ((ansi-color-names-vector ["#c0c0c0" "#336c6c" "#806080" "#0f2050" "#732f2c" "#23733c" "#6c1f1c" "#232333"]) (company-quickhelp-color-background "#b0b0b0") (company-quickhelp-color-foreground "#232333") (fci-rule-color "#c7c7c7") (nrepl-message-colors '("#336c6c" "#205070" "#0f2050" "#806080" "#401440" "#6c1f1c" "#6b400c" "#23733c")) (pdf-view-midnight-colors '("#232333" . "#c7c7c7")) (vc-annotate-color-map '((20 . "#437c7c") (40 . "#336c6c") (60 . "#205070") (80 . "#2f4070") (100 . "#1f3060") (120 . "#0f2050") (140 . "#a080a0") (160 . "#806080") (180 . "#704d70") (200 . "#603a60") (220 . "#502750") (240 . "#401440") (260 . "#6c1f1c") (280 . "#935f5c") (300 . "#834744") (320 . "#732f2c") (340 . "#6b400c") (360 . "#23733c"))) (vc-annotate-very-old-color "#23733c") (vc-annotate-background "#d4d4d4"))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_enabling_applies_all_values_and_disabling_restores_user_state() {
    let elisp_form = r##"(progn
         (defvar ansi-color-names-vector)
         (defvar company-quickhelp-color-background)
         (defvar company-quickhelp-color-foreground)
         (defvar fci-rule-color)
         (defvar nrepl-message-colors)
         (defvar pdf-view-midnight-colors)
         (defvar vc-annotate-color-map)
         (defvar vc-annotate-very-old-color)
         (defvar vc-annotate-background)
         (setq ansi-color-names-vector
               ["u0" "u1" "u2" "u3"
                "u4" "u5" "u6" "u7"]
               company-quickhelp-color-background
               "user-background"
               company-quickhelp-color-foreground
               "user-foreground"
               fci-rule-color
               "user-rule"
               nrepl-message-colors
               '("user-message")
               pdf-view-midnight-colors
               '("user-fg" . "user-bg")
               vc-annotate-color-map
               '((1 . "user-map"))
               vc-annotate-very-old-color
               "user-old"
               vc-annotate-background
               "user-vc-background")
         (let (during after)
         (unwind-protect
             (progn
               (load-theme
                'anti-zenburn
                t)
               (setq during
                     (list
                      (copy-sequence
                       ansi-color-names-vector)
                      company-quickhelp-color-background
                      company-quickhelp-color-foreground
                      fci-rule-color
                      (copy-tree
                       nrepl-message-colors)
                      (copy-tree
                       pdf-view-midnight-colors)
                      (copy-tree
                       vc-annotate-color-map)
                      vc-annotate-very-old-color
                      vc-annotate-background))
               (disable-theme
                'anti-zenburn)
               (setq after
                     (list
                      (copy-sequence
                       ansi-color-names-vector)
                      company-quickhelp-color-background
                      company-quickhelp-color-foreground
                      fci-rule-color
                      (copy-tree
                       nrepl-message-colors)
                      (copy-tree
                       pdf-view-midnight-colors)
                      (copy-tree
                       vc-annotate-color-map)
                      vc-annotate-very-old-color
                      vc-annotate-background)))
           (when
               (custom-theme-enabled-p
                'anti-zenburn)
             (disable-theme
              'anti-zenburn)))
           (list during after)))"##;
    let expect = expect![[
        r##"OK ((["#c0c0c0" "#336c6c" "#806080" "#0f2050" "#732f2c" "#23733c" "#6c1f1c" "#232333"] "#b0b0b0" "#232333" "#c7c7c7" ("#336c6c" "#205070" "#0f2050" "#806080" "#401440" "#6c1f1c" "#6b400c" "#23733c") ("#232333" . "#c7c7c7") ((20 . "#437c7c") (40 . "#336c6c") (60 . "#205070") (80 . "#2f4070") (100 . "#1f3060") (120 . "#0f2050") (140 . "#a080a0") (160 . "#806080") (180 . "#704d70") (200 . "#603a60") (220 . "#502750") (240 . "#401440") (260 . "#6c1f1c") (280 . "#935f5c") (300 . "#834744") (320 . "#732f2c") (340 . "#6b400c") (360 . "#23733c")) "#23733c" "#d4d4d4") (["u0" "u1" "u2" "u3" "u4" "u5" "u6" "u7"] "user-background" "user-foreground" "user-rule" ("user-message") ("user-fg" . "user-bg") ((1 . "user-map")) "user-old" "user-vc-background"))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_vc_annotation_palette_selects_real_age_buckets() {
    let elisp_form = r##"(progn
         (require 'vc-annotate)
         (unwind-protect
             (progn
               (load-theme
                'anti-zenburn
                t)
               (list
                (vc-annotate-oldest-in-map
                 vc-annotate-color-map)
                (mapcar
                 (lambda (age)
                   (cons
                    age
                    (cdr
                     (or
                      (vc-annotate-compcar
                       age
                       vc-annotate-color-map)
                      (cons
                       nil
                       vc-annotate-very-old-color)))))
                 '(0 19.99 20 20.01
                   79.99 80 80.01
                   199.99 200 200.01
                   359.99 360 360.01
                   720))))
           (disable-theme
            'anti-zenburn)))"##;
    let expect = expect![[
        r##"OK (360 ((0 . "#437c7c") (19.99 . "#437c7c") (20 . "#437c7c") (20.01 . "#336c6c") (79.99 . "#2f4070") (80 . "#2f4070") (80.01 . "#1f3060") (199.99 . "#603a60") (200 . "#603a60") (200.01 . "#502750") (359.99 . "#23733c") (360 . "#23733c") (360.01 . "#23733c") (720 . "#23733c")))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_obsolete_ansi_palette_and_modern_face_rendering_both_remain_observable() {
    let elisp_form = r##"(progn
         (require 'ansi-color)
         (unwind-protect
             (progn
               (load-theme
                'anti-zenburn
                t)
               (let* ((rendered
                       (ansi-color-apply
                        (concat
                         "\e[31mfailed\e[0m "
                         "\e[32msucceeded\e[0m "
                         "\e[1;34mprimary\e[0m "
                         "\e[35;47mcombined\e[0m")))
                      (tokens
                       '("failed"
                         "succeeded"
                         "primary"
                         "combined")))
                 (list
                  (copy-sequence
                   ansi-color-names-vector)
                  rendered
                  (mapcar
                   (lambda (token)
                     (let* ((start
                             (string-match
                              token
                              rendered))
                            (face
                             (get-text-property
                              start
                              'font-lock-face
                              rendered)))
                       (list
                        token
                        (copy-tree face))))
                   tokens))))
           (disable-theme
            'anti-zenburn)))"##;
    let expect = expect![[
        r##"OK (["#c0c0c0" "#336c6c" "#806080" "#0f2050" "#732f2c" "#23733c" "#6c1f1c" "#232333"] #("failed succeeded primary combined" 0 6 (font-lock-face (:foreground "red3")) 7 16 (font-lock-face (:foreground "green3")) 17 24 (font-lock-face (ansi-color-bold (:foreground "blue2"))) 25 33 (font-lock-face ((:background "gray90") (:foreground "magenta3")))) (("failed" (:foreground "red3")) ("succeeded" (:foreground "green3")) ("primary" (ansi-color-bold (:foreground "blue2"))) ("combined" ((:background "gray90") (:foreground "magenta3")))))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}
