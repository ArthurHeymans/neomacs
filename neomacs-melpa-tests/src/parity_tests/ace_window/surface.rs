use super::{assert_ace_window_parity, assert_ace_window_posframe_parity};
use expect_test::expect;

#[test]
fn ace_window_core_callable_surface_arglists_interactivity_and_source_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (interactive-form symbol)
            (and (documentation symbol) t)
            (file-name-nondirectory
             (symbol-file symbol 'defun))))
         '(aw-set-make-frame-char
           aw-ignored-p
           aw-window-list
           aw--done
           aw--restore-windows-hscroll
           aw--overlay-str
           aw--point-visible-p
           aw--lead-overlay
           aw--remove-leading-chars
           aw--make-backgrounds
           aw-set-mode-line
           aw--dispatch-action
           aw-make-frame
           aw-use-frame
           aw-clean-up-avy-current-path
           aw-dispatch-default
           aw-select
           aw-window<
           aw--push-window
           aw--pop-window
           aw-switch-to-window
           aw--face-rel-height
           aw-offset
           aw--after-make-frame
           aw-update))"##;
    let expect = expect![[
        r#"OK ((aw-set-make-frame-char (option value) nil nil nil "ace-window.el") (aw-ignored-p (window) nil nil t "ace-window.el") (aw-window-list nil nil nil t "ace-window.el") (aw--done nil nil nil t "ace-window.el") (aw--restore-windows-hscroll nil nil nil t "ace-window.el") (aw--overlay-str (wnd pos path) nil nil t "ace-window.el") (aw--point-visible-p nil nil nil t "ace-window.el") (aw--lead-overlay (path leaf) nil nil t "ace-window.el") (aw--remove-leading-chars nil nil nil nil "ace-window.el") (aw--make-backgrounds (wnd-list) nil nil t "ace-window.el") (aw-set-mode-line (str) nil nil t "ace-window.el") (aw--dispatch-action (char) nil nil t "ace-window.el") (aw-make-frame nil nil nil t "ace-window.el") (aw-use-frame (window) nil nil t "ace-window.el") (aw-clean-up-avy-current-path nil nil nil t "ace-window.el") (aw-dispatch-default (char) nil nil t "ace-window.el") (aw-select (mode-line &optional action) nil nil t "ace-window.el") (aw-window< (wnd1 wnd2) nil nil t "ace-window.el") (aw--push-window (window) nil nil t "ace-window.el") (aw--pop-window nil nil nil t "ace-window.el") (aw-switch-to-window (window) nil nil t "ace-window.el") (aw--face-rel-height nil nil nil nil "ace-window.el") (aw-offset (window) nil nil t "ace-window.el") (aw--after-make-frame (f) nil nil nil "ace-window.el") (aw-update nil nil nil t "ace-window.el"))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_public_and_window_operation_surface_arglists_interactivity_and_source_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (interactive-form symbol)
            (and (documentation symbol) t)
            (file-name-nondirectory
             (symbol-file symbol 'defun))))
         '(ace-select-window
           ace-delete-window
           ace-swap-window
           ace-delete-other-windows
           ace-display-buffer
           aw-transpose-frame
           ace-window
           aw-flip-window
           aw-show-dispatch-help
           aw-delete-window
           aw-switch-buffer-in-window
           aw--switch-buffer
           aw-swap-window
           aw-move-window
           aw-copy-window
           aw-split-window-vert
           aw-split-window-horz
           aw-split-window-fair
           aw-switch-buffer-other-window
           aw-execute-command-other-window
           ace-window-display-mode))"##;
    let expect = expect![[
        r#"OK ((ace-select-window nil t (interactive nil) t "ace-window.el") (ace-delete-window nil t (interactive nil) t "ace-window.el") (ace-swap-window nil t (interactive nil) t "ace-window.el") (ace-delete-other-windows nil t (interactive nil) t "ace-window.el") (ace-display-buffer (buffer alist) nil nil t "ace-window.el") (aw-transpose-frame (w) nil nil t "ace-window.el") (ace-window (arg) t (interactive "p") t "ace-window.el") (aw-flip-window nil t (interactive nil) t "ace-window.el") (aw-show-dispatch-help nil t (interactive nil) t "ace-window.el") (aw-delete-window (window &optional kill-buffer) nil nil t "ace-window.el") (aw-switch-buffer-in-window (window) nil nil t "ace-window.el") (aw--switch-buffer nil nil nil nil "ace-window.el") (aw-swap-window (window) nil nil t "ace-window.el") (aw-move-window (window) nil nil t "ace-window.el") (aw-copy-window (window) nil nil t "ace-window.el") (aw-split-window-vert (window) nil nil t "ace-window.el") (aw-split-window-horz (window) nil nil t "ace-window.el") (aw-split-window-fair (window) nil nil t "ace-window.el") (aw-switch-buffer-other-window (window) nil nil t "ace-window.el") (aw-execute-command-other-window (window) nil nil t "ace-window.el") (ace-window-display-mode (&optional arg) t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) t "ace-window.el"))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_custom_defaults_types_groups_and_source_ownership_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((standard
                  (get symbol
                       'standard-value)))
             (list
              symbol
              (symbol-value symbol)
              (default-value symbol)
              (special-variable-p symbol)
              (list
               (and standard t)
               (and standard
                    (eval
                     (car standard)
                     t)))
              (get symbol 'custom-type)
              (get symbol 'custom-group)
              (and
               (documentation-property
                symbol
                'variable-documentation
                t)
               t)
              (let ((file
                     (symbol-file
                      symbol
                      'defvar)))
                (and file
                     (file-name-nondirectory
                      file))))))
         '(aw-keys
           aw-scope
           aw-translate-char-function
           aw-minibuffer-flag
           aw-ignored-buffers
           aw-ignore-on
           aw-ignore-current
           aw-background
           aw-leading-char-style
           aw-dispatch-always
           aw-dispatch-when-more-than
           aw-reverse-frame-list
           aw-frame-offset
           aw-frame-size
           aw-char-position
           aw-make-frame-char
           aw-display-mode-overlay
           aw-swap-invert
           aw-fair-aspect-ratio
           ace-window-display-mode))"##;
    let expect = expect![[
        r#"OK ((aw-keys #1=(49 50 51 52 53 54 55 56 57) #1# t (t #1#) (repeat character) nil t "ace-window.el") (aw-scope global global t (t global) (choice (const :tag "visible frames" visible) (const :tag "global" global) (const :tag "frame" frame)) nil t "ace-window.el") (aw-translate-char-function identity identity t (t identity) (choice (const :tag "Off" #'identity) (const :tag "Ignore Case" #'downcase) (function :tag "Custom")) nil t "ace-window.el") (aw-minibuffer-flag nil nil t (t nil) boolean nil t "ace-window.el") (aw-ignored-buffers #2=("*Calc Trail*" " *LV*") #2# t (t #2#) (repeat string) nil t "ace-window.el") (aw-ignore-on t t t (t t) boolean nil t "ace-window.el") (aw-ignore-current nil nil t (t nil) boolean nil t "ace-window.el") (aw-background t t t (t t) boolean nil t "ace-window.el") (aw-leading-char-style char char t (t char) (choice (const :tag "single char" 'char) (const :tag "full path" 'path)) nil t "ace-window.el") (aw-dispatch-always nil nil t (t nil) boolean nil t "ace-window.el") (aw-dispatch-when-more-than 2 2 t (t 2) integer nil t "ace-window.el") (aw-reverse-frame-list nil nil t (t nil) boolean nil t "ace-window.el") (aw-frame-offset #3=(13 . 23) #3# t (t #3#) (cons integer integer) nil t "ace-window.el") (aw-frame-size nil nil t (t nil) (cons integer integer) nil t "ace-window.el") (aw-char-position top-left top-left t (t top-left) (choice (const :tag "top left corner only" 'top-left) (const :tag "both left corners" 'left)) nil t "ace-window.el") (aw-make-frame-char 122 122 t (t 122) character nil t "ace-window.el") (aw-display-mode-overlay t t t (t t) boolean nil t "ace-window.el") (aw-swap-invert nil nil t (t nil) boolean nil t "ace-window.el") (aw-fair-aspect-ratio 2 2 t (t 2) number nil t "ace-window.el") (ace-window-display-mode nil nil t (t nil) boolean nil t "ace-window.el"))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_internal_defaults_and_dispatch_table_match() {
    let elisp_form = r##"(list
         aw-dispatch-alist
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (cond
              ((eq symbol
                   'aw--window-ring)
               (list
                'ring
                (ring-size
                 aw--window-ring)
                (ring-length
                 aw--window-ring)))
              ((memq
                symbol
                '(aw--lead-overlay-fn
                  aw--remove-leading-chars-fn))
               (symbol-value symbol))
              (t
               (symbol-value symbol)))
             (special-variable-p
              symbol)))
          '(aw-overlays-back
            ace-window-mode
            aw-empty-buffers-list
            aw--windows-hscroll
            aw--windows-points
            aw--lead-overlay-fn
            aw--remove-leading-chars-fn
            aw-dispatch-function
            aw-action
            aw--window-ring)))"##;
    let expect = expect![[
        r#"OK (((120 aw-delete-window "Delete Window") (109 aw-swap-window "Swap Windows") (77 aw-move-window "Move Window") (99 aw-copy-window "Copy Window") (106 aw-switch-buffer-in-window "Select Buffer") (110 aw-flip-window) (117 aw-switch-buffer-other-window "Switch Buffer Other Window") (101 aw-execute-command-other-window "Execute Command Other Window") (70 aw-split-window-fair "Split Fair Window") (118 aw-split-window-vert "Split Vert Window") (98 aw-split-window-horz "Split Horz Window") (111 delete-other-windows "Delete Other Windows") (84 aw-transpose-frame "Transpose Frame") (63 aw-show-dispatch-help)) ((aw-overlays-back nil t) (ace-window-mode nil t) (aw-empty-buffers-list nil t) (aw--windows-hscroll nil t) (aw--windows-points nil t) (aw--lead-overlay-fn aw--lead-overlay t) (aw--remove-leading-chars-fn aw--remove-leading-chars t) (aw-dispatch-function aw-dispatch-default t) (aw-action nil t) (aw--window-ring (ring 10 0) t)))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_faces_and_customize_group_metadata_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (face)
            (list
             face
             (get face 'face-defface-spec)
             (get face 'face-documentation)
             (get face 'custom-group)
             (file-name-nondirectory
              (symbol-file face
                           'defface))))
          '(aw-leading-char-face
            aw-minibuffer-leading-char-face
            aw-background-face
            aw-mode-line-face
            aw-key-face))
         (get 'ace-window
              'group-documentation)
         (get 'ace-window
              'custom-prefix)
         (and
          (member
           '(ace-window custom-group)
           (get 'convenience
                'custom-group))
          t))"##;
    let expect = expect![[
        r#"OK (((aw-leading-char-face ((((class color)) (:foreground "red")) (((background dark)) (:foreground "gray100")) (((background light)) (:foreground "gray0")) (t (:foreground "gray100" :underline nil))) "Face for each window's leading char." nil "ace-window.el") (aw-minibuffer-leading-char-face ((t :inherit aw-leading-char-face)) "Face for minibuffer leading char." nil "ace-window.el") (aw-background-face ((t (:foreground "gray40"))) "Face for whole window background during selection." nil "ace-window.el") (aw-mode-line-face ((t (:inherit mode-line-buffer-id))) "Face used for displaying the ace window key in the mode-line." nil "ace-window.el") (aw-key-face ((t :inherit font-lock-builtin-face)) "Face used by `aw-show-dispatch-help'." nil "ace-window.el")) "Quickly switch current window." "aw-" t)"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_mode_hooks_and_complete_customize_group_membership_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (boundp symbol)
             (symbol-value symbol)
             (default-value symbol)
             (special-variable-p symbol)
             (get symbol
                  'custom-type)
             (get symbol
                  'custom-group)
             (documentation-property
              symbol
              'variable-documentation
              t)
             (file-name-nondirectory
              (symbol-file
               symbol
               'defvar))))
          '(ace-window-display-mode-hook
            ace-window-posframe-mode-hook))
         (copy-tree
          (get 'ace-window
               'custom-group)))"##;
    let expect = expect![[
        r#"OK (((ace-window-display-mode-hook t nil nil t hook nil "Hook run after entering or leaving `ace-window-display-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "ace-window.el") (ace-window-posframe-mode-hook t nil nil t hook nil "Hook run after entering or leaving `ace-window-posframe-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "ace-window-posframe.el")) ((aw-keys custom-variable) (aw-scope custom-variable) (aw-translate-char-function custom-variable) (aw-minibuffer-flag custom-variable) (aw-ignored-buffers custom-variable) (aw-ignore-on custom-variable) (aw-ignore-current custom-variable) (aw-background custom-variable) (aw-leading-char-style custom-variable) (aw-dispatch-always custom-variable) (aw-dispatch-when-more-than custom-variable) (aw-reverse-frame-list custom-variable) (aw-frame-offset custom-variable) (aw-frame-size custom-variable) (aw-char-position custom-variable) (aw-make-frame-char custom-variable) (aw-leading-char-face custom-face) (aw-minibuffer-leading-char-face custom-face) (aw-background-face custom-face) (aw-mode-line-face custom-face) (aw-key-face custom-face) (aw-display-mode-overlay custom-variable) (aw-swap-invert custom-variable) (aw-fair-aspect-ratio custom-variable) (ace-window-display-mode custom-variable) (ace-window-posframe-mode custom-variable)))"#
    ]];
    assert_ace_window_posframe_parity(elisp_form, expect);
}

#[test]
fn ace_window_source_reload_preserves_prebound_configuration_and_dispatch() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ace-window
               'defun)))
         (setq aw-keys '(?a ?b)
               aw-scope 'frame
               aw-background nil
               aw-make-frame-char nil
               aw-fair-aspect-ratio 9
               aw-dispatch-alist
               '((?q fixture-command
                      "Fixture")))
         (load path nil t)
         (list
          aw-keys
          aw-scope
          aw-background
          aw-make-frame-char
          aw-fair-aspect-ratio
          aw-dispatch-alist
          (featurep 'ace-window)))"##;
    let expect = expect![[r#"OK ((97 98) frame nil nil 9 ((113 fixture-command "Fixture")) t)"#]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_packaged_sources_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-window
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
                '("ace-window.el"
                  "ace-window-posframe.el"
                  "ace-window-pkg.el"
                  "ace-window-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-window.el" 35488 "da78bb76ae630b632118618e3612f1373b1d1036beef73197551aa3c6fa1d714") ("ace-window-posframe.el" 2534 "d162b47d37108ab91fc28d351acac1410bbe6984496ce285cc80001f3eedf97d") ("ace-window-pkg.el" 417 "5295991b6ed8525b73776dd3d4a45e264854225f5dbb8559f1d114040d2c573e") ("ace-window-autoloads.el" 4116 "bc47e9a89f7fab7de63ccb5b208e7b5eff8f175352d81224c1a5b544e70eb70d") ("README-elpa" 1278 "d5b73d81a1c4c78e3abd885b69747606bd5f8bf0b34425002346a16fb4181e15"))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_installation_byte_compiles_both_runtime_sources() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-window
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
                    (list
                     name
                     (file-exists-p path)
                     (file-regular-p path)
                     (> (file-attribute-size
                         (file-attributes
                          path))
                        0))))
                '("ace-window.elc"
                  "ace-window-posframe.elc")))"##;
    let expect = expect![[r#"OK (("ace-window.elc" t t t) ("ace-window-posframe.elc" t t t))"#]];
    assert_ace_window_parity(elisp_form, expect);
}
