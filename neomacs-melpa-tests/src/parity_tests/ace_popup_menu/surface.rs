use super::assert_ace_popup_menu_parity;
use expect_test::expect;

#[test]
fn ace_popup_menu_callable_surface_metadata_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (interactive-form symbol)
            (documentation symbol)
            (file-name-nondirectory
             (symbol-file symbol 'defun))))
         '(ace-popup-menu-mode
           ace-popup-menu))"##;
    let expect = expect![[
        r#"OK ((ace-popup-menu-mode (&optional arg) t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Toggle the ‘ace-popup-menu-mode’ minor mode.\n\nWith a prefix argument ARG, enable ‘ace-popup-menu-mode’ if ARG\nis positive, and disable it otherwise.  If called from Lisp,\nenable the mode if ARG is omitted or NIL, and toggle it if ARG is\n‘toggle’.\n\nThis minor mode is global.  When it’s active any call to\n‘x-popup-menu’ will result in a call of ‘ace-popup-menu’\ninstead.  That function in turn implements a more efficient\ninterface to select an option from a list.  Emacs Lisp code can\nalso use ‘ace-popup-menu’ directly." "ace-popup-menu.el") (ace-popup-menu (orig-fun position menu) nil nil "Pop up a menu in a temporary window and return user’s selection.\n\nIf POSITION is nil or MENU is a keymap or list of keymaps, the\noriginal ‘x-popup-menu’ function is called via ORIG-FUN instead\nof ‘avy-menu’.  To understand the format of the MENU argument,\nsee documentation for ‘x-popup-menu’." "ace-popup-menu.el"))"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_variables_group_defaults_and_custom_metadata_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value symbol)
             (default-value symbol)
             (special-variable-p symbol)
             (let ((standard
                    (get symbol
                         'standard-value)))
               (list
                (and standard t)
                (and standard
                     (eval
                      (car standard)
                      t))))
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (documentation-property
              symbol
              'variable-documentation
              t)
             (let ((file
                    (symbol-file
                     symbol
                     'defvar)))
               (and file
                    (file-name-nondirectory
                     file)))))
          '(ace-popup-menu-show-pane-header
            ace-popup-menu-mode
            ace-popup-menu-mode-hook))
         (get 'ace-popup-menu
              'custom-group)
         (get 'ace-popup-menu
              'group-documentation)
         (get 'ace-popup-menu
              'custom-prefix)
         (copy-tree
          (get 'ace-popup-menu
               'custom-links))
         (get 'ace-popup-menu
              'custom-tag)
         (get
          'ace-popup-menu-show-pane-header
          'custom-tag)
         (and
          (member
           '(ace-popup-menu custom-group)
           (get 'convenience
                'custom-group))
          t))"##;
    let expect = expect![[
        r#"OK (((ace-popup-menu-show-pane-header nil nil t (t nil) boolean nil "Whether to print headers of individual panes in Ace Popup Menu." "ace-popup-menu.el") (ace-popup-menu-mode nil nil t (t nil) boolean nil "Non-nil if Ace-Popup-Menu mode is enabled.\nSee the `ace-popup-menu-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `ace-popup-menu-mode'." "ace-popup-menu.el") (ace-popup-menu-mode-hook nil nil t (t nil) hook nil "Hook run after entering or leaving `ace-popup-menu-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "ace-popup-menu.el")) ((ace-popup-menu-show-pane-header custom-variable) (ace-popup-menu-mode custom-variable)) "Replace GUI popup menu with something more efficient." "ace-popup-menu-" ((url-link :tag "GitHub" "https://github.com/mrkkrp/ace-popup-menu")) "Ace popup menu" "Show pane header" t)"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_source_reload_preserves_prebound_customization_and_hook() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ace-popup-menu
               'defun)))
         (setq ace-popup-menu-show-pane-header
               'prebound-header
               ace-popup-menu-mode-hook
               '(prebound-hook))
         (load path nil t)
         (list
          ace-popup-menu-show-pane-header
          ace-popup-menu-mode-hook
          (featurep 'ace-popup-menu)))"##;
    let expect = expect!["OK (prebound-header (prebound-hook) t)"];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-popup-menu
                        package-alist)))
                     (directory
                      (package-desc-dir
                       descriptor)))
               (mapcar
                (lambda (name)
                  (let ((path
                         (expand-file-name
                          name
                          directory)))
                    (with-temp-buffer
                      (set-buffer-multibyte nil)
                      (insert-file-contents-literally
                       path)
                      (list
                       name
                       (buffer-size)
                       (secure-hash
                        'sha256
                        (current-buffer))))))
                '("ace-popup-menu.el"
                  "ace-popup-menu-pkg.el"
                  "ace-popup-menu-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-popup-menu.el" 3200 "ee00f79935c28dda27de143f64b7260d7f38fdf43ef4e5e93d6bddd98a9634d5") ("ace-popup-menu-pkg.el" 495 "69acd428cd3009a4f3767f159d10874de39a2c3b75811b6ec9daaffd4c276d46") ("ace-popup-menu-autoloads.el" 2071 "c615ed5d48ba54817283f88f8b510f7da54f927537a979538c747e4045431672") ("README-elpa" 198 "300c74468c5d8c0c2be7ac69706100e0d054c2ffb941245b205d9e2aab94d93e"))"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_installation_produces_a_local_byte_compilation_artifact() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-popup-menu
                        package-alist)))
                     (directory
                      (package-desc-dir
                       descriptor))
                     (path
                      (expand-file-name
                       "ace-popup-menu.elc"
                       directory)))
               (list
                (file-exists-p path)
                (file-regular-p path)
                (> (file-attribute-size
                    (file-attributes path))
                   0)))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_popup_menu_parity(elisp_form, expect);
}
