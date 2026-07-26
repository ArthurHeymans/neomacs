use super::assert_ace_jump_mode_parity;
use expect_test::expect;

#[test]
fn ace_jump_mode_public_commands_callable_metadata_matches() {
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
         '(ace-jump-mode-pop-mark
           ace-jump-char-mode
           ace-jump-word-mode
           ace-jump-line-mode
           ace-jump-mode
           ace-jump-quick-exchange
           ace-jump-move
           ace-jump-done))"##;
    let expect = expect![[
        r#"OK ((ace-jump-mode-pop-mark nil t (interactive nil) "Pop up a postion from ‘ace-jump-mode-mark-ring’, and jump back to that position" "ace-jump-mode.el") (ace-jump-char-mode (query-char) t (interactive (list (read-char "Query Char:"))) "AceJump char mode" "ace-jump-mode.el") (ace-jump-word-mode (head-char) t (interactive (list (if ace-jump-word-mode-use-query-char (read-char "Head Char:") nil))) "AceJump word mode.\nYou can set ‘ace-jump-word-mode-use-query-char’ to nil to prevent\nasking for a head char, that will mark all the word in current\nbuffer." "ace-jump-mode.el") (ace-jump-line-mode nil t (interactive nil) "AceJump line mode.\nMarked each no empty line and move there" "ace-jump-mode.el") (ace-jump-mode (&optional prefix) t (interactive "p") "AceJump mode is a minor mode for you to quick jump to a\nposition in the curret view.\n   There is three submode now:\n     ‘ace-jump-char-mode’\n     ‘ace-jump-word-mode’\n     ‘ace-jump-line-mode’\n\nYou can specify the sequence about which mode should enter\nby customize ‘ace-jump-mode-submode-list’.\n\nIf you do not want to query char for word mode, you can change\n‘ace-jump-word-mode-use-query-char’ to nil.\n\nIf you don’t like the default move keys, you can change it by\nsetting ‘ace-jump-mode-move-keys’.\n\nYou can constrol whether use the case sensitive via\n‘ace-jump-mode-case-fold’.\n" "ace-jump-mode.el") (ace-jump-quick-exchange nil t (interactive nil) "The function that we can use to quick exhange the current mode between\nword-mode and char-mode" "ace-jump-mode.el") (ace-jump-move nil t (interactive nil) "move cursor based on user input" "ace-jump-mode.el") (ace-jump-done nil t (interactive nil) "stop AceJump motion" "ace-jump-mode.el"))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_data_and_tree_callable_metadata_matches() {
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
         '(aj-queue-push
           aj-queue-pop
           ace-jump-char-category
           ace-jump-search-candidate
           ace-jump-tree-breadth-first-construct
           ace-jump-tree-preorder-traverse
           ace-jump-populate-overlay-to-search-tree
           ace-jump-delete-overlay-in-search-tree
           ace-jump-buffer-substring
           ace-jump-update-overlay-in-search-tree))"##;
    let expect = expect![[
        r#"OK ((aj-queue-push (item q) nil nil "enqueue" "ace-jump-mode.el") (aj-queue-pop (q) nil nil "dequeue" "ace-jump-mode.el") (ace-jump-char-category (query-char) nil nil "Detect the type of the char.\nFor the ascii table, refer to http://www.asciitable.com/\n\nThere is four possible return value:\n1. ’digit: the number character\n2. ’alpha: A-Z and a-z\n3. ’punc : all the printable punctuaiton\n4. ’other: all the others" "ace-jump-mode.el") (ace-jump-search-candidate (re-query-string visual-area-list) nil nil "Search the RE-QUERY-STRING in current view, and return the candidate position list.\nRE-QUERY-STRING should be an valid regex used for ‘search-forward-regexp’.\n\nYou can control whether use the case sensitive or not by ‘ace-jump-mode-case-fold’.\n\nEvery possible ‘match-beginning’ will be collected.\nThe returned value is a list of ‘aj-position’ record." "ace-jump-mode.el") (ace-jump-tree-breadth-first-construct (total-leaf-node max-child-node) nil nil "Constrct the search tree, each item in the tree is a cons cell.\nThe (car tree-node) is the type, which should be only ’branch or ’leaf.\nThe (cdr tree-node) is data stored in a leaf when type is ’leaf,\nwhile a child node list when type is ’branch" "ace-jump-mode.el") (ace-jump-tree-preorder-traverse (tree &optional leaf-func branch-func) nil nil "we move over tree via preorder, and call BRANCH-FUNC on each branch\nnode and call LEAF-FUNC on each leaf node" "ace-jump-mode.el") (ace-jump-populate-overlay-to-search-tree (tree candidate-list) nil nil "Populate the overlay to search tree, every leaf will give one overlay" "ace-jump-mode.el") (ace-jump-delete-overlay-in-search-tree (tree) nil nil "Delete all the overlay in search tree leaf node" "ace-jump-mode.el") (ace-jump-buffer-substring (pos) nil nil "Get the char under the POS, which is aj-position structure." "ace-jump-mode.el") (ace-jump-update-overlay-in-search-tree (tree keys) nil nil "Update overlay ’display property using each name in keys" "ace-jump-mode.el"))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_runtime_and_mark_callable_metadata_matches() {
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
         '(ace-jump-list-visual-area
           ace-jump-do
           ace-jump-jump-to
           ace-jump-push-mark
           ace-jump-kill-buffer
           ace-jump-move-to-end-if
           ace-jump-move-first-to-end-if
           ace-jump-mode-enable-mark-sync
           ace-jump-mode-disable-mark-sync))"##;
    let expect = expect![[
        r#"OK ((ace-jump-list-visual-area nil nil nil "Based on ‘ace-jump-mode-scope’, search the possible buffers that is showing now." "ace-jump-mode.el") (ace-jump-do (re-query-string) nil nil "The main function to start the AceJump mode.\nQUERY-STRING should be a valid regexp string, which finally pass to ‘search-forward-regexp’.\n\nYou can constrol whether use the case sensitive via ‘ace-jump-mode-case-fold’.\n" "ace-jump-mode.el") (ace-jump-jump-to (position) nil nil "Jump to the POSITION, which is a ‘aj-position’ structure storing the position information" "ace-jump-mode.el") (ace-jump-push-mark nil nil nil "Push the current position information onto the ‘ace-jump-mode-mark-ring’." "ace-jump-mode.el") (ace-jump-kill-buffer (buffer) nil nil "Utility function to kill buffer for ace jump mode.\nWe also need to handle the buffer which has clients on it" "ace-jump-mode.el") (ace-jump-move-to-end-if (l pred) nil nil "Move all the element in a list to the end of list if it make\nthe PRED to return non-nil.\n\nPRED is a function object which can pass to funcall and accept\none argument, which will be every element in the list.\nSuch as : (lambda (x) (equal x 1)) " "ace-jump-mode.el") (ace-jump-move-first-to-end-if (l pred) nil nil "Only move the first found one to the end of list" "ace-jump-mode.el") (ace-jump-mode-enable-mark-sync nil nil nil "Enable the sync funciton between ace jump mode mark ring and emacs mark ring.\n\n1. This function will enable the advice which activate on\n‘pop-mark’ and ‘pop-global-mark’. These advice will remove the\nsame marker from ‘ace-jump-mode-mark-ring’ when user use\n‘pop-mark’ or ‘global-pop-mark’ to jump back. \n\n2. Set variable ‘ace-jump-sync-emacs-mark-ring’ to t, which will\nsync mark information with emacs mark ring. " "ace-jump-mode.el") (ace-jump-mode-disable-mark-sync nil nil nil "Disable the sync funciton between ace jump mode mark ring and emacs mark ring.\n\n1. This function will diable the advice which activate on\n‘pop-mark’ and ‘pop-global-mark’. These advice will remove the\nsame marker from ‘ace-jump-mode-mark-ring’ when user use\n‘pop-mark’ or ‘global-pop-mark’ to jump back. \n\n2. Set variable ‘ace-jump-sync-emacs-mark-ring’ to nil, which\nwill stop synchronizing mark information with emacs mark ring. " "ace-jump-mode.el"))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_struct_surface_and_macro_expansions_match() {
    let elisp_form = r##"(list
         (macroexpand
          '(aj-position-buffer p))
         (macroexpand
          '(aj-position-window p))
         (macroexpand
          '(aj-position-frame p))
         (macroexpand
          '(aj-position-recover-buffer p))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (help-function-arglist symbol t)
             (file-name-nondirectory
              (symbol-file symbol 'defun))))
          '(make-aj-position
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
            aj-queue-tail)))"##;
    let expect = expect![[
        r#"OK ((aj-visual-area-buffer (aj-position-visual-area p)) (aj-visual-area-window (aj-position-visual-area p)) (aj-visual-area-frame (aj-position-visual-area p)) (aj-visual-area-recover-buffer (aj-position-visual-area p)) ((make-aj-position (&rest --cl-rest--) "ace-jump-mode.el") (copy-aj-position (arg) "ace-jump-mode.el") (aj-position-p #1=(x) "ace-jump-mode.el") (aj-position-offset #1# "ace-jump-mode.el") (aj-position-visual-area #1# "ace-jump-mode.el") (make-aj-visual-area (&rest --cl-rest--) "ace-jump-mode.el") (copy-aj-visual-area (arg) "ace-jump-mode.el") (aj-visual-area-p #1# "ace-jump-mode.el") (aj-visual-area-buffer #1# "ace-jump-mode.el") (aj-visual-area-window #1# "ace-jump-mode.el") (aj-visual-area-frame #1# "ace-jump-mode.el") (aj-visual-area-recover-buffer #1# "ace-jump-mode.el") (make-aj-queue (&rest --cl-rest--) "ace-jump-mode.el") (copy-aj-queue (arg) "ace-jump-mode.el") (aj-queue-p #1# "ace-jump-mode.el") (aj-queue-head #1# "ace-jump-mode.el") (aj-queue-tail #1# "ace-jump-mode.el")))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-jump-mode
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
                '("ace-jump-mode.el"
                  "ace-jump-mode-pkg.el"
                  "ace-jump-mode-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-jump-mode.el" 43161 "7f8c2abd6ba900a1cccb9c7cb96c3f7fb8cdcee2202235cdb4f6ddb64eb285fa") ("ace-jump-mode-pkg.el" 439 "ae56aa50439faf184d1056a1fb0c2726e24f7a45cec98b3af78cdeb78c993cf5") ("ace-jump-mode-autoloads.el" 1921 "424c9f09837cf71cdfa2d818ec0022e58d1b6e9318e918d6cffc56d413e92ac4") ("README-elpa" 1872 "bbbeabcc085264d0bff307828bcf30d7bd9283525bbf6f8410c74587e97af094"))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_installation_produces_local_byte_compilation_artifact() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-jump-mode
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor))
                     (path
                      (expand-file-name
                       "ace-jump-mode.elc"
                       directory)))
               (list
                (file-exists-p path)
                (file-regular-p path)
                (> (file-attribute-size
                    (file-attributes path))
                   0)))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_old_style_advice_objects_match() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let* ((function (car spec))
                  (name (cdr spec))
                  (advice
                   (ad-find-advice
                    function
                    'before
                    name)))
             (list
              function
              name
              (not (null advice))
              (ad-advice-enabled advice)
              (ad-advice-position
               function
               'before
               name)
              (ad-advice-protected advice))))
         '((pop-mark . ace-jump-pop-mark-advice)
           (pop-global-mark
            . ace-jump-pop-global-mark-advice)))"##;
    let expect = expect![
        "OK ((pop-mark ace-jump-pop-mark-advice t t 0 nil) (pop-global-mark ace-jump-pop-global-mark-advice t t 0 nil))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_position_macros_callable_metadata_and_source_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (macrop symbol)
            (help-function-arglist symbol t)
            (documentation symbol)
            (file-name-nondirectory
             (symbol-file symbol 'defun))))
         '(aj-position-buffer
           aj-position-window
           aj-position-frame
           aj-position-recover-buffer))"##;
    let expect = expect![[
        r#"OK ((aj-position-buffer t (aj-pos) "Get the buffer object from ‘aj-position’." "ace-jump-mode.el") (aj-position-window t (aj-pos) "Get the window object from ‘aj-position’." "ace-jump-mode.el") (aj-position-frame t (aj-pos) "Get the frame object from ‘aj-position’." "ace-jump-mode.el") (aj-position-recover-buffer t (aj-pos) "Get the recover-buffer object from ‘aj-position’." "ace-jump-mode.el"))"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
