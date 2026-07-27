use expect_test::expect;

use super::{assert_applescript_mode_autoload_parity, assert_applescript_mode_parity};

#[test]
fn applescript_mode_exact_pin_descriptor_dependencies_and_origin_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'applescript-mode
                      package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-reqs descriptor)
          (package-desc-summary descriptor)
          (copy-tree
           (package-desc-extras descriptor))
          (featurep
           'applescript-mode)))"##;
    let expect = expect![[
        r#"OK (applescript-mode "20210802.1715" ((emacs (24 3))) "Major mode for editing AppleScript source." ((:maintainers ("sakito" . "sakito@users.sourceforge.jp")) (:authors ("sakito" . "sakito@users.sourceforge.jp")) (:keywords "languages" "tools") (:revdesc . "00c141bbff46") (:commit . "00c141bbff46c89a96598b605dee05dd1d89f624") (:url . "https://github.com/emacsorphanage/applescript-mode")) t)"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_installed_payload_has_exact_inventory_sizes_and_content_digests() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'applescript-mode
                    package-alist)))
                 (directory
                  (package-desc-dir descriptor))
                 (all-files
                  (sort
                   (mapcar
                    (lambda (path)
                      (file-relative-name
                       path
                       directory))
                    (directory-files-recursively
                     directory
                     ".*"
                     nil))
                   #'string<)))
         (list
          all-files
          (mapcar
           (lambda (relative)
             (let ((path
                    (expand-file-name
                     relative
                     directory)))
               (list
                relative
                (nth
                 7
                 (file-attributes path))
                (secure-hash
                 'sha256
                 path)
                (file-executable-p path))))
           '("applescript-mode-pkg.el"
             "applescript-mode.el"))))"##;
    let expect = expect![[
        r#"OK (("README-elpa" "applescript-mode-autoloads.el" "applescript-mode-pkg.el" "applescript-mode.el" "applescript-mode.elc") (("applescript-mode-pkg.el" 463 "b166247b35ab57f102027907c90ecd646c6fdbc7d796c9a5abb017bbfe176395" nil) ("applescript-mode.el" 16931 "2d945ed1cbe3beecde7d84e77e718f288472790cceee457add9f2086f4b42b3d" nil)))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_callable_surface_has_exact_commands_macros_arglists_and_docs() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((documentation
                  (documentation
                   symbol
                   t)))
             (list
              symbol
              (and
               (fboundp symbol)
               t)
              (and
               (macrop symbol)
               t)
              (commandp symbol)
              (help-function-arglist
               symbol
               t)
              (and
               documentation
               (secure-hash
                'sha256
                documentation)))))
         '(as-font-lock-mode-hook
           as-safe
           as-keep-region-active
           as-point
           applescript-mode
           as-execute-buffer
           as-execute-string
           as-execute-region
           as-execute-code
           as-mode-version
           as-language-version
           as-beginning-of-handler
           as-end-of-handler
           as-goto-initial-line
           as-outline-level
           as-unescape-string
           as-escape-string
           as-sjis-byte-list-escape
           as-string-to-sjis-string-with-escape
           as-decode-string
           as-encode-string
           as-parse-result))"##;
    let expect = expect![[
        r#"OK ((as-font-lock-mode-hook t nil nil nil nil) (as-safe t t nil (&rest body) "c41d98f10d44627c0ec2b08ef0f9249f9fd97866cffb0961aa54e80b7c6daa42") (as-keep-region-active t nil nil nil "094fd1c7ac2344dcd7e5b1a07f309e684808ed14affade87b6584183b4d25641") (as-point t nil nil (position) "8f8f9a7a14366f1118d6c842e546cd6b4ec094c89cadb7769b6bc11c1f346270") (applescript-mode t nil t nil "d28f8d3bd533bde5b1a559ed409466d7bf48be705892064a06635cc10727d79c") (as-execute-buffer t nil t (&optional async) "22c8232b264515f432e06e02b9d51df5464f05c0b5128096dde6d8ef69d3ff48") (as-execute-string t nil t (string &optional async) "ca8be0bd050bf631a5d1d09a7af74a9d305d7641b18cf68f7c67ede3b0d7e093") (as-execute-region t nil t (start end &optional async) "6381ddf7ef2d3d5e77464ec57275296abfe7e8cff41d6ac6893fa8904f65803e") (as-execute-code t nil nil (code) "0dd47c393247e14cde8fbeffe92376b0d0cd44ad7c35ed5e6e40af8cd8f21341") (as-mode-version t nil t nil "2a47906266079f476578558c6ae1747835c482016dd6feccad9ec23d51d74766") (as-language-version t nil t nil "c897919151f52d0bfb9c3983b78b863b902104fbe791c58d11c826cb12ed10de") (as-beginning-of-handler t nil nil (sym) nil) (as-end-of-handler t nil nil (sym) nil) (as-goto-initial-line t nil nil nil nil) (as-outline-level t nil nil nil "8540b3f34749aa30af6ab318122f98f49cb9c4aac9b4cb9d7c4afecab3637611") (as-unescape-string t nil nil (str) "1b043ba1db2bb8cb21db26df1001379cb184fa79635c796123efa6e800e3f045") (as-escape-string t nil nil (str) "ea54145e507597ef757f36310844ac08be735ce089edd93107a7d85fea8961d2") (as-sjis-byte-list-escape t nil nil (lst) nil) (as-string-to-sjis-string-with-escape t nil nil (str) "50821aca4767e00a4d3a3cda5ec5f17f6c84632ff76a4f2c2e0a6994ad74a07f") (as-decode-string t nil nil (str) "d00264279395b3886e7cfbedd43594d1d4fa5b078ca9cce83328c9814a1c39b4") (as-encode-string t nil nil (str) "1cafc883d6b41abb656856b51207ee8cf039f78a979fb7f0b868c30a9b1475f3") (as-parse-result t nil nil (retstr) "ef0a4c38dd3c02acf0ee68ee9e35a8950116215a0e005a9a0752788e0d0bcd11"))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_configuration_constants_faces_and_variable_contracts_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((value
                  (and
                   (boundp symbol)
                   (symbol-value symbol))))
             (list
              symbol
              (cond
               ((eq
                 symbol
                 'applescript-font-lock-keywords)
                (list
                 (length value)
                 (secure-hash
                  'sha256
                  (prin1-to-string value))))
               ((keymapp value)
                :keymap)
               ((syntax-table-p value)
                :syntax-table)
               (t value))
              (get symbol 'custom-type)
              (get symbol 'custom-group)
              (get symbol 'standard-value)
              (and
               (get symbol 'face)
               t)
              (local-variable-if-set-p
               symbol))))
         '(applescript-mode-version
           applescript-mode-help-address
           as-osascript-command
           as-osacompile-command
           as-osascript-command-args
           as-indent-offset
           as-continuation-offset
           as-pseudo-keyword-face
           as-command-face
           applescript-font-lock-keywords
           applescript-mode-abbrev-table
           applescript-mode-hook
           as-mode-map
           as-mode-syntax-table
           as-menu
           as-output-buffer))"##;
    let expect = expect![[
        r#"OK ((applescript-mode-version "$Revision$" nil nil nil nil nil) (applescript-mode-help-address "sakito@users.sourceforge.jp" nil nil nil nil nil) (as-osascript-command "osascript" string nil ("osascript") nil nil) (as-osacompile-command "osacompile" string nil ("osacompile") nil nil) (as-osascript-command-args #1=("-ss") (repeat string) nil ('#1#) nil nil) (as-indent-offset 4 integer nil (4) nil nil) (as-continuation-offset 4 integer nil (4) nil nil) (as-pseudo-keyword-face as-pseudo-keyword-face nil nil nil t nil) (as-command-face as-command-face nil nil nil t nil) (applescript-font-lock-keywords (6 "25b4eebaf9d6dabbf1bd18687a1b64b859e482cf59d9e3941b956caf64f1a58b") nil nil nil nil nil) (applescript-mode-abbrev-table #<obarray n=1> nil nil nil nil nil) (applescript-mode-hook nil nil nil nil nil nil) (as-mode-map :keymap nil nil nil nil nil) (as-mode-syntax-table :syntax-table nil nil nil nil nil) (as-menu :keymap nil nil nil nil nil) (as-output-buffer "*AppleScript Output*" nil nil nil nil t))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_autoloads_register_commands_file_extensions_and_interpreters() {
    let elisp_form = r##"(list
         (featurep
          'applescript-mode)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (and
              (fboundp symbol)
              t)
             (and
              (fboundp symbol)
              (autoloadp
               (symbol-function symbol)))
             (commandp symbol)))
          '(applescript-mode
            as-execute-buffer
            as-execute-string
            as-execute-region
            as-mode-version
            as-language-version))
         (assoc
          "\\.\\(applescript\\|scpt\\)\\'"
          auto-mode-alist)
         (rassq
          'applescript-mode
          auto-mode-alist)
         (assoc
          "osascript"
          interpreter-mode-alist))"##;
    let expect = expect![[
        r#"OK (nil ((applescript-mode t t t) (as-execute-buffer nil nil nil) (as-execute-string nil nil nil) (as-execute-region nil nil nil) (as-mode-version nil nil nil) (as-language-version nil nil nil)) #1=("\\.\\(applescript\\|scpt\\)\\'" . applescript-mode) #1# ("osascript" . applescript-mode))"#
    ]];

    assert_applescript_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_keymap_menu_and_interactive_specs_expose_the_complete_ui_contract() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (key)
            (list
             key
             (lookup-key
              as-mode-map
              (kbd key))))
          '("C-c C-c"
            "C-c C-s"
            "C-c |"
            "C-c ;"
            "C-c :"))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (interactive-form symbol)))
          '(applescript-mode
            as-execute-buffer
            as-execute-string
            as-execute-region
            as-mode-version
            as-language-version))
         (and
          (keymapp as-menu)
          (mapcar
           (lambda (event)
             (lookup-key
              as-mode-map
              (vector
               'menu-bar
               event)))
           '(AppleScript applescript))))"##;
    let expect = expect![[
        r#"OK ((("C-c C-c" as-execute-buffer) ("C-c C-s" as-execute-string) ("C-c |" as-execute-region) ("C-c ;" comment-region) ("C-c :" uncomment-region)) ((applescript-mode (interactive nil)) (as-execute-buffer (interactive "P")) (as-execute-string (interactive "sExecute AppleScript: ")) (as-execute-region (interactive "r\nP")) (as-mode-version (interactive nil)) (as-language-version (interactive nil))) (#2=(keymap "AppleScript" (Comment\ Out\ Region menu-item "Comment Out Region" comment-region :enable (mark)) (Uncomment\ Region menu-item "Uncomment Region" uncomment-region :enable (mark)) (nil . #1=("--")) (Execute\ buffer menu-item "Execute buffer" as-execute-buffer) (Execute\ region menu-item "Execute region" as-execute-region :enable (mark)) (Execute\ string menu-item "Execute string" as-execute-string) (nil-6 . #1#) (Mode\ Version menu-item "Mode Version" as-mode-version) (AppleScript\ Version menu-item "AppleScript Version" as-language-version)) #2#))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_syntax_table_classifies_comments_strings_delimiters_and_operators() {
    let elisp_form = r##"(mapcar
         (lambda (character)
           (let ((entry
                  (aref
                   as-mode-syntax-table
                   character)))
             (list
              character
              (char-to-string character)
              entry
              (syntax-class entry)
              (with-syntax-table
                  as-mode-syntax-table
                (string
                 (char-syntax character))))))
         '(?\" ?- ?* ?\( ?\) ?\n ?\f ?: ?\\ ?a ?_))"##;
    let expect = expect![[
        r#"OK ((34 "\"" (7) 7 "\"") (45 "-" (196609) 1 ".") (42 "*" (2490369) 1 ".") (40 "(" (65540 . 41) 4 "(") (41 ")" (524293 . 40) 5 ")") (10 "\n" #1=(12) 12 ">") (12 "\f" #1# 12 ">") (58 ":" #2=(1) 1 ".") (92 "\\" #2# 1 ".") (97 "a" (2) 2 "w") (95 "_" (3) 3 "_"))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}
