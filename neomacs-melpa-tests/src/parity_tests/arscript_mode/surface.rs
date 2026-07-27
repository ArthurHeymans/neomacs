use expect_test::expect;

use super::{assert_arscript_mode_autoload_parity, assert_arscript_mode_parity};

#[test]
fn installed_descriptor_and_runtime_bytes_identify_the_exact_pinned_melpa_build() {
    let elisp_form = r##"(let* ((descriptor
         (cadr (assq 'arscript-mode package-alist)))
       (directory (package-desc-dir descriptor)))
  (list
   (featurep 'arscript-mode)
   (package-installed-p 'arscript-mode)
   (package-desc-name descriptor)
   (package-version-join (package-desc-version descriptor))
   (package-desc-reqs descriptor)
   (package-desc-summary descriptor)
   (package-desc-extras descriptor)
   (sort
    (directory-files directory nil "\\.el\\'")
    #'string<)
   (mapcar
    (lambda (name)
      (let ((path (expand-file-name name directory)))
        (list
         name
         (file-attribute-size (file-attributes path))
         (with-temp-buffer
           (set-buffer-multibyte nil)
           (insert-file-contents-literally path)
           (secure-hash 'sha256 (current-buffer))))))
    '("arscript-mode.el" "arscript-mode-pkg.el"))))"##;
    let expect = expect![[
        r#"OK (t t arscript-mode "20240819.1927" ((emacs (25 1))) "Major mode for editing arscript files." ((:maintainers ("James Dyer" . "captainflasmr@gmail.com")) (:authors ("James Dyer" . "captainflasmr@gmail.com")) (:keywords "convenience") (:revdesc . "797e1d0ef131") (:commit . "797e1d0ef1312e8ff846abd0c6853358041f7691") (:url . "https://github.com/captainflasmr/arscript-mode")) ("arscript-mode-autoloads.el" "arscript-mode-pkg.el" "arscript-mode.el") (("arscript-mode.el" 6087 "b17348600cad7dc889ea805f688494d7375596f22183831655f07c8a1845c596") ("arscript-mode-pkg.el" 446 "e092986544947071f25089f72dda1967adb7f74dd444c6f42187c2602ff1153c")))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn autoload_is_case_sensitive_and_matching_a_file_poisonously_changes_global_editing_defaults() {
    let elisp_form = r##"(list
   (featurep 'arscript-mode)
   (featurep 'arscript-mode-autoloads)
   (mapcar
    (lambda (symbol)
      (list
       symbol
       (fboundp symbol)
       (and (fboundp symbol)
            (autoloadp (symbol-function symbol)))
       (commandp symbol)))
    '(arscript-mode arscript-indent-line))
   (seq-filter
    (lambda (entry)
      (eq (cdr entry) 'arscript-mode))
    auto-mode-alist)
   (mapcar
    (lambda (filename)
      (let ((buffer
             (get-buffer-create
              (concat " *arscript-autoload-" filename "*"))))
        (unwind-protect
            (with-current-buffer buffer
              (setq buffer-file-name filename)
              (set-auto-mode)
              (list
               filename
               major-mode
               (eq indent-line-function
                   #'arscript-indent-line)
               comment-start))
          (kill-buffer buffer))))
    '("before.txt"
      "painting.arscript"
      "painting.ARSCRIPT"
      "painting.arscript.backup"
      ".arscript"
      "after.txt")))"##;
    let expect = expect![[
        r#"OK (nil t ((arscript-mode t t t) (arscript-indent-line nil nil nil)) (("\\.arscript\\'" . arscript-mode)) (("before.txt" text-mode nil nil) ("painting.arscript" fundamental-mode t "//") ("painting.ARSCRIPT" fundamental-mode t "//") ("painting.arscript.backup" fundamental-mode t "//") (".arscript" fundamental-mode t "//") ("after.txt" text-mode t "//")))"#
    ]];
    assert_arscript_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn complete_package_owned_callable_surface_has_exact_arguments_and_interactivity() {
    let elisp_form = r##"(let* ((source
         (file-truename (getenv "NEOMACS_PACKAGE_SOURCE")))
       (symbols
        (sort
         (seq-filter
          (lambda (symbol)
            (and
             (fboundp symbol)
             (let ((file (symbol-file symbol 'defun)))
               (and file
                    (string=
                     source
                     (file-truename file))))))
          (apropos-internal "^arscript-"))
         (lambda (left right)
           (string< (symbol-name left)
                    (symbol-name right))))))
  (mapcar
   (lambda (symbol)
     (list
      symbol
      (copy-tree (help-function-arglist symbol t))
      (commandp symbol)
      (interactive-form symbol)
      (file-name-nondirectory
       (symbol-file symbol 'defun))))
   symbols))"##;
    let expect = expect![[
        r#"OK ((arscript-indent-line nil t (interactive nil) "arscript-mode.el") (arscript-mode nil t (interactive nil) "arscript-mode.el"))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn entering_mode_preserves_the_upstream_identity_reset_bug_but_installs_editing_state() {
    let elisp_form = r##"(with-temp-buffer
  (let ((events nil)
        (arscript-mode-hook nil))
    (setq arscript-mode-hook
          (list
           (lambda ()
             (push
              (list major-mode mode-name
                    (buffer-string))
              events))))
    (insert
     "<Header>\n"
     "Painting Name: \"Willow\"\n"
     "</Header>\n")
    (set-buffer-modified-p nil)
    (arscript-mode)
    (list
     major-mode
     mode-name
     (get 'arscript-mode 'derived-mode-parent)
     (eq indent-line-function
         #'arscript-indent-line)
     indent-tabs-mode
     comment-start
     comment-end
     (local-variable-p 'font-lock-defaults)
     (length (car font-lock-defaults))
     (eq (current-local-map)
         arscript-mode-map)
     (eq (syntax-table)
         arscript-mode-syntax-table)
     (buffer-string)
     (buffer-modified-p)
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK (fundamental-mode "Fundamental" prog-mode t nil "//" "" t 10 nil nil "<Header>\nPainting Name: \"Willow\"\n</Header>\n" nil ((fundamental-mode "Fundamental" "<Header>\nPainting Name: \"Willow\"\n</Header>\n")))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn mode_reinitialization_erases_preexisting_buffer_locals_before_user_hooks_run() {
    let elisp_form = r##"(with-temp-buffer
  (let ((arscript-mode-hook nil)
        hook-observation)
    (setq-local tab-width 7)
    (setq-local comment-start "CUSTOM")
    (setq-local case-fold-search 'custom)
    (setq-local arscript-test-local 'discard-me)
    (setq arscript-mode-hook
          (list
           (lambda ()
             (setq hook-observation
                   (list
                    tab-width
                    comment-start
                    case-fold-search
                    (local-variable-p
                     'arscript-test-local))))))
    (arscript-mode)
    (list
     tab-width
     comment-start
     case-fold-search
     (local-variable-p 'arscript-test-local)
     hook-observation)))"##;
    let expect = expect![[r#"OK (8 "//" t nil (8 "//" t nil))"#]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn mode_has_no_package_configuration_variables_key_bindings_or_process_commands() {
    let elisp_form = r##"(let ((symbols
        (sort
         (apropos-internal "^arscript-")
         (lambda (left right)
           (string< (symbol-name left)
                    (symbol-name right))))))
  (list
   (mapcar
    (lambda (symbol)
      (list
       symbol
       (fboundp symbol)
       (boundp symbol)
       (custom-variable-p symbol)
       (commandp symbol)))
    symbols)
   (where-is-internal
    'arscript-indent-line arscript-mode-map)
   (where-is-internal
    'arscript-mode arscript-mode-map)
   (seq-filter
    (lambda (symbol)
      (and
       (commandp symbol)
       (string-match-p
        "\\(?:process\\|run\\|execute\\)"
        (symbol-name symbol))))
    symbols)))"##;
    let expect = expect![
        "OK (((arscript-indent-line t nil nil t) (arscript-mode t nil nil t) (arscript-mode-abbrev-table nil t nil nil) (arscript-mode-autoloads nil nil nil nil) (arscript-mode-hook nil t nil nil) (arscript-mode-map nil t nil nil) (arscript-mode-syntax-table nil t nil nil)) nil nil nil)"
    ];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn slash_comments_are_font_lock_only_and_do_not_change_syntax_parse_state() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "// full-line annotation\n"
   "Painting Name: \"// literal-looking text\"\n")
  (arscript-mode)
  (font-lock-ensure)
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (let* ((position (match-beginning 0))
            (state (syntax-ppss position)))
       (list
        needle
        (get-text-property position 'face)
        (nth 3 state)
        (nth 4 state)
        (char-syntax (char-after position)))))
   '("full-line"
     "Painting"
     "literal-looking")))"##;
    let expect = expect![[
        r#"OK (("full-line" font-lock-comment-face nil nil 119) ("Painting" font-lock-keyword-face nil nil 119) ("literal-looking" font-lock-string-face 34 nil 119))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn raw_font_lock_rules_preserve_space_only_matching_in_wait_and_scalar_patterns() {
    let elisp_form = r##"(with-temp-buffer
  (arscript-mode)
  (let* ((rules (car font-lock-defaults))
         (wait-rule (nth 8 rules))
         (numeric-rule (nth 9 rules)))
    (list
     (car wait-rule)
     (string-to-list (car wait-rule))
     (string-match (car wait-rule)
                   "Wait: 0.018s")
     (string-match (car wait-rule)
                   "Wait:\t0.018s")
     (string-match (car wait-rule)
                   "Wait:ss0.018s")
     (car numeric-rule)
     (string-to-list (car numeric-rule))
     (string-match (car numeric-rule)
                   "Pr: 0.237271")
     (string-match (car numeric-rule)
                   "Pr:\t0.237271")
     (string-match (car numeric-rule)
                   "Pr:ss0.237271"))))"##;
    let expect = expect![[
        r#"OK ("\\(Wait:\\) +\\([0-9\\.s]+\\)" (92 40 87 97 105 116 58 92 41 32 43 92 40 91 48 45 57 92 46 115 93 43 92 41) 0 nil nil "\\([A-Za-z]+:\\) +\\([0-9\\.s]+\\)" (92 40 91 65 45 90 97 45 122 93 43 58 92 41 32 43 92 40 91 48 45 57 92 46 115 93 43 92 41) 0 nil nil)"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}
