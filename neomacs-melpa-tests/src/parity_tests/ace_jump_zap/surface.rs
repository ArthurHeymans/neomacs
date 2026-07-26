use super::assert_ace_jump_zap_parity;
use expect_test::expect;

#[test]
fn ace_jump_zap_public_commands_callable_metadata_matches() {
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
         '(ace-jump-zap-up-to-char
           ace-jump-zap-to-char
           ace-jump-zap-to-char-dwim
           ace-jump-zap-up-to-char-dwim))"##;
    let expect = expect![[
        r#"OK ((ace-jump-zap-up-to-char nil t (interactive nil) "Call ‘ace-jump-char-mode’ and zap all characters up to the selected character." "ace-jump-zap.el") (ace-jump-zap-to-char nil t (interactive nil) "Call ‘ace-jump-char-mode’ and zap all characters up to and including the selected character." "ace-jump-zap.el") (ace-jump-zap-to-char-dwim (&optional prefix) t (interactive "P") "Without PREFIX, call ‘zap-to-char’.\nWith PREFIX, call ‘ace-jump-zap-to-char’." "ace-jump-zap.el") (ace-jump-zap-up-to-char-dwim (&optional prefix) t (interactive "P") "Without PREFIX, call ‘zap-up-to-char’.\nWith PREFIX, call ‘ace-jump-zap-up-to-char’." "ace-jump-zap.el"))"#
    ]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_internal_callable_metadata_matches() {
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
         '(ajz/maybe-zap-start
           ajz/maybe-zap-end
           ajz/reset
           ajz/keyboard-reset
           ajz/forward-query
           ajz/closeness-to-point
           ajz/maybe-limit-candidate-length
           ajz/maybe-sort-candidate-list))"##;
    let expect = expect![[
        r#"OK ((ajz/maybe-zap-start nil nil nil "Push the mark when zapping with ‘ace-jump-char-mode’." "ace-jump-zap.el") (ajz/maybe-zap-end nil nil nil "Zap after jumping with ‘ace-jump-char-mode.’." "ace-jump-zap.el") (ajz/reset nil nil nil "Reset the internal zapping variable flags." "ace-jump-zap.el") (ajz/keyboard-reset nil t (interactive nil) "Reset when ‘ace-jump-mode’ is cancelled.\nAlso called when chosen character isn’t found while zapping." "ace-jump-zap.el") (ajz/forward-query nil nil nil "Filter for checking if jump candidate is after point." "ace-jump-zap.el") (ajz/closeness-to-point (c1 c2) nil nil "Compare C1 to C2 to determine closer candidate to point." "ace-jump-zap.el") (ajz/maybe-limit-candidate-length (args) nil nil "Limit the candidates to 52 when ‘ajz/52-character-limit’ is non-nil." "ace-jump-zap.el") (ajz/maybe-sort-candidate-list (args) nil nil "Maybe sort and limit the ‘ace-jump-mode’ node-tree." "ace-jump-zap.el"))"#
    ]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_declares_exact_builtin_up_to_char_autoload() {
    let elisp_form = r##"(let ((definition
              (symbol-function
               'zap-up-to-char)))
         (list
          (autoloadp definition)
          (nth 1 definition)
          (nth 2 definition)
          (nth 3 definition)
          (nth 4 definition)
          (commandp 'zap-up-to-char)))"##;
    let expect = expect![[
        r#"OK (t "misc" "Kill up to, but not including ARGth occurrence of CHAR." nil nil nil)"#
    ]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_internal_and_custom_variable_defaults_match() {
    let elisp_form = r##"(list
         ajz/zapping
         ajz/to-char
         ajz/saved-point
         ajz/zap-function
         ajz/forward-only
         ajz/sort-by-closest
         ajz/52-character-limit)"##;
    let expect = expect!["OK (nil nil nil delete-region nil t t)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_variable_metadata_and_source_ownership_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (special-variable-p symbol)
            (get symbol 'custom-type)
            (get symbol 'standard-value)
            (documentation-property
             symbol
             'variable-documentation
             t)
            (file-name-nondirectory
             (symbol-file symbol 'defvar))))
         '(ajz/zapping
           ajz/to-char
           ajz/saved-point
           ajz/zap-function
           ajz/forward-only
           ajz/sort-by-closest
           ajz/52-character-limit))"##;
    let expect = expect![[
        r#"OK ((ajz/zapping t nil nil "Internal flag for detecting if currently zapping." "ace-jump-zap.el") (ajz/to-char t nil nil "Internal flag for determining if zapping to-char or up-to-char." "ace-jump-zap.el") (ajz/saved-point t nil nil "Internal variable for caching the current point." "ace-jump-zap.el") (ajz/zap-function t nil ('delete-region) "This is the function used for zapping between point and char.\nThe default is `delete-region' but it could also be `kill-region'." "ace-jump-zap.el") (ajz/forward-only t nil (nil) "Set to non-nil to choose to only zap forward from the point.\nDefault will zap in both directions from the point in the current window." "ace-jump-zap.el") (ajz/sort-by-closest t nil (t) "Non-nil means sort the zap candidates by proximity to the current point.\nSet to nil for the default `ace-jump-mode' ordering.\nEnabled by default as of 0.1.0." "ace-jump-zap.el") (ajz/52-character-limit t nil (t) "Set to non-nil to limit zapping reach to the first 52 characters.\nEnabled by default as of 0.1.0." "ace-jump-zap.el"))"#
    ]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_hooks_are_registered_once_in_source_order() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ajz/reset
               'defun))
             (ace-jump-mode-before-jump-hook
              '(sentinel-before))
             (ace-jump-mode-end-hook
              '(sentinel-end)))
         (load path nil t)
         (load path nil t)
         (list
          ace-jump-mode-before-jump-hook
          ace-jump-mode-end-hook))"##;
    let expect =
        expect!["OK ((ajz/maybe-zap-start sentinel-before) (ajz/maybe-zap-end sentinel-end))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_filter_advices_are_installed_exactly_once() {
    let elisp_form = r##"(let ((limit-count 0)
             (sort-count 0)
             (path
              (symbol-file
               'ajz/reset
               'defun)))
         (load path nil t)
         (load path nil t)
         (advice-mapc
          (lambda (function _properties)
            (when
                (eq function
                    #'ajz/maybe-limit-candidate-length)
              (setq limit-count
                    (1+ limit-count))))
          'ace-jump-tree-breadth-first-construct)
         (advice-mapc
          (lambda (function _properties)
            (when
                (eq function
                    #'ajz/maybe-sort-candidate-list)
              (setq sort-count
                    (1+ sort-count))))
          'ace-jump-populate-overlay-to-search-tree)
         (list
          (not
           (null
            (advice-member-p
             #'ajz/maybe-limit-candidate-length
             'ace-jump-tree-breadth-first-construct)))
          limit-count
          (not
           (null
            (advice-member-p
             #'ajz/maybe-sort-candidate-list
             'ace-jump-populate-overlay-to-search-tree)))
          sort-count))"##;
    let expect = expect!["OK (t 1 t 1)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_source_reload_preserves_all_prebound_internal_and_custom_values() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ajz/reset
               'defun)))
         (setq ajz/zapping 'prebound-active)
         (setq ajz/to-char 'prebound-to)
         (setq ajz/saved-point 91)
         (setq ajz/zap-function 'prebound-zap)
         (setq ajz/forward-only 'prebound-forward)
         (setq ajz/sort-by-closest 'prebound-sort)
         (setq ajz/52-character-limit
               'prebound-limit)
         (load path nil t)
         (list
          ajz/zapping
          ajz/to-char
          ajz/saved-point
          ajz/zap-function
          ajz/forward-only
          ajz/sort-by-closest
          ajz/52-character-limit))"##;
    let expect = expect![
        "OK (prebound-active prebound-to 91 prebound-zap prebound-forward prebound-sort prebound-limit)"
    ];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-jump-zap
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
                '("ace-jump-zap.el"
                  "ace-jump-zap-pkg.el"
                  "ace-jump-zap-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-jump-zap.el" 5508 "eef0cd175e6a174b80fe2b2a7ac46b5faa9077bc041b707099e71b31d4d42e6f") ("ace-jump-zap-pkg.el" 499 "495054729d7f469eed1a2dff5ecdd00b90980a47ea3388ab04ce81854a805de3") ("ace-jump-zap-autoloads.el" 1302 "28e47a51c66205771a6c57032116158f4429539dac40290d42dd8faddde09d32") ("README-elpa" 97 "b9dae7a5da3788370c63695fd64e55eb1ecf25b30e80855b6722619db1690e0f"))"#
    ]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_installation_produces_local_byte_compilation_artifact() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-jump-zap
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor))
                     (path
                      (expand-file-name
                       "ace-jump-zap.elc"
                       directory)))
               (list
                (file-exists-p path)
                (file-regular-p path)
                (> (file-attribute-size
                    (file-attributes path))
                   0)))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}
