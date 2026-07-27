use expect_test::expect;

use super::{
    assert_all_the_icons_gnus_autoload_parity, assert_all_the_icons_gnus_parity,
    assert_all_the_icons_gnus_with_prelude_parity,
};

#[test]
fn exact_release_descriptor_and_real_installed_dependencies_are_stable() {
    let elisp_form = r##"
(let* ((descriptor
        (cadr (assq 'all-the-icons-gnus package-alist)))
       (dash (cadr (assq 'dash package-alist)))
       (icons (cadr (assq 'all-the-icons package-alist)))
       (extras (package-desc-extras descriptor)))
  (list
   (package-desc-name descriptor)
   (package-version-join (package-desc-version descriptor))
   (package-desc-reqs descriptor)
   (alist-get :commit extras)
   (alist-get :url extras)
   (package-version-join (package-desc-version dash))
   (package-version-join (package-desc-version icons))
   (package-installed-p 'dash '(2 12 0))
   (package-installed-p 'all-the-icons '(3 1 0))
   (featurep 'gnus)
   (featurep 'dash)
   (featurep 'all-the-icons)
   (featurep 'all-the-icons-gnus)
   (mapcar
    (lambda (library)
      (file-name-nondirectory (locate-library library)))
    '("all-the-icons-gnus" "dash" "all-the-icons" "gnus"))))
"##;
    let expect = expect![[
        r#"OK (all-the-icons-gnus "20180511.654" ((emacs (24 4)) (dash (2 12 0)) (all-the-icons (3 1 0))) "27f78996da0725943bcfb2d18038e6f7bddfa9c7" "https://github.com/nlamirault/all-the-icons-gnus" "20260221.1346" "20250527.927" t t t t t t ("all-the-icons-gnus.el" "dash.el" "all-the-icons.el" "gnus.el"))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn complete_function_and_macro_signatures_docs_and_interactive_contracts_are_stable() {
    let elisp_form = r##"
(mapcar
 (lambda (function)
   (let ((documentation (documentation function)))
     (list
      function
      (macrop function)
      (help-function-arglist function t)
      (interactive-form function)
      (and documentation t)
      (and documentation
           (secure-hash 'sha256 documentation)))))
 '(all-the-icons-gnus--pretty-gnus
   all-the-icons-gnus--add-faces
   all-the-icons-gnus--set-format
   all-the-icons-gnus-setup))
"##;
    let expect = expect![[
        r#"OK ((all-the-icons-gnus--pretty-gnus t (word icon props) nil t "025c1a0480161a55d2569261915800af5fb3b85874c1564ef2b7acfd69496c41") (all-the-icons-gnus--add-faces nil nil (interactive nil) t "a7dc40aee8314ce9c0de0885a1d51d4314b8434069cc22de94ea6099002e99b5") (all-the-icons-gnus--set-format nil nil nil nil nil) (all-the-icons-gnus-setup nil nil nil t "56775e20042162f549a888b4df2a902eb268a86b54b3bdd3e6d7f99169f45723"))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn exact_pretty_article_mapping_inventory_pins_regex_icons_and_face_properties() {
    let elisp_form = r##"
(mapcar
 (lambda (entry)
   (let ((icon (cadr entry)))
     (list
      (car entry)
      icon
      (substring-no-properties icon)
      (get-text-property 0 'face icon)
      (get-text-property 0 'display icon)
      (caddr entry))))
 pretty-gnus-article-alist)
"##;
    let expect = expect![[
        r##"OK (("\\<\\(X-PGP-Fingerprint:  : \\)" #("" 0 1 (face #1=(:family #2="FontAwesome" :height 1.2) font-lock-face #1# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(X-mailer:  : \\)" #("" 0 1 (face #3=(:family #2# :height 1.2) font-lock-face #3# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(User-Agent:  : \\)" #("" 0 1 (face #4=(:family #2# :height 1.2) font-lock-face #4# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(Content-Type:  : \\)" #("" 0 1 (face #5=(:family #2# :height 1.2) font-lock-face #5# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(Organization:  : \\)" #("" 0 1 (face #6=(:family #2# :height 1.2) font-lock-face #6# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(Date:  : \\)" #("" 0 1 (face #7=(:family #2# :height 1.2) font-lock-face #7# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(Reply-To:  : \\)" #("" 0 1 (face #8=(:family #2# :height 1.2) font-lock-face #8# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(CC:  : \\)" #("" 0 1 (face #9=(:family "github-octicons" :height 1.2) font-lock-face #9# display (raise -0.24) rear-nonsticky t)) "" (:family "github-octicons" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(To:  : \\)" #("" 0 1 (face #10=(:family #2# :height 1.2) font-lock-face #10# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(Subject:  : \\)" #("" 0 1 (face #11=(:family #2# :height 1.2) font-lock-face #11# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")) ("\\<\\(From:  : \\)" #("" 0 1 (face #12=(:family #2# :height 1.2) font-lock-face #12# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (raise -0.24) (:foreground "#375E97")))"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn source_initialization_replaces_preexisting_article_mapping_and_loads_required_features() {
    let prelude = r##"
(setq pretty-gnus-article-alist
      '(("sentinel-regexp" "sentinel-icon"
         (:foreground "sentinel"))))
"##;
    let elisp_form = r##"
(list
 (length pretty-gnus-article-alist)
 (assoc "sentinel-regexp" pretty-gnus-article-alist)
 (featurep 'gnus)
 (featurep 'dash)
 (featurep 'all-the-icons)
 (featurep 'all-the-icons-gnus))
"##;
    let expect = expect!["OK (11 nil t t t t)"];
    assert_all_the_icons_gnus_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_only_setup_without_loading_source_or_gnus() {
    let elisp_form = r##"
(mapcar
 (lambda (symbol)
   (let ((definition
          (and (fboundp symbol)
               (symbol-function symbol))))
     (list
      symbol
      (and (autoloadp definition) t)
      (and (autoloadp definition) (nth 1 definition))
      (and (autoloadp definition) (nth 3 definition))
      (and (autoloadp definition) (nth 4 definition)))))
 '(all-the-icons-gnus-setup
   all-the-icons-gnus--add-faces
   all-the-icons-gnus--set-format
   all-the-icons-gnus--pretty-gnus))
"##;
    let expect = expect![[
        r#"OK ((all-the-icons-gnus-setup t "all-the-icons-gnus" nil nil) (all-the-icons-gnus--add-faces nil nil nil nil) (all-the-icons-gnus--set-format nil nil nil nil) (all-the-icons-gnus--pretty-gnus nil nil nil nil))"#
    ]];
    assert_all_the_icons_gnus_autoload_parity(elisp_form, expect);
}

#[test]
fn macroexpansion_and_custom_mapping_use_real_icon_dependency_and_front_insertion() {
    let elisp_form = r##"
(let ((before (length pretty-gnus-article-alist))
      (expansion
       (macroexpand-1
        '(all-the-icons-gnus--pretty-gnus
          "List-Id: "
          (all-the-icons-faicon "list")
          (:foreground "#123456")))))
  (all-the-icons-gnus--pretty-gnus
   "List-Id: "
   (all-the-icons-faicon "list")
   (:foreground "#123456"))
  (let* ((entry (car pretty-gnus-article-alist))
         (icon (cadr entry)))
    (list
     before
     expansion
     (length pretty-gnus-article-alist)
     (car entry)
     icon
     (substring-no-properties icon)
     (get-text-property 0 'face icon)
     (caddr entry))))
"##;
    let expect = expect![[
        r##"OK (11 (add-to-list 'pretty-gnus-article-alist (list (rx bow (group "List-Id: " " : ")) (all-the-icons-faicon "list") '(:foreground "#123456"))) 12 "\\<\\(List-Id:  : \\)" #("" 0 1 (face #1=(:family "FontAwesome" :height 1.2) font-lock-face #1# display (raise -0.24) rear-nonsticky t)) "" (:family "FontAwesome" :height 1.2) (:foreground "#123456"))"##
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}
