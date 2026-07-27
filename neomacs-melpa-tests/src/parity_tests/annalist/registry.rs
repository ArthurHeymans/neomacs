use expect_test::expect;

use super::{assert_annalist_autoload_parity, assert_annalist_parity};

#[test]
fn annalist_exact_pin_metadata_and_builtin_registration_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr (assq 'annalist package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join (package-desc-version descriptor))
          (package-desc-reqs descriptor)
          (package-desc-summary descriptor)
          (copy-tree (package-desc-extras descriptor))
          (featurep 'annalist)
          (let ((settings
                 (plist-get
                  annalist--tomes-settings
                  'keybindings)))
            (list
             (plist-get settings :type)
             (plist-get settings :key-indices)
             (plist-get settings :final-index)
             (plist-get settings :metadata-index)
             (plist-get settings :primary-key)
             (plist-get settings :table-start-index)
             (plist-get settings :preprocess)
             (plist-get settings :record-update)
             (mapcar
              (lambda (item)
                (list
                 item
                 (plist-get
                  (plist-get settings item)
                  :index)))
              '(keymap
                state
                key
                definition
                previous-definition))))
          (mapcar
           (lambda (view)
             (let ((settings
                    (annalist--get-view-settings
                     'keybindings
                     view)))
               (list
                view
                (plist-get
                 (plist-get settings 'keymap)
                 :predicate)
                (plist-get
                 (plist-get settings 'state)
                 :predicate)
                (funcall
                 (plist-get
                  (plist-get settings 'key)
                  :format)
                 (kbd "C-c a"))
                (plist-get
                 (plist-get
                  settings
                  'previous-definition)
                 :title)
                (plist-get settings :hooks))))
           '(default valid active))))"##;
    let expect = expect![[
        r#"OK (annalist "20260531.1558" ((emacs (24 4)) (cl-lib (0 5))) "Record and display information such as keybindings." ((:maintainers ("Fox Kiester" . "noct@posteo.net")) (:authors ("Fox Kiester" . "noct@posteo.net")) (:keywords "convenience" "tools" "keybindings" "org") (:revdesc . "0d958732b710") (:commit . "0d958732b710a8e9edc4c70b2318570e1c7d4923") (:url . "https://github.com/noctuid/annalist.el")) t (keybindings (0 1 2) 4 5 (keymap state key) 2 annalist--preprocess-keybinding annalist--update-keybinding ((keymap 0) (state 1) (key 2) (definition 3) (previous-definition 4))) ((default nil nil "=C-c a=" "Previous" annalist-multiline-source-blocks) (valid annalist--valid-keymap-p annalist--valid-state-p "=C-c a=" "Previous" annalist-multiline-source-blocks) (active annalist--active-keymap-p annalist--valid-state-and-evil-on-p "=C-c a=" "Previous" annalist-multiline-source-blocks)))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_installed_payload_is_exact_and_contains_no_upstream_test_sources() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr (assq 'annalist package-alist)))
                 (directory (package-desc-dir descriptor)))
         (mapcar
          (lambda (name)
            (let ((path (expand-file-name name directory)))
              (if (string-suffix-p ".elc" name)
                  (list
                   name
                   :compiled
                   (file-regular-p path)
                   (> (nth 7 (file-attributes path)) 0))
                (with-temp-buffer
                  (set-buffer-multibyte nil)
                  (insert-file-contents-literally path)
                  (list
                   name
                   (buffer-size)
                   (secure-hash 'sha256 (current-buffer)))))))
          (sort
           (directory-files directory nil "\\`[^.]")
           #'string<)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 168 "65a5081f43cbdfa5c51e05e93b4d6517e8a9d3a5d836d29d69c52e6e35583f26") ("annalist-autoloads.el" 1805 "809a76ef763739972e14afd9177a629557b09cff4e3ff674367cfa48e4519b05") ("annalist-pkg.el" 480 "d73ed4ba393a3c9e6bbfa7c97d33d272b6bee549cd09b3454fbcbe8a8d9f091b") ("annalist.el" 36518 "1ca468e25a6118819aecc81cb26c16534aeaf4d5c180636697c6cde66b705cee") ("annalist.elc" :compiled t t) ("annalist.info" 20368 "b2f27591138b52083d7d0151afe5665c6d9b1bed5efa7f0f647ff1f10a6fa5ab") ("dir" 667 "19ecf1b26957297dd2be95864b639dce5096771502faa0fd2b0e4c04c873e5d8"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_customization_defaults_types_and_widget_contract_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (let ((standard
                   (get symbol 'standard-value)))
              (list
               symbol
               (default-value symbol)
               (and standard (eval (car standard)))
               (get symbol 'custom-type)
               (get symbol 'custom-group))))
          '(annalist-record
            annalist-record-whitelist
            annalist-record-blacklist
            annalist-describe-hook
            annalist-org-startup-folded
            annalist-multiline-function
            annalist-update-previous-key-definition))
         (get 'annalist-list 'widget-type)
         (get 'annalist-list 'widget-documentation)
         (get 'annalist-list 'widget-type))"##;
    let expect = expect![[
        r#"OK (((annalist-record t t boolean nil) (annalist-record-whitelist nil nil annalist-list nil) (annalist-record-blacklist nil nil annalist-list nil) (annalist-describe-hook nil nil hook nil) (annalist-org-startup-folded nil nil (choice (const :tag "nofold: show all" nil) (const :tag "fold: overview" t) (const :tag "content: all headlines" content) (const :tag "show everything, even drawers" showeverything)) nil) (annalist-multiline-function lispy-alt-multiline lispy-alt-multiline function nil) (annalist-update-previous-key-definition on-change on-change (choice (const :tag "When definition has changed" on-change) (const :tag "When the key was previously unbound" nil)) nil)) #1=(lazy :type (choice (list (list symbol symbol)) (const nil))) "List with elements in the form (<annalist-name> <tome-type>).\nType for `annalist-record-blacklist' and `annalist-record-whitelist'." #1#)"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_command_and_function_surface_has_exact_interactive_and_documentation_shape() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp symbol)
            (commandp symbol)
            (documentation symbol)))
         '(annalist-define-tome
           annalist-define-view
           annalist-record
           annalist-describe
           annalist-plistify-record
           annalist-listify-record
           annalist-verbatim
           annalist-code
           annalist-capitalize
           annalist-compose
           annalist-string-<
           annalist-key-<
           annalist-multiline-source-blocks))"##;
    let expect = expect![[
        r#"OK ((annalist-define-tome t nil "Create a new type of thing that can be recorded called TYPE.\nSETTINGS be a list of items and any settings necessary for recording them.") (annalist-define-view t nil "Define a display method for TYPE called NAME.\nTo define the default view SETTINGS, NAME should be 'default. If INHERIT is\nnon-nil, inherit SETTINGS from that view.\n\n(fn TYPE NAME SETTINGS &key INHERIT)") (annalist-record t nil "In the store for ANNALIST, TYPE, and LOCAL, record RECORD.\nANNALIST should correspond to the package/user recording this information (e.g.\n'general, 'me, etc.). TYPE is the type of information being recorded (e.g.\n'keybindings). LOCAL corresponds to whether to store RECORD only for the current\nbuffer. This information together is used to select where RECORD should be\nstored in and later retrieved from with ‘annalist-describe’. RECORD should be a\nlist of items to record and later print as org headings and column entries in a\nsingle row. If PLIST is non-nil, RECORD should be a plist instead of an ordered\nlist (e.g. '(keymap org-mode-map key \"C-c a\" ...)). The plist keys should be\nthe symbols used for the definition of TYPE.\n\n(fn ANNALIST TYPE RECORD &key LOCAL PLIST)") (annalist-describe t nil "Describe information recorded by ANNALIST for TYPE.\nFor example: (annalist-describe 'general 'keybindings) If VIEW is non-nil, use\nthose settings for displaying recorded information instead of the defaults.") (annalist-plistify-record t nil "Convert the ordered RECORD list of TYPE to a plist.") (annalist-listify-record t nil "Convert the RECORD plist of TYPE to an ordered list.") (annalist-verbatim t nil "Format ITEM to be surrounded by equal signs.") (annalist-code t nil "Format ITEM to be surrounded by tildes.") (annalist-capitalize t nil "Convert ITEM to a string and capitalize it.") (annalist-compose t nil "Return a function composed of FNS.\nFNS will be called right to left.") (annalist-string-< t nil "Return whether X is lexicographiclly less than Y.\nThe string forms of X and Y as obtained with ‘format’ are compared.") (annalist-key-< t nil "Return whether X is lexicographically less than Y.\nBoth are considered to be keys in their bindable forms. Compare their\ndescriptive forms as obtained with ‘key-description’") (annalist-multiline-source-blocks t nil "Format Emacs Lisp source blocks in current buffer using lispy.\nWhen lispy is installed, use ‘lispy-multiline’ to format the elisp source blocks\nin the current buffer. This is useful since annalist will extract items to\nsource blocks as a single line."))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_autoloads_register_only_record_and_describe_entry_points() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (and (fboundp symbol) t)
            (and
             (fboundp symbol)
             (autoloadp (symbol-function symbol)))
            (commandp symbol)))
         '(annalist-record
           annalist-describe
           annalist-define-tome
           annalist-define-view
           annalist-code))"##;
    let expect = expect![
        "OK ((annalist-record t t nil) (annalist-describe t t nil) (annalist-define-tome nil nil nil) (annalist-define-view nil nil nil) (annalist-code nil nil nil))"
    ];

    assert_annalist_autoload_parity(elisp_form, expect);
}
