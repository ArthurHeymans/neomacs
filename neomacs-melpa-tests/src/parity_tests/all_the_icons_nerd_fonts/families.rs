use expect_test::expect;

use super::assert_all_the_icons_nerd_fonts_parity;

#[test]
fn all_the_icons_nerd_fonts_defines_all_fifteen_family_functions_and_data_variables() {
    let elisp_form = r##"(mapcar
         (lambda (family)
           (let ((data
                  (intern
                   (format
                    "all-the-icons-data/%s-alist"
                    (string-remove-prefix
                     "all-the-icons-" (symbol-name family))))))
             (list family
                   (fboundp family)
                   (help-function-arglist family t)
                   (boundp data)
                   (and (boundp data)
                        (length (symbol-value data))))))
         '(all-the-icons-nerd-iec
           all-the-icons-nerd-pom
           all-the-icons-nerd-oct
           all-the-icons-nerd-pl
           all-the-icons-nerd-ple
           all-the-icons-nerd-fa
           all-the-icons-nerd-fae
           all-the-icons-nerd-weather
           all-the-icons-nerd-seti
           all-the-icons-nerd-custom
           all-the-icons-nerd-dev
           all-the-icons-nerd-cod
           all-the-icons-nerd-linux
           all-the-icons-nerd-mdi
           all-the-icons-nerd-md))"##;
    let expect = expect![
        "OK ((all-the-icons-nerd-iec t #1=(icon-name &rest args) t 5) (all-the-icons-nerd-pom t #1# t 11) (all-the-icons-nerd-oct t #1# t 310) (all-the-icons-nerd-pl t #1# t 9) (all-the-icons-nerd-ple t #1# t 34) (all-the-icons-nerd-fa t #1# t 1817) (all-the-icons-nerd-fae t #1# t 170) (all-the-icons-nerd-weather t #1# t 228) (all-the-icons-nerd-seti t #1# t 167) (all-the-icons-nerd-custom t #1# t 42) (all-the-icons-nerd-dev t #1# t 508) (all-the-icons-nerd-cod t #1# t 438) (all-the-icons-nerd-linux t #1# t 130) (all-the-icons-nerd-mdi t #1# t 0) (all-the-icons-nerd-md t #1# t 6880))"
    ];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_family_data_matches_exact_filtered_nerd_icon_corpus() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let* ((family (nth 0 spec))
                  (source (symbol-value (nth 1 spec)))
                  (prefix (nth 2 spec))
                  (data
                   (all-the-icons-nerd-fonts--get-nerd-data-alist
                    family))
                  (expected
                   (cl-loop
                    for (name . icon) in source
                    when (string-prefix-p prefix name)
                    collect
                    (cons
                     (string-replace
                      "_" "-"
                      (substring name (length prefix)))
                     icon))))
             (list family
                   (equal data expected)
                   (length data)
                   (secure-hash
                    'sha256
                    (prin1-to-string data)))))
         '((all-the-icons-nerd-iec
            nerd-icons/ipsicon-alist "nf-iec-")
           (all-the-icons-nerd-pom
            nerd-icons/pomicon-alist "nf-pom-")
           (all-the-icons-nerd-oct
            nerd-icons/octicon-alist "nf-oct-")
           (all-the-icons-nerd-pl
            nerd-icons/powerline-alist "nf-pl-")
           (all-the-icons-nerd-ple
            nerd-icons/powerline-alist "nf-ple-")
           (all-the-icons-nerd-fa
            nerd-icons/faicon-alist "nf-fa-")
           (all-the-icons-nerd-fae
            nerd-icons/faicon-alist "nf-fae-")
           (all-the-icons-nerd-weather
            nerd-icons/wicon-alist "nf-weather-")
           (all-the-icons-nerd-seti
            nerd-icons/sucicon-alist "nf-seti-")
           (all-the-icons-nerd-custom
            nerd-icons/sucicon-alist "nf-custom-")
           (all-the-icons-nerd-dev
            nerd-icons/devicon-alist "nf-dev-")
           (all-the-icons-nerd-cod
            nerd-icons/codicon-alist "nf-cod-")
           (all-the-icons-nerd-linux
            nerd-icons/flicon-alist "nf-linux-")
           (all-the-icons-nerd-mdi
            nerd-icons/mdicon-alist "nf-mdi-")
           (all-the-icons-nerd-md
            nerd-icons/mdicon-alist "nf-md-")))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-nerd-iec t 5 "8b4b0189c20b619a83ca6739f4326fb61d66b7d627057345a2c02c5331bee698") (all-the-icons-nerd-pom t 11 "337a9d2c8e44b3a98d82253a63ea16eda92ad41c916a0baec29c6efa8d3f2776") (all-the-icons-nerd-oct t 310 "35959213b753519d42fbac8281bca477d8779dab415a83c04b4a455771d7ec7f") (all-the-icons-nerd-pl t 9 "3fcc83a5f4ea398e20b5f59e9a1b37c463b00beeda40af31c29ae9fad13a2959") (all-the-icons-nerd-ple t 34 "1755ffd466a0b3cd87b3ddd17d154cced4b57576b9ef683f61ec8c06c005fd6a") (all-the-icons-nerd-fa t 1817 "f0120fa0207435c74670405d2cec3b3dc40adf2cb472907a0a698a51a486f664") (all-the-icons-nerd-fae t 170 "dffb50d716e71d514835d3858eee41536ef21949f83faca9fbf5843ee727072a") (all-the-icons-nerd-weather t 228 "7b7ee967d5e335384a327b8e9b7b7358a43efa33b968f6ba2c7b3221792b672b") (all-the-icons-nerd-seti t 167 "a49e370767266df4947f9daa390838d22b82ca82c48a1a167098160148e67d15") (all-the-icons-nerd-custom t 42 "9648ba2cddf913ac8b6466624e2e0428d244d18e16c353a941401e8f35b2ddfc") (all-the-icons-nerd-dev t 508 "7df9c65cb8e93223d22a8a07964eb67a096da1acac1701748eeb0723adb2c564") (all-the-icons-nerd-cod t 438 "fcfee2465110866f4ff1fff8d740bbb54b7a717f2ae9683f9a902acd0b404295") (all-the-icons-nerd-linux t 130 "dd2d5ebb76ef60e99533ac2a38e4d536eb7c887052ee48addfd21acb83f6be3f") (all-the-icons-nerd-mdi t 0 "5da3a4c7f117944275b4c8629c4916403625d5a4a6573a01ecb03f0e9d2edbe6") (all-the-icons-nerd-md t 6880 "d739c4d52aeeb4c54c109a1aa4405c019ca8f94c2cf7c1a3f64a6602a9d06918"))"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_family_data_removes_prefixes_and_normalizes_underscores() {
    let elisp_form = r##"(mapcar
         (lambda (family)
           (let* ((data
                   (all-the-icons-nerd-fonts--get-nerd-data-alist
                    family))
                  (names (mapcar #'car data)))
             (list
              family
              (car data)
              (car (last data))
              (seq-some
               (lambda (name)
                 (string-match-p "_" name))
               names)
              (seq-some
               (lambda (name)
                 (string-prefix-p "nf-" name))
               names))))
         '(all-the-icons-nerd-fa
           all-the-icons-nerd-md
           all-the-icons-nerd-cod
           all-the-icons-nerd-dev))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-nerd-fa ("500px" . "") ("zhihu" . "") nil nil) (all-the-icons-nerd-md ("ab-testing" . "󰇉") ("zodiac-virgo" . "󰪈") nil nil) (all-the-icons-nerd-cod ("account" . "") ("zoom-out" . "") nil nil) (all-the-icons-nerd-dev ("aarch64" . "") ("zig" . "") nil nil))"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_render_representative_icons_with_real_text_properties() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let* ((family (car spec))
                  (name (cadr spec))
                  (icon (funcall family name)))
             (list family name
                   (string-to-list icon)
                   (all-the-icons-icon-family icon)
                   (text-properties-at 0 icon))))
         '((all-the-icons-nerd-fa "github")
           (all-the-icons-nerd-md "language-rust")
           (all-the-icons-nerd-cod "terminal")
           (all-the-icons-nerd-dev "python")
           (all-the-icons-nerd-oct "file")
           (all-the-icons-nerd-weather "day-sunny")))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-nerd-fa "github" (61595) "Symbols Nerd Font" (face #1=(:family "Symbols Nerd Font" :height 1.2) font-lock-face #1# display (raise -0.24) rear-nonsticky t)) (all-the-icons-nerd-md "language-rust" (988695) "Symbols Nerd Font" (face #2=(:family "Symbols Nerd Font" :height 1.2) font-lock-face #2# display (raise -0.24) rear-nonsticky t)) (all-the-icons-nerd-cod "terminal" (60037) "Symbols Nerd Font" (face #3=(:family "Symbols Nerd Font" :height 1.2) font-lock-face #3# display (raise -0.24) rear-nonsticky t)) (all-the-icons-nerd-dev "python" (59196) "Symbols Nerd Font" (face #4=(:family "Symbols Nerd Font" :height 1.2) font-lock-face #4# display (raise -0.24) rear-nonsticky t)) (all-the-icons-nerd-oct "file" (62629) "Symbols Nerd Font" (face #5=(:family "Symbols Nerd Font" :height 1.2) font-lock-face #5# display (raise -0.24) rear-nonsticky t)) (all-the-icons-nerd-weather "day-sunny" (58125) "Symbols Nerd Font" (face #6=(:family "Symbols Nerd Font" :height 1.2) font-lock-face #6# display (raise -0.24) rear-nonsticky t)))"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_family_renderer_honors_face_height_adjust_and_flip_arguments() {
    let elisp_form = r##"(let ((icon
                (all-the-icons-nerd-fa
                 "github"
                 :face 'font-lock-keyword-face
                 :height 1.75
                 :v-adjust -0.2
                 :flip 'horizontal)))
         (list
          (string-to-list icon)
          (all-the-icons-icon-family icon)
          (text-properties-at 0 icon)
          (get-text-property 0 'display icon)))"##;
    let expect = expect![[
        r#"OK ((61595) "Symbols Nerd Font" (face #1=(:family "Symbols Nerd Font" :height 2.1 :inherit font-lock-keyword-face) font-lock-face #1# display #2=(raise -0.24) rear-nonsticky t) #2#)"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_data_lookup_and_existence_handle_known_and_unknown_inputs() {
    let elisp_form = r##"(list
         (all-the-icons-nerd-fonts--icon-exists-p
          'all-the-icons-nerd-fa "github")
         (all-the-icons-nerd-fonts--icon-exists-p
          'all-the-icons-nerd-md "language-rust")
         (all-the-icons-nerd-fonts--icon-exists-p
          'all-the-icons-nerd-fa "definitely-missing")
         (all-the-icons-nerd-fonts--icon-exists-p
          'all-the-icons-nerd-missing "github")
         (all-the-icons-nerd-fonts--get-nerd-data-alist
          'unrelated-family))"##;
    let expect = expect![[r#"OK (("github" . "") ("language-rust" . "󱘗") nil nil nil)"#]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}
