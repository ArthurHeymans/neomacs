use expect_test::expect;

use super::{assert_aggressive_indent_autoload_parity, assert_aggressive_indent_parity};

#[test]
fn aggressive_indent_defaults_custom_metadata_and_buffer_local_state_match() {
    let elisp_form = r##"(list
         (featurep 'aggressive-indent)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value symbol)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (local-variable-if-set-p symbol)))
          '(aggressive-indent-dont-electric-modes
            aggressive-indent-excluded-modes
            aggressive-indent-protected-commands
            aggressive-indent-protected-current-commands
            aggressive-indent-comments-too
            aggressive-indent-modes-to-prefer-defun
            aggressive-indent-dont-indent-if
            aggressive-indent-stop-here-hook
            aggressive-indent-region-function
            aggressive-indent-sit-for-time
            aggressive-indent--changed-list
            aggressive-indent--idle-timer))
         (with-temp-buffer
           (setq aggressive-indent--changed-list '((2 4))
                 aggressive-indent--idle-timer 'sentinel)
           (list
            aggressive-indent--changed-list
            aggressive-indent--idle-timer
            (local-variable-p 'aggressive-indent--changed-list)
            (local-variable-p 'aggressive-indent--idle-timer))))"##;
    let expect = expect![[
        r#"OK (t ((aggressive-indent-dont-electric-modes nil (choice (const :tag "Never use `electric-indent-mode'." t) (repeat :tag "List of major-modes to avoid `electric-indent-mode'." symbol)) nil nil) (aggressive-indent-excluded-modes (elm-mode haskell-mode inf-ruby-mode makefile-mode makefile-gmake-mode python-mode sql-interactive-mode text-mode yaml-mode) (repeat symbol) nil nil) (aggressive-indent-protected-commands (undo undo-tree-undo undo-tree-redo undo-tree-visualize undo-tree-visualize-undo undo-tree-visualize-redo whitespace-cleanup) (repeat symbol) nil nil) (aggressive-indent-protected-current-commands (query-replace-regexp query-replace exit-minibuffer) (repeat symbol) nil nil) (aggressive-indent-comments-too nil boolean nil nil) (aggressive-indent-modes-to-prefer-defun (emacs-lisp-mode lisp-mode scheme-mode clojure-mode) (repeat symbol) nil nil) (aggressive-indent-dont-indent-if nil (repeat sexp) nil nil) (aggressive-indent-stop-here-hook nil hook nil nil) (aggressive-indent-region-function indent-region function nil nil) (aggressive-indent-sit-for-time 0.05 float nil nil) (aggressive-indent--changed-list nil nil nil t) (aggressive-indent--idle-timer nil nil nil t)) (((2 4)) sentinel t t))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_complete_callable_macro_alias_and_command_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (macrop symbol)
            (autoloadp (symbol-function symbol))))
         '(aggressive-indent-bug-report
           aggressive-indent--run-user-hooks
           aggressive-indent-indent-defun
           aggressive-indent--softly-indent-defun
           aggressive-indent--indent-current-balanced-line
           aggressive-indent--extend-end-to-whole-sexps
           aggressive-indent-indent-region-and-on
           aggressive-indent--softly-indent-region-and-on
           aggressive-indent--process-changed-list-and-indent
           aggressive-indent--clear-change-list
           aggressive-indent--while-no-input
           aggressive-indent--maybe-cancel-timer
           aggressive-indent--indent-if-changed
           aggressive-indent--keep-track-of-changes
           aggressive-indent-mode
           aggressive-indent--local-electric
           global-aggressive-indent-mode
           aggressive-indent-global-mode))"##;
    let expect = expect![
        "OK ((aggressive-indent-bug-report nil t nil nil) (aggressive-indent--run-user-hooks nil nil nil nil) (aggressive-indent-indent-defun (&optional l r) t nil nil) (aggressive-indent--softly-indent-defun (&optional l r) nil nil nil) (aggressive-indent--indent-current-balanced-line (column) nil nil nil) (aggressive-indent--extend-end-to-whole-sexps (beg end) nil nil nil) (aggressive-indent-indent-region-and-on (l r) t nil nil) (aggressive-indent--softly-indent-region-and-on (l r &rest _) nil nil nil) (aggressive-indent--process-changed-list-and-indent nil nil nil nil) (aggressive-indent--clear-change-list nil nil nil nil) (aggressive-indent--while-no-input (&rest body) nil t nil) (aggressive-indent--maybe-cancel-timer nil nil nil nil) (aggressive-indent--indent-if-changed (buffer) nil nil nil) (aggressive-indent--keep-track-of-changes (l r &rest _) nil nil nil) (aggressive-indent-mode (&optional arg) t nil nil) (aggressive-indent--local-electric (on) nil nil nil) (global-aggressive-indent-mode #1=(&optional arg) t nil nil) (aggressive-indent-global-mode #1# t nil nil))"
    ];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_internal_guards_preserve_order_and_protect_real_editing_states() {
    let elisp_form = r##"(let ((forms aggressive-indent--internal-dont-indent-if))
         (with-temp-buffer
           (emacs-lisp-mode)
           (insert "(message \"inside\") ; comment")
           (goto-char (point-min))
           (search-forward "inside")
           (let ((inside-string
                  (run-hook-wrapped
                   'aggressive-indent--internal-dont-indent-if
                   #'eval)))
             (search-forward "comment")
             (let ((inside-comment
                    (run-hook-wrapped
                     'aggressive-indent--internal-dont-indent-if
                     #'eval)))
               (goto-char (point-min))
               (let ((last-command 'undo)
                     (this-command 'self-insert-command))
                 (list
                  (length forms)
                  (car forms)
                  (car (last forms))
                  inside-string
                  inside-comment
                  (run-hook-wrapped
                   'aggressive-indent--internal-dont-indent-if
                   #'eval)))))))"##;
    let expect = expect![
        "OK (10 (memq last-command aggressive-indent-protected-commands) (let ((sp (syntax-ppss))) (or (and (not aggressive-indent-comments-too) (elt sp 4)) (elt sp 3))) 34 t (undo undo-tree-undo undo-tree-redo undo-tree-visualize undo-tree-visualize-undo undo-tree-visualize-redo whitespace-cleanup))"
    ];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_deferred_integrations_append_all_available_package_guards() {
    let elisp_form = r##"(progn
         (defvar yas--active-field-overlay nil)
         (defvar company-candidates nil)
         (defvar ac-completing nil)
         (defvar multiple-cursors-mode nil)
         (defvar iedit-mode nil)
         (mapc #'provide
               '(yasnippet company auto-complete
                 multiple-cursors-core iedit evil coq ruby-mode))
         (list
          (length aggressive-indent--internal-dont-indent-if)
          (cl-subseq
           aggressive-indent--internal-dont-indent-if
           10)
          (let ((yas--active-field-overlay
                 (make-overlay 1 1)))
            (with-temp-buffer
              (move-overlay
               yas--active-field-overlay
               (point-min)
               (point-max)
               (current-buffer))
              (prog1
                  (run-hook-wrapped
                   'aggressive-indent--internal-dont-indent-if
                   #'eval)
                (delete-overlay
                 yas--active-field-overlay))))
          (let ((company-candidates '("candidate")))
            (run-hook-wrapped
             'aggressive-indent--internal-dont-indent-if
             #'eval))))"##;
    let expect = expect![[
        r#"OK (17 (undo-in-progress (null (buffer-modified-p)) (and (boundp 'smerge-mode) smerge-mode) (equal (buffer-name) "*ediff-merge*") (let ((line (thing-at-point 'line))) (and (stringp line) (stringp comment-start) (string-match (concat "\\`[[:blank:]]*" (substring comment-start 0 1) "[[:blank:]]*$") line))) (let ((sp (syntax-ppss))) (or (and (not aggressive-indent-comments-too) (elt sp 4)) (elt sp 3))) (and (overlayp yas--active-field-overlay) (overlay-end yas--active-field-overlay))) t ("candidate"))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_autoloads_register_commands_alias_and_interactive_contracts() {
    let elisp_form = r##"(list
         (featurep 'aggressive-indent)
         (mapcar
          (lambda (symbol)
            (let ((definition (symbol-function symbol)))
              (list
               symbol
               (autoloadp definition)
               (and
                (autoloadp definition)
                (nth 1 definition))
               (commandp symbol)
               (help-function-arglist symbol t))))
          '(aggressive-indent-indent-defun
            aggressive-indent-indent-region-and-on
            aggressive-indent-mode
            global-aggressive-indent-mode
            aggressive-indent-global-mode))
         (eq
          (indirect-function 'aggressive-indent-global-mode)
          (indirect-function 'global-aggressive-indent-mode))
         (assoc
          "aggressive-indent"
          package--builtin-versions))"##;
    let expect = expect![[
        r#"OK (nil ((aggressive-indent-indent-defun t "aggressive-indent" t "[Arg list not available until function definition is loaded.]") (aggressive-indent-indent-region-and-on t "aggressive-indent" t "[Arg list not available until function definition is loaded.]") (aggressive-indent-mode t "aggressive-indent" t "[Arg list not available until function definition is loaded.]") (global-aggressive-indent-mode t "aggressive-indent" t "[Arg list not available until function definition is loaded.]") (aggressive-indent-global-mode nil nil t "[Arg list not available until function definition is loaded.]")) t nil)"#
    ]];
    assert_aggressive_indent_autoload_parity(elisp_form, expect);
}
