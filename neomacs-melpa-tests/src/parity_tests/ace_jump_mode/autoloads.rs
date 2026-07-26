use super::assert_ace_jump_mode_autoload_parity;
use expect_test::expect;

#[test]
fn ace_jump_mode_fresh_autoload_provides_only_autoload_feature() {
    let elisp_form = r##"(list
         (featurep 'ace-jump-mode-autoloads)
         (featurep 'ace-jump-mode)
         (featurep 'cl))"##;
    let expect = expect!["OK (t nil nil)"];
    assert_ace_jump_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_fresh_autoload_commands_have_exact_objects() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((definition
                  (symbol-function symbol)))
             (list
              symbol
              (autoloadp definition)
              (nth 1 definition)
              (nth 2 definition)
              (nth 3 definition)
              (nth 4 definition)
              (commandp symbol)
              (file-name-nondirectory
               (symbol-file symbol 'defun)))))
         '(ace-jump-mode-pop-mark
           ace-jump-char-mode
           ace-jump-word-mode
           ace-jump-line-mode
           ace-jump-mode))"##;
    let expect = expect![[
        r#"OK ((ace-jump-mode-pop-mark t "ace-jump-mode" "Pop up a postion from `ace-jump-mode-mark-ring', and jump back to that position" t nil t "ace-jump-mode.el") (ace-jump-char-mode t "ace-jump-mode" "AceJump char mode\n\n(fn QUERY-CHAR)" t nil t "ace-jump-mode.el") (ace-jump-word-mode t "ace-jump-mode" "AceJump word mode.\nYou can set `ace-jump-word-mode-use-query-char' to nil to prevent\nasking for a head char, that will mark all the word in current\nbuffer.\n\n(fn HEAD-CHAR)" t nil t "ace-jump-mode.el") (ace-jump-line-mode t "ace-jump-mode" "AceJump line mode.\nMarked each no empty line and move there" t nil t "ace-jump-mode.el") (ace-jump-mode t "ace-jump-mode" "AceJump mode is a minor mode for you to quick jump to a\nposition in the curret view.\n   There is three submode now:\n     `ace-jump-char-mode'\n     `ace-jump-word-mode'\n     `ace-jump-line-mode'\n\nYou can specify the sequence about which mode should enter\nby customize `ace-jump-mode-submode-list'.\n\nIf you do not want to query char for word mode, you can change\n`ace-jump-word-mode-use-query-char' to nil.\n\nIf you don't like the default move keys, you can change it by\nsetting `ace-jump-mode-move-keys'.\n\nYou can constrol whether use the case sensitive via\n`ace-jump-mode-case-fold'.\n\n(fn &optional PREFIX)" t nil t "ace-jump-mode.el"))"#
    ]];
    assert_ace_jump_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_fresh_autoload_leaves_private_surface_undefined() {
    let elisp_form = r##"(let ((functions
              '(aj-position-buffer
            aj-position-window
            aj-position-frame
            aj-position-recover-buffer
            make-aj-position
            copy-aj-position
            aj-position-p
            aj-position-offset
            aj-position-visual-area
            make-aj-visual-area
            copy-aj-visual-area
            aj-visual-area-p
            aj-visual-area-buffer
            aj-visual-area-window
            aj-visual-area-frame
            aj-visual-area-recover-buffer
            make-aj-queue
            copy-aj-queue
            aj-queue-p
            aj-queue-head
            aj-queue-tail
            aj-queue-push
            aj-queue-pop
            ace-jump-char-category
            ace-jump-search-candidate
            ace-jump-tree-breadth-first-construct
            ace-jump-tree-preorder-traverse
            ace-jump-populate-overlay-to-search-tree
            ace-jump-delete-overlay-in-search-tree
            ace-jump-buffer-substring
            ace-jump-update-overlay-in-search-tree
            ace-jump-list-visual-area
            ace-jump-do
            ace-jump-jump-to
            ace-jump-push-mark
            ace-jump-quick-exchange
            ace-jump-move
            ace-jump-done
            ace-jump-kill-buffer
            ace-jump-move-to-end-if
            ace-jump-move-first-to-end-if
            ace-jump-mode-enable-mark-sync
            ace-jump-mode-disable-mark-sync))
             (variables
              '(ace-jump-mode-scope
            ace-jump-word-mode-use-query-char
            ace-jump-mode-case-fold
            ace-jump-mode-mark-ring
            ace-jump-mode-mark-ring-max
            ace-jump-mode-gray-background
            ace-jump-mode-detect-punc
            ace-jump-mode-submode-list
            ace-jump-mode-move-keys
            ace-jump-mode
            ace-jump-background-overlay-list
            ace-jump-search-tree
            ace-jump-query-char
            ace-jump-current-mode
            ace-jump-sync-emacs-mark-ring
            ace-jump-search-filter
            ace-jump-mode-before-jump-hook
            ace-jump-mode-end-hook
            ace-jump-allow-invisible)))
         (list
          (length functions)
          (delq nil
                (mapcar
                 (lambda (symbol)
                   (and
                    (fboundp symbol)
                    symbol))
                 functions))
          (length variables)
          (delq nil
                (mapcar
                 (lambda (symbol)
                   (and
                    (boundp symbol)
                    symbol))
                 variables))))"##;
    let expect = expect!["OK (43 nil 19 nil)"];
    assert_ace_jump_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_fresh_autoload_registers_definition_prefix() {
    let elisp_form = r##"(list
         (gethash
          "ace-jump-"
          definition-prefixes)
         (gethash
          "aj-"
          definition-prefixes)
         (gethash
          "ace-jump-mode"
          definition-prefixes))"##;
    let expect = expect![[
        r#"OK (("ace-jump-mode" "ace-jump-mode") ("ace-jump-mode" "ace-jump-mode") nil)"#
    ]];
    assert_ace_jump_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_autoload_invocation_loads_feature_before_body_error() {
    let elisp_form = r##"(condition-case error-data
         (ace-jump-char-mode 0)
       (error
        (list
         (featurep 'ace-jump-mode)
         (car error-data)
         (cdr error-data)
         (autoloadp
          (symbol-function
           'ace-jump-char-mode)))))"##;
    let expect = expect![[r#"OK (t error ("[AceJump] Non-printable character") nil)"#]];
    assert_ace_jump_mode_autoload_parity(elisp_form, expect);
}
