use super::assert_ace_jump_mode_parity;
use expect_test::expect;

#[test]
fn ace_jump_mode_primary_configuration_defaults_match() {
    let elisp_form = r##"(list
         ace-jump-word-mode-use-query-char
         ace-jump-mode-case-fold
         ace-jump-mode-mark-ring
         ace-jump-mode-mark-ring-max
         ace-jump-mode-gray-background
         ace-jump-mode-scope
         ace-jump-mode-detect-punc
         ace-jump-mode-submode-list)"##;
    let expect = expect![
        "OK (t t nil 100 t global t (ace-jump-word-mode ace-jump-char-mode ace-jump-line-mode))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_keys_cover_lowercase_then_uppercase_ascii() {
    let elisp_form = r##"(list
         (length ace-jump-mode-move-keys)
         ace-jump-mode-move-keys
         (mapconcat
          (lambda (character)
            (char-to-string character))
          ace-jump-mode-move-keys
          "")
         (equal ace-jump-mode-move-keys
                (append
                 (number-sequence ?a ?z)
                 (number-sequence ?A ?Z))))"##;
    let expect = expect![[
        r#"OK (52 (97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 119 120 121 122 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90) "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ" t)"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_internal_state_defaults_match() {
    let elisp_form = r##"(list
         ace-jump-mode
         ace-jump-background-overlay-list
         ace-jump-search-tree
         ace-jump-query-char
         ace-jump-current-mode
         ace-jump-sync-emacs-mark-ring
         ace-jump-search-filter
         ace-jump-mode-before-jump-hook
         ace-jump-mode-end-hook
         ace-jump-allow-invisible)"##;
    let expect = expect!["OK (nil nil nil nil nil nil nil nil nil nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_configuration_variables_are_special_and_source_owned() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (special-variable-p symbol)
            (file-name-nondirectory
             (symbol-file symbol 'defvar))
            (documentation-property
             symbol
             'variable-documentation
             t)))
         '(ace-jump-word-mode-use-query-char
           ace-jump-mode-case-fold
           ace-jump-mode-mark-ring
           ace-jump-mode-mark-ring-max
           ace-jump-mode-gray-background
           ace-jump-mode-scope
           ace-jump-mode-detect-punc
           ace-jump-mode-submode-list
           ace-jump-mode-move-keys))"##;
    let expect = expect![[
        r#"OK ((ace-jump-word-mode-use-query-char t "ace-jump-mode.el" "If we need to ask for the query char before enter `ace-jump-word-mode'") (ace-jump-mode-case-fold t "ace-jump-mode.el" "If non-nil, the ace-jump mode will ignore case.\n\nThe default value is set to the same as `case-fold-search'.") (ace-jump-mode-mark-ring t "ace-jump-mode.el" "The list that is used to store the history for jump back.") (ace-jump-mode-mark-ring-max t "ace-jump-mode.el" "The max length of `ace-jump-mode-mark-ring'") (ace-jump-mode-gray-background t "ace-jump-mode.el" "By default, when there is more than one candidate, the ace\njump mode will gray the background and then mark the possible\ncandidate position. Set this to nil means do not gray\nbackground.") (ace-jump-mode-scope t "ace-jump-mode.el" "Define what is the scope that ace-jump-mode works.\n\nNow, there are four kinds of values for this:\n1. 'global  : ace jump can work across any window and frame, this is also the default.\n2. 'frame   : ace jump will work for the all windows in current frame.\n3. 'visible : ace jump will work for all windows in visible frames.\n3. 'window  : ace jump will only work on current window only.\n              This is the same behavior for 1.0 version.") (ace-jump-mode-detect-punc t "ace-jump-mode.el" "When this is non-nil, the ace jump word mode will detect the\nchar that is not alpha or number. Then, if the query char is a\nprintable punctuaction, we will use char mode to start the ace\njump mode. If it is nil, an error will come up when\nnon-alpha-number is given under word mode.") (ace-jump-mode-submode-list t "ace-jump-mode.el" "*The mode list when start ace jump mode.\nThe sequence is the calling sequence when give prefix argument.\n\nSuch as:\n  If you use the default sequence, which is\n      '(ace-jump-word-mode\n        ace-jump-char-mode\n        ace-jump-line-mode)\nand using key to start up ace jump mode, such as 'C-c SPC',\nthen the usage to start each mode is as below:\n\n   C-c SPC           ==> ace-jump-word-mode\n   C-u C-c SPC       ==> ace-jump-char-mode\n   C-u C-u C-c SPC   ==> ace-jump-line-mode\n\nCurrently, the valid submode is:\n   `ace-jump-word-mode'\n   `ace-jump-char-mode'\n   `ace-jump-line-mode'\n\n") (ace-jump-mode-move-keys t "ace-jump-mode.el" "*The keys that used to move when enter AceJump mode.\nEach key should only an printable character, whose name will\nfill each possible location.\n\nIf you want your own moving keys, you can custom that as follow,\nfor example, you only want to use lower case character:\n(setq ace-jump-mode-move-keys (loop for i from ?a to ?z collect i)) "))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_internal_variables_are_special_and_source_owned() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (special-variable-p symbol)
            (file-name-nondirectory
             (symbol-file symbol 'defvar))
            (documentation-property
             symbol
             'variable-documentation
             t)))
         '(ace-jump-mode
           ace-jump-background-overlay-list
           ace-jump-search-tree
           ace-jump-query-char
           ace-jump-current-mode
           ace-jump-sync-emacs-mark-ring
           ace-jump-search-filter
           ace-jump-mode-before-jump-hook
           ace-jump-mode-end-hook
           ace-jump-allow-invisible))"##;
    let expect = expect![[
        r#"OK ((ace-jump-mode t "ace-jump-mode.el" "AceJump minor mode.") (ace-jump-background-overlay-list t "ace-jump-mode.el" "Background overlay which will grey all the display.") (ace-jump-search-tree t "ace-jump-mode.el" "N-branch Search tree. Every leaf node holds the overlay that\nis used to highlight the target positions.") (ace-jump-query-char t "ace-jump-mode.el" "Save the query char used between internal mode.") (ace-jump-current-mode t "ace-jump-mode.el" "Save the current mode.\nSee `ace-jump-mode-submode-list' for possible value.") (ace-jump-sync-emacs-mark-ring t "ace-jump-mode.el" "When this variable is not-nil, everytime `ace-jump-mode-pop-mark' is called,\nace jump will try to remove the same mark from buffer local mark\nring and global-mark-ring, which help you to sync the mark\ninformation between emacs and ace jump.\n\nNote, never try to set this variable manually, this is for ace\njump internal use.  If you want to change it, use\n`ace-jump-mode-enable-mark-sync' or\n`ace-jump-mode-disable-mark-sync'.") (ace-jump-search-filter t "ace-jump-mode.el" "This should be nil or a point-dependant predicate\nthat `ace-jump-search-candidate' will use as an additional filter.") (ace-jump-mode-before-jump-hook t "ace-jump-mode.el" "Function(s) to call just before moving the cursor to a selected match") (ace-jump-mode-end-hook t "ace-jump-mode.el" "Function(s) to call when ace-jump-mode is going to end up") (ace-jump-allow-invisible t "ace-jump-mode.el" "Control if ace-jump should select the invisible char as candidate.\nNormally, the ace jump mark cannot be seen if the target character is invisible.\nSo default to be nil, which will not include those invisible character as candidate."))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_defvar_load_preserves_prebound_values() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ace-jump-char-category
               'defun)))
         (unload-feature 'ace-jump-mode t)
         (setq ace-jump-mode-scope 'prebound-scope)
         (setq ace-jump-mode-mark-ring-max 7)
         (setq ace-jump-mode-move-keys '(?x ?y))
         (load path nil t)
         (list
          ace-jump-mode-scope
          ace-jump-mode-mark-ring-max
          ace-jump-mode-move-keys
          (featurep 'ace-jump-mode)))"##;
    let expect = expect!["OK (prebound-scope 7 (120 121) t)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_case_fold_default_was_captured_at_initial_load() {
    let elisp_form = r##"(list
         case-fold-search
         ace-jump-mode-case-fold
         (eq case-fold-search
             ace-jump-mode-case-fold))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_group_and_face_metadata_match() {
    let elisp_form = r##"(list
         (get 'ace-jump 'group-documentation)
         (assq 'ace-jump
               (get 'convenience 'custom-group))
         (mapcar
          (lambda (face)
            (list
             face
             (not
              (null
               (facep face)))
             (get face 'face-defface-spec)
             (get face 'face-documentation)
             (get face 'face-modified)))
          '(ace-jump-face-background
            ace-jump-face-foreground)))"##;
    let expect = expect![[
        r#"OK ("ace jump group" (ace-jump custom-group) ((ace-jump-face-background t ((t (:foreground "gray40"))) "Face for background of AceJump motion" nil) (ace-jump-face-foreground t ((((class color)) (:foreground "red" :underline nil)) (((background dark)) (:foreground "gray100" :underline nil)) (((background light)) (:foreground "gray0" :underline nil)) (t (:foreground "gray100" :underline nil))) "Face for foreground of AceJump motion" nil)))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_minor_mode_alist_registration_is_idempotent() {
    let elisp_form = r##"(let* ((path
               (symbol-file
                'ace-jump-char-category
                'defun))
              (before
               (length
                (delq nil
                      (mapcar
                       (lambda (entry)
                         (and
                          (eq
                           (car-safe entry)
                           'ace-jump-mode)
                          entry))
                       minor-mode-alist)))))
         (load path nil t)
         (load path nil t)
         (list
          before
          (length
           (delq nil
                 (mapcar
                  (lambda (entry)
                    (and
                     (eq (car-safe entry)
                         'ace-jump-mode)
                     entry))
                  minor-mode-alist)))
          (assq 'ace-jump-mode
                minor-mode-alist)))"##;
    let expect = expect!["OK (1 1 (ace-jump-mode ace-jump-mode))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
