use expect_test::expect;

use super::{assert_agtags_autoload_parity, assert_agtags_parity};

#[test]
fn agtags_defaults_custom_metadata_constants_and_mutable_state_match() {
    let elisp_form = r##"(list
         (featurep 'agtags)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value symbol)
             (get symbol 'custom-type)
             (get symbol 'custom-group)
             (get symbol 'safe-local-variable)
             (local-variable-if-set-p symbol)))
          '(agtags-key-prefix
            agtags-global-ignore-case
            agtags-global-treat-text))
         agtags-created-tag-files
         agtags--history-list
         agtags--global-to-list-cache
         agtags--display-buffer-dwim
         (functionp agtags--completion-table)
         (list
          (length agtags--global-mode-font-lock-keywords)
          (length agtags--path-regexp-alist)
          (length agtags--grep-regexp-alist)))"##;
    let expect = expect![[
        r#"OK (t ((agtags-key-prefix "C-c t" string nil stringp nil) (agtags-global-ignore-case nil boolean nil booleanp nil) (agtags-global-treat-text nil boolean nil booleanp nil)) ("GPATH" "GTAGS" "GRTAGS") nil nil ((display-buffer-reuse-window display-buffer-same-window) (inhibit-same-window)) t (2 1 1))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_complete_callable_surface_arglists_and_command_status_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (macrop symbol)
            (autoloadp (symbol-function symbol))))
         '(agtags--fix-param
           agtags--quote-text
           agtags--parse-root
           agtags--is-active
           agtags--run-global-to-list
           agtags--run-cached-global-to-list
           agtags--run-global-to-mode
           agtags--run-global-completing
           agtags--read-dwim
           agtags--read-input
           agtags--read-input-dwim
           agtags--read-completing
           agtags--read-completing-dwim
           agtags--auto-update
           agtags--compile-goto-error
           agtags--global-mode-finished
           agtags-grep-mode
           agtags-path-mode
           agtags--completion-at-point
           agtags-xref--make-xref
           agtags-xref--find-symbol
           agtags-xref--backend
           xref-backend-identifier-at-point
           xref-backend-identifier-completion-table
           xref-backend-definitions
           xref-backend-references
           xref-backend-apropos
           agtags-mode
           agtags-update-tags
           agtags-open-file
           agtags-find-file
           agtags-find-tag
           agtags-find-rtag
           agtags-find-with-pattern
           agtags-find-with-string
           agtags-switch-dwim
           agtags-bind-keys))"##;
    let expect = expect![
        "OK ((agtags--fix-param (string) nil nil nil) (agtags--quote-text (string) nil nil nil) (agtags--parse-root nil nil nil nil) (agtags--is-active (dir) nil nil nil) (agtags--run-global-to-list (arguments &optional dir) nil nil nil) (agtags--run-cached-global-to-list (arguments) nil nil nil) (agtags--run-global-to-mode (string args &optional result) nil nil nil) (agtags--run-global-completing (flag string predicate code) nil nil nil) (agtags--read-dwim nil nil nil nil) (agtags--read-input (prompt) nil nil nil) (agtags--read-input-dwim (prompt) nil nil nil) (agtags--read-completing (flag prompt) nil nil nil) (agtags--read-completing-dwim (flag prompt) nil nil nil) (agtags--auto-update nil nil nil nil) (agtags--compile-goto-error (orig-fun &rest args) nil nil nil) (agtags--global-mode-finished (buffer _tatus) nil nil nil) (agtags-grep-mode nil t nil nil) (agtags-path-mode nil t nil nil) (agtags--completion-at-point nil nil nil nil) (agtags-xref--make-xref (ctags-x-line) nil nil nil) (agtags-xref--find-symbol (symbol &rest args) nil nil nil) (agtags-xref--backend nil nil nil nil) (xref-backend-identifier-at-point (backend) nil nil nil) (xref-backend-identifier-completion-table (backend) nil nil nil) (xref-backend-definitions (backend identifier) nil nil nil) (xref-backend-references (backend identifier) nil nil nil) (xref-backend-apropos (backend pattern) nil nil nil) (agtags-mode (&optional arg) t nil nil) (agtags-update-tags nil t nil nil) (agtags-open-file nil t nil nil) (agtags-find-file nil t nil nil) (agtags-find-tag nil t nil nil) (agtags-find-rtag nil t nil nil) (agtags-find-with-pattern nil t nil nil) (agtags-find-with-string nil t nil nil) (agtags-switch-dwim nil t nil nil) (agtags-bind-keys nil nil nil nil))"
    ];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_mode_maps_parents_navigation_bindings_and_mode_metadata_match() {
    let elisp_form = r##"(list
         (eq
          (keymap-parent agtags--global-mode-map)
          special-mode-map)
         (mapcar
          (lambda (key)
            (list
             key
             (lookup-key
              agtags--global-mode-map
              (kbd key))))
          '("<follow-link>" "<mouse-2>" "RET" "C-m"
            "g" "n" "p" "{" "}"))
         (eq agtags-grep-mode-map
             agtags--global-mode-map)
         (eq agtags-path-mode-map
             agtags--global-mode-map)
         (get 'agtags-grep-mode 'derived-mode-parent)
         (get 'agtags-path-mode 'derived-mode-parent)
         (get 'agtags-mode 'variable-documentation)
         (advice-member-p
          #'agtags--compile-goto-error
          'compile-goto-error))"##;
    let expect = expect![[
        r#"OK (t (("<follow-link>" mouse-face) ("<mouse-2>" compile-goto-error) ("RET" compile-goto-error) ("C-m" compile-goto-error) ("g" recompile) ("n" compilation-next-error) ("p" compilation-previous-error) ("{" compilation-previous-file) ("}" compilation-next-file)) t t grep-mode compilation-mode "Non-nil if AGtags mode is enabled.\nUse the command `agtags-mode' to change this variable." #[128 "������\3#��" [agtags--compile-goto-error #[(&optional event) ((if event (posn-set-point (event-end event))) (or (compilation-buffer-p (current-buffer)) (error "Not in a compilation buffer")) (compilation--ensure-parse (point)) (if (get-text-property (point) 'compilation-directory) (dired-other-window (car (get-text-property (point) 'compilation-directory))) (setq compilation-current-error (point)) (next-error-internal))) (cl-struct-compilation--message-tags t) nil "Visit the source for the error message at point.\nUse this command in a compilation log buffer." (list last-input-event)] :around nil apply] 5 advice])"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_regexp_contracts_parse_paths_lines_and_colored_match_columns() {
    let elisp_form = r##"(let* ((path-regexp
                  (caar
                   agtags--path-regexp-alist))
                 (grep-entry
                  (car
                   agtags--grep-regexp-alist))
                 (grep-regexp
                  (car grep-entry))
                 (column-function
                  (car
                   (nth 3 grep-entry))))
         (list
          (mapcar
           (lambda (line)
             (list line
                   (and
                    (string-match
                     path-regexp line)
                    (match-string 0 line))))
           '("src/main.c"
             "path with space.c"
             "\"quoted.c\""
             "nested/header.h"))
          (mapcar
           (lambda (line)
             (with-temp-buffer
               (insert line)
               (goto-char (point-min))
               (when
                   (re-search-forward
                    grep-regexp nil t)
                 (list
                  (match-string 1)
                  (match-string 2)
                  (funcall
                   column-function)))))
           (list
            "src/main.c:12:plain match"
            (concat
             "src/main.c:12:"
             (propertize
              "colored"
              'global-color t))
            "bad line"))))"##;
    let expect = expect![[
        r#"OK ((("src/main.c" "src/main.c") ("path with space.c" nil) ("\"quoted.c\"" "\"quoted.c\"") ("nested/header.h" "nested/header.h")) (("src/main.c" "12" nil) ("src/main.c" "12" 0) nil))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_autoloads_register_only_annotated_modes_and_key_binding_entrypoint() {
    let elisp_form = r##"(list
         (featurep 'agtags)
         (mapcar
          (lambda (symbol)
            (let ((definition
                   (symbol-function symbol)))
              (list
               symbol
               (autoloadp definition)
               (and
                (autoloadp definition)
                (nth 1 definition))
               (commandp symbol)
               (help-function-arglist symbol t))))
          '(agtags-grep-mode
            agtags-path-mode
            agtags-mode
            agtags-bind-keys))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)))
          '(agtags-update-tags
            agtags-open-file
            agtags-find-file
            agtags-find-tag)))"##;
    let expect = expect![[
        r#"OK (nil ((agtags-grep-mode t "agtags" t "[Arg list not available until function definition is loaded.]") (agtags-path-mode nil nil t nil) (agtags-mode t "agtags" t "[Arg list not available until function definition is loaded.]") (agtags-bind-keys t "agtags" nil "[Arg list not available until function definition is loaded.]")) ((agtags-update-tags nil) (agtags-open-file nil) (agtags-find-file nil) (agtags-find-tag nil)))"#
    ]];
    assert_agtags_autoload_parity(elisp_form, expect);
}
