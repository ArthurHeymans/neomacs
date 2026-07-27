use expect_test::expect;

use super::assert_all_the_icons_parity;

#[test]
fn all_the_icons_file_workflow_honors_regexp_extension_case_default_and_overrides() {
    let elisp_form = r##"(let ((files
                '("package.json" "PACKAGE.JSON" "main.rs"
                  "component-test.jsx" "README.md" ".gitignore"
                  "archive.unknown" "no-extension")))
         (mapcar
          (lambda (file)
            (let ((icon
                   (all-the-icons-icon-for-file
                    file :height 1.75 :v-adjust 0.125
                    :face 'all-the-icons-green)))
              (list file
                    (string-to-list icon)
                    (all-the-icons-icon-family icon)
                    (text-properties-at 0 icon))))
          files))"##;
    let expect = expect![[
        r#"OK (("package.json" (59676) "file-icons" (face #1=(:family "file-icons" :height 2.1 :inherit all-the-icons-green) font-lock-face #1# display (raise 0.15) rear-nonsticky t)) ("PACKAGE.JSON" (59676) "file-icons" (face #2=(:family "file-icons" :height 2.1 :inherit all-the-icons-green) font-lock-face #2# display (raise 0.15) rear-nonsticky t)) ("main.rs" (59692) "all-the-icons" (face #3=(:family "all-the-icons" :height 2.1 :inherit all-the-icons-green) font-lock-face #3# display (raise 0.15) rear-nonsticky t)) ("component-test.jsx" (60007) "file-icons" (face #4=(:family "file-icons" :height 2.1 :inherit all-the-icons-green) font-lock-face #4# display (raise 0.15) rear-nonsticky t)) ("README.md" (61447) "github-octicons" (face #5=(:family "github-octicons" :height 2.1 :inherit all-the-icons-green) font-lock-face #5# display (raise 0.15) rear-nonsticky t)) (".gitignore" (61487) "github-octicons" (face #6=(:family "github-octicons" :height 2.1 :inherit all-the-icons-green) font-lock-face #6# display (raise 0.15) rear-nonsticky t)) ("archive.unknown" (61462) "FontAwesome" (face #7=(:family "FontAwesome" :height 2.1 :inherit all-the-icons-green) font-lock-face #7# display (raise 0.15) rear-nonsticky t)) ("no-extension" (61462) "FontAwesome" (face #8=(:family "FontAwesome" :height 2.1 :inherit all-the-icons-green) font-lock-face #8# display (raise 0.15) rear-nonsticky t)))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_file_family_selection_matches_rendered_icon_family() {
    let elisp_form = r##"(mapcar
         (lambda (file)
           (let ((icon (all-the-icons-icon-for-file file)))
             (list file
                   (all-the-icons-icon-family-for-file file)
                   (all-the-icons-icon-family icon)
                   (string-to-list icon))))
         '("server.py" "Cargo.toml" "logo.svg" "notes.org"
           "Dockerfile" "mystery.bin.nope"))"##;
    let expect = expect![[
        r#"OK (("server.py" "all-the-icons" "all-the-icons" (59688)) ("Cargo.toml" "FontAwesome" "FontAwesome" (61462)) ("logo.svg" "all-the-icons" "all-the-icons" (59651)) ("notes.org" "file-icons" "file-icons" (59671)) ("Dockerfile" "file-icons" "file-icons" (61702)) ("mystery.bin.nope" "FontAwesome" "FontAwesome" (61462)))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_mode_workflow_supports_direct_derived_and_unknown_modes() {
    let elisp_form = r##"(progn
         (put 'all-the-icons-child-rust-mode
              'derived-mode-parent 'rust-mode)
         (mapcar
          (lambda (mode)
            (let ((icon
                   (all-the-icons-icon-for-mode
                    mode :height 1.4 :face
                    'all-the-icons-cyan)))
              (list mode
                    (stringp icon)
                    (if (stringp icon)
                        (string-to-list icon)
                      icon)
                    (and (stringp icon)
                         (text-properties-at 0 icon))
                    (all-the-icons-icon-family-for-mode mode))))
          '(emacs-lisp-mode rust-mode
            all-the-icons-child-rust-mode
            all-the-icons-unknown-mode)))"##;
    let expect = expect![[
        r#"OK ((emacs-lisp-mode t (59686) (face #1=(:family "file-icons" :height 1.68 :inherit all-the-icons-cyan) font-lock-face #1# display (raise -0.12) rear-nonsticky t) "file-icons") (rust-mode t (59692) (face #2=(:family "all-the-icons" :height 1.68 :inherit all-the-icons-cyan) font-lock-face #2# display (raise -0.24) rear-nonsticky t) "all-the-icons") (all-the-icons-child-rust-mode t (59692) (face #3=(:family "all-the-icons" :height 1.68 :inherit all-the-icons-cyan) font-lock-face #3# display (raise -0.24) rear-nonsticky t) nil) (all-the-icons-unknown-mode nil all-the-icons-unknown-mode nil nil))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_web_mode_workflow_selects_each_content_type_and_override() {
    let elisp_form = r##"(mapcar
         (lambda (content-type)
           (setq web-mode-content-type content-type)
           (let* ((icon
                   (all-the-icons--web-mode-icon
                    :height 1.3 :v-adjust 0.2
                    :face 'all-the-icons-pink)))
             (list content-type
                   (string-to-list icon)
                   (all-the-icons--web-mode-icon-family)
                   (text-properties-at 0 icon))))
         '("jsx" "javascript" "json" "xml" "css" "html" nil))"##;
    let expect = expect![[
        r#"OK (("jsx" (61697) "file-icons" (face #1=(:family "file-icons" :height 1.56 :inherit all-the-icons-pink) font-lock-face #1# display (raise 0.24) rear-nonsticky t)) ("javascript" (59654) "all-the-icons" (face #2=(:family "all-the-icons" :height 1.56 :inherit all-the-icons-pink) font-lock-face #2# display (raise 0.24) rear-nonsticky t)) ("json" (59659) "all-the-icons" (face #3=(:family "all-the-icons" :height 1.56 :inherit all-the-icons-pink) font-lock-face #3# display (raise 0.24) rear-nonsticky t)) ("xml" (61897) "FontAwesome" (face #4=(:family "FontAwesome" :height 1.56 :inherit all-the-icons-pink) font-lock-face #4# display (raise 0.24) rear-nonsticky t)) ("css" (59675) "all-the-icons" (face #5=(:family "all-the-icons" :height 1.56 :inherit all-the-icons-pink) font-lock-face #5# display (raise 0.24) rear-nonsticky t)) ("html" (59698) "all-the-icons" (face #6=(:family "all-the-icons" :height 1.56 :inherit all-the-icons-pink) font-lock-face #6# display (raise 0.24) rear-nonsticky t)) (nil (59698) "all-the-icons" (face #7=(:family "all-the-icons" :height 1.56 :inherit all-the-icons-pink) font-lock-face #7# display (raise 0.24) rear-nonsticky t)))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_buffer_workflow_prefers_matching_file_then_falls_back_to_mode() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (setq buffer-file-name "/project/src/main.rs")
           (setq major-mode 'rust-mode)
           (let ((auto-mode-alist '(("\\.rs\\'" . rust-mode))))
             (let ((icon (all-the-icons-icon-for-buffer)))
               (list (all-the-icons-auto-mode-match?)
                     (string-to-list icon)
                     (all-the-icons-icon-family-for-buffer)))))
         (with-temp-buffer
           (setq buffer-file-name "/project/template.txt")
           (setq major-mode 'emacs-lisp-mode)
           (let ((auto-mode-alist '(("\\.txt\\'" . text-mode))))
             (let ((icon (all-the-icons-icon-for-buffer)))
               (list (all-the-icons-auto-mode-match?)
                     (string-to-list icon)
                     (all-the-icons-icon-family-for-buffer)))))
         (with-temp-buffer
           (rename-buffer "notes-without-file" t)
           (setq major-mode 'text-mode)
           (list (all-the-icons-auto-mode-match?)
                 (string-to-list
                  (all-the-icons-icon-for-buffer))
                 (all-the-icons-icon-family-for-buffer))))"##;
    let expect = expect![[
        r#"OK ((t (59692) "all-the-icons") (nil (59686) "file-icons") (nil (61457) "github-octicons"))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_url_workflow_handles_specific_catchall_and_default_hosts() {
    let elisp_form = r##"(mapcar
         (lambda (url)
           (let ((icon
                  (all-the-icons-icon-for-url
                   url :height 1.2 :v-adjust 0
                   :face 'all-the-icons-orange)))
             (list url
                   (string-to-list icon)
                   (all-the-icons-icon-family icon)
                   (text-properties-at 0 icon))))
         '("https://github.com/eval-exec/neomacs"
           "https://stackoverflow.com/questions/1"
           "https://example.net/manual.pdf"
           "https://unknown.invalid/path"))"##;
    let expect = expect![[
        r#"OK (("https://github.com/eval-exec/neomacs" (61450) "github-octicons" (face #1=(:family "github-octicons" :height 1.44 :inherit all-the-icons-orange) font-lock-face #1# display (raise 0.0) rear-nonsticky t)) ("https://stackoverflow.com/questions/1" (61804) "FontAwesome" (face #2=(:family "FontAwesome" :height 1.44 :inherit all-the-icons-orange) font-lock-face #2# display (raise 0.0) rear-nonsticky t)) ("https://example.net/manual.pdf" (61460) "github-octicons" (face #3=(:family "github-octicons" :height 1.44 :inherit all-the-icons-orange) font-lock-face #3# display (raise 0.0) rear-nonsticky t)) ("https://unknown.invalid/path" (61612) "FontAwesome" (face #4=(:family "FontAwesome" :height 1.44 :inherit all-the-icons-orange) font-lock-face #4# display (raise 0.0) rear-nonsticky t)))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_weather_workflow_matches_ordered_patterns_and_preserves_unknown_text() {
    let elisp_form = r##"(mapcar
         (lambda (weather)
           (let ((result
                  (all-the-icons-icon-for-weather weather)))
             (list weather
                   (string-to-list result)
                   (and
                    (get-text-property 0 'face result)
                    (all-the-icons-icon-family result))
                   (text-properties-at 0 result))))
         '("rain and snow" "partly cloudy night"
           "clear night" "not available"
           "alien atmosphere"))"##;
    let expect = expect![[
        r#"OK (("rain and snow" (61463) "Weather Icons" (face #1=(:family "Weather Icons" :height 1.2) font-lock-face #1# display (raise -0.24) rear-nonsticky t)) ("partly cloudy night" (61569) "Weather Icons" (face #2=(:family "Weather Icons" :height 1.2) font-lock-face #2# display (raise -0.24) rear-nonsticky t)) ("clear night" (61486) "Weather Icons" (face #3=(:family "Weather Icons" :height 1.2) font-lock-face #3# display (raise -0.24) rear-nonsticky t)) ("not available" (61563) "Weather Icons" (face #4=(:family "Weather Icons" :height 1.2) font-lock-face #4# display (raise -0.24) rear-nonsticky t)) ("alien atmosphere" (97 108 105 101 110 32 97 116 109 111 115 112 104 101 114 101) nil nil))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_match_helper_and_auto_mode_contract_use_first_matching_entry() {
    let elisp_form = r##"(let ((alist
                '(("foo" . first)
                  ("foobar" . second)
                  ("bar" . third))))
         (list
          (all-the-icons-match-to-alist "xxfoobarxx" alist)
          (all-the-icons-match-to-alist "none" alist)
          (with-temp-buffer
            (rename-buffer "demo.special" t)
            (setq major-mode 'special-mode)
            (let ((auto-mode-alist
                   '(("\\.special\\'" . special-mode))))
              (list
               (all-the-icons-auto-mode-match?)
               (all-the-icons-auto-mode-match?
                "other.unknown"))))))"##;
    let expect = expect!["OK (first nil (t nil))"];
    assert_all_the_icons_parity(elisp_form, expect);
}
