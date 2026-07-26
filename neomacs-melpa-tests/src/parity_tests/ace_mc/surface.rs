use super::assert_ace_mc_parity;
use expect_test::expect;

#[test]
fn ace_mc_callable_surface_metadata_matches() {
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
         '(ace-mc-maybe-jump-start
           ace-mc-maybe-jump-end
           ace-mc-reset
           ace-mc-do-keyboard-reset
           ace-mc-quick-exchange
           ace-mc-add-multiple-cursors
           ace-mc-add-single-cursor
           ace-mc-regexp-mode
           ace-mc-add-char))"##;
    let expect = expect![[
        r#"OK ((ace-mc-maybe-jump-start nil nil nil "Push the mark when marking with ‘ace-jump-char-mode’." "ace-mc.el") (ace-mc-maybe-jump-end nil nil nil "Add/remove cursor jumping with ‘ace-jump-char-mode.’." "ace-mc.el") (ace-mc-reset nil nil nil "Reset the internal jumping flags." "ace-mc.el") (ace-mc-do-keyboard-reset nil t (interactive nil) "Reset when the function ‘ace-jump-mode’ is cancelled.\nAlso called when chosen character isn’t found while zapping." "ace-mc.el") (ace-mc-quick-exchange nil t (interactive nil) "Act like ‘ace-jump-quick-exchange’, switching between ‘ace-jump-char-mode’ and ‘ace-jump-word-mode’." "ace-mc.el") (ace-mc-add-multiple-cursors (&optional prefix single-mode) t (interactive "pi") #("Use AceJump to add or remove multiple cursors.\n\n‘ace-mc-add-multiple-cursors’ will prompt your for locations to add\nmultiple cursors.  If a cursor already exists at that location,\nit will be removed.  This process continues looping until you\nexit, for example by pressing return or escape.\n\nWithout a C-u prefix argument, use the default\nAceJump jumping mode as described in\n‘ace-jump-mode-submode-list’.  When called interactively with one\nor more C-u prefix arguments PREFIX, use the\ncorresponding mode from ‘ace-jump-mode-submode-list’.  For\nexample, by default\n   M-x ace-mc-add-multiple-cursors ==> ‘ace-jump-word-mode’\n   C-u M-x ace-mc-add-multiple-cursors ==> ‘ace-jump-char-mode’\n   C-u C-u M-x ace-mc-add-multiple-cursors ==> ‘ace-jump-line-mode’\n\nIf SINGLE-MODE is set to ’t’, don’t loop.\n\nWhen the region is active, prompt for AceJump matches based on matching strings." 301 304 (font-lock-face help-key-binding face help-key-binding) 449 452 (font-lock-face help-key-binding face help-key-binding) 568 572 (font-lock-face help-key-binding face help-key-binding) 572 599 (font-lock-face help-key-binding face help-key-binding) 628 631 (font-lock-face help-key-binding face help-key-binding) 632 636 (font-lock-face help-key-binding face help-key-binding) 636 663 (font-lock-face help-key-binding face help-key-binding) 692 695 (font-lock-face help-key-binding face help-key-binding) 696 699 (font-lock-face help-key-binding face help-key-binding) 700 704 (font-lock-face help-key-binding face help-key-binding) 704 731 (font-lock-face help-key-binding face help-key-binding)) "ace-mc.el") (ace-mc-add-single-cursor (&optional prefix) t (interactive "p") "Add a single multiple cursor.\n\nThis is a wrapper for ‘ace-mc-add-multiple-cursors’, only adding\na single cursor.\n\nPREFIX is passed to ‘ace-mc-add-multiple-cursors’, see the\ndocumentation there." "ace-mc.el") (ace-mc-regexp-mode (regex) nil nil "Ace Jump Multiple Cursor with a REGEX." "ace-mc.el") (ace-mc-add-char (query-char) nil nil "Call ‘ace-jump-char-mode’ with a character QUERY-CHAR and add a cursor at the point." "ace-mc.el"))"#
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_variable_defaults_metadata_and_source_ownership_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (symbol-value symbol)
            (special-variable-p symbol)
            (get symbol 'standard-value)
            (documentation-property
             symbol
             'variable-documentation
             t)
            (file-name-nondirectory
             (symbol-file symbol 'defvar))))
         '(ace-mc-marking
           ace-mc-keyboard-reset
           ace-mc-query-char
           ace-mc-loop-marking
           ace-mc-saved-point
           ace-mc-ace-mode-function))"##;
    let expect = expect![[
        r#"OK ((ace-mc-marking nil t nil "Internal flag for detecting if currently marking." "ace-mc.el") (ace-mc-keyboard-reset nil t nil "See if we've quit out yet." "ace-mc.el") (ace-mc-query-char nil t nil "Char." "ace-mc.el") (ace-mc-loop-marking nil t nil "Keep adding until we quit." "ace-mc.el") (ace-mc-saved-point nil t nil "The position of our cursor before jumping around with ace-jump." "ace-mc.el") (ace-mc-ace-mode-function nil t nil "The function from `ace-jump-mode-submode-list` to use." "ace-mc.el"))"#
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_source_reload_preserves_all_prebound_package_variables() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ace-mc-add-char
               'defun)))
         (setq ace-mc-marking 'prebound-marking
               ace-mc-keyboard-reset 'prebound-reset
               ace-mc-query-char 'prebound-query
               ace-mc-loop-marking 'prebound-loop
               ace-mc-saved-point 'prebound-point
               ace-mc-ace-mode-function 'prebound-function)
         (load path nil t)
         (list
          ace-mc-marking
          ace-mc-keyboard-reset
          ace-mc-query-char
          ace-mc-loop-marking
          ace-mc-saved-point
          ace-mc-ace-mode-function
          (featurep 'ace-mc)))"##;
    let expect = expect![
        "OK (prebound-marking prebound-reset prebound-query prebound-loop prebound-point prebound-function t)"
    ];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-mc
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor)))
               (mapcar
                (lambda (name)
                  (let ((path
                         (expand-file-name
                          name
                          directory)))
                    (with-temp-buffer
                      (set-buffer-multibyte nil)
                      (insert-file-contents-literally path)
                      (list
                       name
                       (buffer-size)
                       (secure-hash
                        'sha256
                        (current-buffer))))))
                '("ace-mc.el"
                  "ace-mc-pkg.el"
                  "ace-mc-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-mc.el" 7883 "af63058842c14c01d5af6bc586b2b3fc4b89e8a7d9ab9d058bfa539ce04a3147") ("ace-mc-pkg.el" 510 "96cd3ce16cdd0b05c170955dde42e3961ebd8943aee6ae5f3d487f7610807554") ("ace-mc-autoloads.el" 2003 "e8e9e1e29648cc32071c980666803c35b19d23c9692f5976e9655cde77f3405b") ("README-elpa" 898 "0c9541e6254f5bcf2f84a8cbc6d2cefe55ab14efc23a4e10a8d0012da4bb62a3"))"#
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_installation_produces_a_local_byte_compilation_artifact() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-mc
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor))
                     (path
                      (expand-file-name
                       "ace-mc.elc"
                       directory)))
               (list
                (file-exists-p path)
                (file-regular-p path)
                (> (file-attribute-size
                    (file-attributes path))
                   0)))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_mc_parity(elisp_form, expect);
}
