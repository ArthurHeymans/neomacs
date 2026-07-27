use expect_test::expect;

use super::{
    assert_all_the_icons_nerd_fonts_autoload_parity, assert_all_the_icons_nerd_fonts_parity,
};

#[test]
fn all_the_icons_nerd_fonts_loads_exact_dependencies_and_callable_surface() {
    let elisp_form = r##"(list
         (featurep 'all-the-icons-nerd-fonts)
         (featurep 'all-the-icons)
         (featurep 'nerd-icons-data)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (macrop symbol)
                  (help-function-arglist symbol t)
                  (commandp symbol)))
          '(all-the-icons-nerd-fonts--define-family
            all-the-icons-nerd-fonts--build-override-map
            all-the-icons-nerd-fonts--get-nerd-data-alist
            all-the-icons-nerd-fonts--icon-exists-p
            all-the-icons-nerd-fonts--make-advice
            all-the-icons-nerd-fonts--install-advice
            all-the-icons-nerd-fonts--remove-advice
            all-the-icons-nerd-fonts-prefer
            all-the-icons-nerd-fonts-unprefer
            all-the-icons-nerd-fonts--check-configs)))"##;
    let expect = expect![
        "OK (t t t ((all-the-icons-nerd-fonts--define-family t t (family data-alist prefix) nil) (all-the-icons-nerd-fonts--build-override-map t nil nil nil) (all-the-icons-nerd-fonts--get-nerd-data-alist t nil (nerd-family) nil) (all-the-icons-nerd-fonts--icon-exists-p t nil (nerd-family icon-name) nil) (all-the-icons-nerd-fonts--make-advice t nil (orig-family) nil) (all-the-icons-nerd-fonts--install-advice t nil nil nil) (all-the-icons-nerd-fonts--remove-advice t nil nil nil) (all-the-icons-nerd-fonts-prefer t nil (&optional list-vars) nil) (all-the-icons-nerd-fonts-unprefer t nil nil t) (all-the-icons-nerd-fonts--check-configs t nil nil nil)))"
    ];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_customs_publish_exact_defaults_types_and_group() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (default-value symbol)
                 (get symbol 'custom-type)
                 (get symbol 'custom-group)
                 (get symbol 'standard-value)))
         '(all-the-icons-nerd-fonts-family
           all-the-icons-nerd-fonts-advise-all-the-icons-functions
           all-the-icons-nerd-fonts-convert-families
           all-the-icons-nerd-fonts-overrides))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-nerd-fonts-family "Symbols Nerd Font" string nil ((funcall #'#[nil ("Symbols Nerd Font") #1=(t)]))) (all-the-icons-nerd-fonts-advise-all-the-icons-functions t boolean nil ((funcall #'#[nil (t) #1#]))) (all-the-icons-nerd-fonts-convert-families ((all-the-icons-material . all-the-icons-nerd-md) (all-the-icons-faicon . all-the-icons-nerd-fa) (all-the-icons-octicon . all-the-icons-nerd-oct) (all-the-icons-wicon . all-the-icons-nerd-weather)) (alist :key-type symbol :value-type symbol) nil ((funcall #'#[nil ('((all-the-icons-material . all-the-icons-nerd-md) (all-the-icons-faicon . all-the-icons-nerd-fa) (all-the-icons-octicon . all-the-icons-nerd-oct) (all-the-icons-wicon . all-the-icons-nerd-weather))) #1#]))) (all-the-icons-nerd-fonts-overrides ((all-the-icons-alltheicon "aws" all-the-icons-nerd-fa "amazon") (all-the-icons-alltheicon "c-line" all-the-icons-nerd-custom "c") (all-the-icons-alltheicon "clojure-line" all-the-icons-nerd-dev "clojure") (all-the-icons-alltheicon "cplusplus-line" all-the-icons-nerd-custom "cpp") (all-the-icons-alltheicon "csharp-line" all-the-icons-nerd-md "language-csharp") (all-the-icons-alltheicon "elixir" all-the-icons-nerd-custom "elixir") (all-the-icons-alltheicon "git" all-the-icons-nerd-md "git") (all-the-icons-alltheicon "go" all-the-icons-nerd-seti "go") (all-the-icons-alltheicon "google-drive" all-the-icons-nerd-md "google-drive") (all-the-icons-alltheicon "gulp" all-the-icons-nerd-seti "gulp") (all-the-icons-alltheicon "haskell" all-the-icons-nerd-seti "haskell") (all-the-icons-alltheicon "html5" all-the-icons-nerd-fa "html5") (all-the-icons-alltheicon "java" all-the-icons-nerd-fae "java") (all-the-icons-alltheicon "javascript" all-the-icons-nerd-seti "javascript") (all-the-icons-alltheicon "nodejs" all-the-icons-nerd-md "nodejs") (all-the-icons-alltheicon "prolog" all-the-icons-nerd-dev "prolog") (all-the-icons-alltheicon "python" all-the-icons-nerd-fae "python") (all-the-icons-alltheicon "react" all-the-icons-nerd-md "react") (all-the-icons-alltheicon "ruby-alt" all-the-icons-nerd-cod "ruby") (all-the-icons-alltheicon "rust" all-the-icons-nerd-dev "rust") (all-the-icons-alltheicon "sass" all-the-icons-nerd-dev "sass") (all-the-icons-alltheicon "scala" all-the-icons-nerd-dev "scala") (all-the-icons-alltheicon "script" all-the-icons-nerd-seti "html") (all-the-icons-alltheicon "swift" all-the-icons-nerd-dev "swift") (all-the-icons-alltheicon "terminal" all-the-icons-nerd-fa "terminal") (all-the-icons-faicon "github" all-the-icons-nerd-cod "github") (all-the-icons-faicon "git" all-the-icons-nerd-md "git") (all-the-icons-faicon "newspaper-o" all-the-icons-nerd-md "newspaper") (all-the-icons-faicon "shitsinbulk" all-the-icons-nerd-fa "shirtsinbulk") (all-the-icons-fileicon "bib" all-the-icons-nerd-fa "book") (all-the-icons-fileicon "cljs" all-the-icons-nerd-dev "clojure") (all-the-icons-fileicon "dockerfile" all-the-icons-nerd-linux "docker") (all-the-icons-fileicon "go" all-the-icons-nerd-seti "go") (all-the-icons-fileicon "gnu" all-the-icons-nerd-dev "gnu") (all-the-icons-fileicon "php" all-the-icons-nerd-dev "php") (all-the-icons-fileicon "racket" all-the-icons-fileicon "lisp") (all-the-icons-fileicon "test-ruby" all-the-icons-nerd-cod "ruby") (all-the-icons-fileicon "tex" all-the-icons-nerd-cod "text-size") (all-the-icons-material "email" all-the-icons-nerd-md "email") (all-the-icons-material "error" all-the-icons-nerd-seti "error") (all-the-icons-material "git" all-the-icons-nerd-md "git") (all-the-icons-material "message" all-the-icons-nerd-md "message-text") (all-the-icons-material "star" all-the-icons-nerd-md "star") (all-the-icons-material "style" all-the-icons-nerd-md "border-style") (all-the-icons-octicon "dashboard" all-the-icons-nerd-cod "dashboard") (all-the-icons-octicon "file-pdf" all-the-icons-nerd-cod "file-pdf") (all-the-icons-octicon "file-symlink-directory" all-the-icons-nerd-cod "file-symlink-directory") (all-the-icons-octicon "file-text" all-the-icons-nerd-oct "file") (all-the-icons-octicon "gist" all-the-icons-nerd-cod "notebook") (all-the-icons-octicon "mail-read" all-the-icons-nerd-cod "mail-read") (all-the-icons-octicon "ruby" all-the-icons-nerd-cod "ruby") (all-the-icons-octicon "message-text" all-the-icons-nerd-md "message-text") (all-the-icons-octicon "settings" all-the-icons-nerd-cod "settings") (all-the-icons-octicon "settings" all-the-icons-nerd-cod "settings")) (list (symbol :tag "Source family") (string :tag "Soruce icon") (symbol :tag "Destination family") (symbol :tag "Destination icon")) nil ((funcall #'#[nil ('((all-the-icons-alltheicon "aws" all-the-icons-nerd-fa "amazon") (all-the-icons-alltheicon "c-line" all-the-icons-nerd-custom "c") (all-the-icons-alltheicon "clojure-line" all-the-icons-nerd-dev "clojure") (all-the-icons-alltheicon "cplusplus-line" all-the-icons-nerd-custom "cpp") (all-the-icons-alltheicon "csharp-line" all-the-icons-nerd-md "language-csharp") (all-the-icons-alltheicon "elixir" all-the-icons-nerd-custom "elixir") (all-the-icons-alltheicon "git" all-the-icons-nerd-md "git") (all-the-icons-alltheicon "go" all-the-icons-nerd-seti "go") (all-the-icons-alltheicon "google-drive" all-the-icons-nerd-md "google-drive") (all-the-icons-alltheicon "gulp" all-the-icons-nerd-seti "gulp") (all-the-icons-alltheicon "haskell" all-the-icons-nerd-seti "haskell") (all-the-icons-alltheicon "html5" all-the-icons-nerd-fa "html5") (all-the-icons-alltheicon "java" all-the-icons-nerd-fae "java") (all-the-icons-alltheicon "javascript" all-the-icons-nerd-seti "javascript") (all-the-icons-alltheicon "nodejs" all-the-icons-nerd-md "nodejs") (all-the-icons-alltheicon "prolog" all-the-icons-nerd-dev "prolog") (all-the-icons-alltheicon "python" all-the-icons-nerd-fae "python") (all-the-icons-alltheicon "react" all-the-icons-nerd-md "react") (all-the-icons-alltheicon "ruby-alt" all-the-icons-nerd-cod "ruby") (all-the-icons-alltheicon "rust" all-the-icons-nerd-dev "rust") (all-the-icons-alltheicon "sass" all-the-icons-nerd-dev "sass") (all-the-icons-alltheicon "scala" all-the-icons-nerd-dev "scala") (all-the-icons-alltheicon "script" all-the-icons-nerd-seti "html") (all-the-icons-alltheicon "swift" all-the-icons-nerd-dev "swift") (all-the-icons-alltheicon "terminal" all-the-icons-nerd-fa "terminal") (all-the-icons-faicon "github" all-the-icons-nerd-cod "github") (all-the-icons-faicon "git" all-the-icons-nerd-md "git") (all-the-icons-faicon "newspaper-o" all-the-icons-nerd-md "newspaper") (all-the-icons-faicon "shitsinbulk" all-the-icons-nerd-fa "shirtsinbulk") (all-the-icons-fileicon "bib" all-the-icons-nerd-fa "book") (all-the-icons-fileicon "cljs" all-the-icons-nerd-dev "clojure") (all-the-icons-fileicon "dockerfile" all-the-icons-nerd-linux "docker") (all-the-icons-fileicon "go" all-the-icons-nerd-seti "go") (all-the-icons-fileicon "gnu" all-the-icons-nerd-dev "gnu") (all-the-icons-fileicon "php" all-the-icons-nerd-dev "php") (all-the-icons-fileicon "racket" all-the-icons-fileicon "lisp") (all-the-icons-fileicon "test-ruby" all-the-icons-nerd-cod "ruby") (all-the-icons-fileicon "tex" all-the-icons-nerd-cod "text-size") (all-the-icons-material "email" all-the-icons-nerd-md "email") (all-the-icons-material "error" all-the-icons-nerd-seti "error") (all-the-icons-material "git" all-the-icons-nerd-md "git") (all-the-icons-material "message" all-the-icons-nerd-md "message-text") (all-the-icons-material "star" all-the-icons-nerd-md "star") (all-the-icons-material "style" all-the-icons-nerd-md "border-style") (all-the-icons-octicon "dashboard" all-the-icons-nerd-cod "dashboard") (all-the-icons-octicon "file-pdf" all-the-icons-nerd-cod "file-pdf") (all-the-icons-octicon "file-symlink-directory" all-the-icons-nerd-cod "file-symlink-directory") (all-the-icons-octicon "file-text" all-the-icons-nerd-oct "file") (all-the-icons-octicon "gist" all-the-icons-nerd-cod "notebook") (all-the-icons-octicon "mail-read" all-the-icons-nerd-cod "mail-read") (all-the-icons-octicon "ruby" all-the-icons-nerd-cod "ruby") (all-the-icons-octicon "message-text" all-the-icons-nerd-md "message-text") (all-the-icons-octicon "settings" all-the-icons-nerd-cod "settings") (all-the-icons-octicon "settings" all-the-icons-nerd-cod "settings"))) #1#]))))"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_constants_capture_complete_rewrite_scope() {
    let elisp_form = r##"(list
         all-the-icons-nerd-fonts--alist-vars
         all-the-icons-nerd-fonts--data-remap-alist
         all-the-icons-nerd-fonts--skip-families
         (length all-the-icons-nerd-fonts-overrides)
         (secure-hash
          'sha256
          (prin1-to-string
           all-the-icons-nerd-fonts-overrides))
         (get
          'all-the-icons-nerd-fonts-convert-icons
          'byte-obsolete-variable))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-dir-icon-alist all-the-icons-dir-icon-overrides all-the-icons-extension-icon-alist all-the-icons-icon-alist all-the-icons-mode-icon-alist all-the-icons-url-alist all-the-icons-weather-icon-alist all-the-icons-web-mode-icon-alist) ((all-the-icons-data/alltheicon-alist . all-the-icons-data/alltheicons-alist) (all-the-icons-data/fileicon-alist . all-the-icons-data/file-icon-alist) (all-the-icons-data/wicon-alist . all-the-icons-data/weather-icons-alist)) (all-the-icons--web-mode-icon) 54 "8e021c545160b4526d05a9ae3747d5d3f7ea9b8df970295252d0876ccdd5f0d8" ("Use `all-the-icons-nerd-fonts-overrides' instead." nil "0.2"))"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_package_descriptor_records_pin_and_dependencies() {
    let elisp_form = r##"(let ((description
                        (cadr
                         (assq
                          'all-the-icons-nerd-fonts
                          package-alist))))
         (list
          (package-version-join
           (package-desc-version description))
          (package-desc-reqs description)
          (file-name-nondirectory
           (directory-file-name
            (package-desc-dir description)))))"##;
    let expect = expect![[
        r#"OK ("20260614.1246" ((emacs (28 1)) (all-the-icons (5 0)) (nerd-icons (0 0 1))) "all-the-icons-nerd-fonts-20260614.1246")"#
    ]];
    assert_all_the_icons_nerd_fonts_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_nerd_fonts_autoloads_prefer_and_unprefer_without_runtime_load() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp symbol)
            (autoloadp
             (and (fboundp symbol)
                  (symbol-function symbol)))
            (commandp symbol)
            (help-function-arglist symbol t)))
         '(all-the-icons-nerd-fonts-prefer
           all-the-icons-nerd-fonts-unprefer))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-nerd-fonts-prefer t t nil "[Arg list not available until function definition is loaded.]") (all-the-icons-nerd-fonts-unprefer t t t "[Arg list not available until function definition is loaded.]"))"#
    ]];
    assert_all_the_icons_nerd_fonts_autoload_parity(elisp_form, expect);
}
