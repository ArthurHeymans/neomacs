use expect_test::expect;

use super::{assert_act_mode_autoload_parity, assert_act_mode_parity};

#[test]
fn act_mode_exact_pin_metadata_feature_variables_docs_and_regex_registry_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq
                  'act-mode
                  package-alist))))
         (list
          (package-desc-name
           descriptor)
          (package-version-join
           (package-desc-version
            descriptor))
          (package-desc-reqs
           descriptor)
          (package-desc-summary
           descriptor)
          (copy-tree
           (package-desc-extras
            descriptor))
          (featurep
           'act-mode)
          (mapcar
           (lambda (symbol)
             (list
              symbol
              (boundp symbol)
              (default-boundp symbol)
              (default-value symbol)
              (local-variable-if-set-p
               symbol)
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
           '(act-keywords
             act-types
             act-functions
             act-fontlock))))"##;
    let expect = expect![[
        r#"OK (act-mode "20240718.39" ((emacs (26 1))) "Major mode for the ACT programming language." ((:maintainers ("Rafael Campos Nunes" . "rcamposnunes@outlook.com")) (:authors ("Rafael Campos Nunes" . "rcamposnunes@outlook.com")) (:revdesc . "90d7d6266915") (:commit . "90d7d626691591b24d83596149bc89fd51ba39b4") (:url . "https://github.com/rafaelcn/act")) t ((act-keywords t t ("export" "import") nil "List of keywords in act." "act-mode.el") (act-types t t ("preal" "pint" "bool" "int" "e1of" "e2of" "e3of" "c1of" "globals" "globals_np") nil "List of types in act." "act-mode.el") (act-functions t t ("defproc" "deftype" "defchan" "prs") nil "List of functions in act." "act-mode.el") (act-fontlock t t (("//.*" . font-lock-comment-face) ("\\<\\(\\(?:ex\\|im\\)port\\)\\>" . font-lock-keyword-face) ("\\<\\(def\\(?:chan\\|proc\\|type\\)\\|prs\\)\\>" . font-lock-function-name-face) ("\\<\\(bool\\|c1of\\|e\\(?:[123]of\\)\\|globals\\(?:_np\\)?\\|int\\|p\\(?:int\\|real\\)\\)\\>" . font-lock-type-face) ("<[[:digit:]]+>" . font-lock-constant-face)) nil "List for font-lock defaults." "act-mode.el")))"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_callable_command_doc_parent_and_auto_mode_registration_match() {
    let elisp_form = r##"(list
         (fboundp
          'act-mode)
         (help-function-arglist
          'act-mode
          t)
         (commandp
          'act-mode)
         (interactive-form
          'act-mode)
         (documentation
          'act-mode
          t)
         (let ((file
                (symbol-file
                 'act-mode
                 'defun)))
           (and file
                (file-name-nondirectory
                 file)))
         (get
          'act-mode
          'derived-mode-parent)
         (rassq
          'act-mode
          auto-mode-alist)
         (seq-filter
          (lambda (entry)
            (eq
             (cdr entry)
             'act-mode))
          auto-mode-alist))"##;
    let expect = expect![[
        r#"OK (t nil t (interactive nil) "Major mode for the act programming language.\n\nIn addition to any hooks its parent mode `prog-mode' might have run,\nthis mode runs the hook `act-mode-hook', as the final or penultimate\nstep during initialization.\n\n\\{act-mode-map}" "act-mode.el" prog-mode #1=("\\.act\\'" . act-mode) (#1#))"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_installed_package_inventory_and_content_assets_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'act-mode
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor))
                 (names
                  (sort
                   (directory-files
                    directory
                    nil
                    "^[^.].*")
                   #'string<)))
         (list
          names
          (mapcar
           (lambda (name)
             (let ((path
                    (expand-file-name
                     name
                     directory)))
               (list
                name
                (file-regular-p path)
                (if
                    (string-suffix-p
                     ".elc"
                     name)
                    t
                  (with-temp-buffer
                    (insert-file-contents-literally
                     path)
                    (list
                     (buffer-size)
                     (secure-hash
                      'sha256
                      (current-buffer))))))))
           names)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" "act-mode-autoloads.el" "act-mode-pkg.el" "act-mode.el" "act-mode.elc") (("README-elpa" t (55 "87be1ca6ec59e9443c55a81dfa2c5dbf3ce5fcbb6a5d26ffa8ead8b0da422e64")) ("act-mode-autoloads.el" t (944 "b4b7f2b4ffd113c02a41baef9b23f90b4af354f9995a6512a9f4affdc11540a8")) ("act-mode-pkg.el" t (421 "4ac6d571fb4ee009ba146d9b6bd7b94576f016dd66bbf86b4892a244a6eff7a8")) ("act-mode.el" t (1642 "1ca97867de633ca6f7133c19a1a0485de6d696cf3cb1d79e8c8e7030a489ba74")) ("act-mode.elc" t t)))"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_generated_autoload_registers_only_mode_then_introspection_loads_runtime() {
    let elisp_form = r##"(let ((before
                (list
                 (featurep
                  'act-mode)
                 (featurep
                  'act-mode-autoloads)
                 (autoloadp
                  (symbol-function
                   'act-mode))
                 (rassq
                  'act-mode
                  auto-mode-alist)
                 (boundp
                  'act-keywords)
                 (boundp
                  'act-fontlock))))
         (list
          before
          (help-function-arglist
           'act-mode
           t)
          (commandp
           'act-mode)
          (interactive-form
           'act-mode)
          (let ((file
                 (symbol-file
                  'act-mode
                  'defun)))
            (and file
                 (file-name-nondirectory
                  file)))
          (list
           (featurep
            'act-mode)
           (autoloadp
            (symbol-function
             'act-mode))
           (rassq
            'act-mode
            auto-mode-alist)
           (boundp
            'act-keywords)
           (boundp
            'act-fontlock))))"##;
    let expect = expect![[
        r#"OK ((nil t t nil nil nil) "[Arg list not available until function definition is loaded.]" t (interactive nil) "act-mode.el" (t nil ("\\.act\\'" . act-mode) t t))"#
    ]];
    assert_act_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn act_mode_direct_source_load_registers_feature_variables_mode_and_extension_once() {
    let elisp_form = r##"(list
         (featurep
          'act-mode)
         (boundp
          'act-keywords)
         (fboundp
          'act-mode)
         (length
          (seq-filter
           (lambda (entry)
             (equal entry
                    '("\\.act\\'" . act-mode)))
           auto-mode-alist))
         (progn
           (load
            (getenv
             "NEOMACS_PACKAGE_SOURCE")
            nil t t)
           (length
            (seq-filter
             (lambda (entry)
               (equal entry
                      '("\\.act\\'" . act-mode)))
             auto-mode-alist))))"##;
    let expect = expect!["OK (t t t 1 1)"];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_direct_source_reload_preserves_every_prebound_defvar_value() {
    let elisp_form = r##"(let ((original-keywords
                act-keywords)
               (original-types
                act-types)
               (original-functions
                act-functions)
               (original-fontlock
                act-fontlock))
         (unwind-protect
             (progn
               (setq act-keywords
                     '(sentinel-keyword)
                     act-types
                     '(sentinel-type)
                     act-functions
                     '(sentinel-function)
                     act-fontlock
                     '((sentinel-regexp
                        .
                        sentinel-face)))
               (load
                (getenv
                 "NEOMACS_PACKAGE_SOURCE")
                nil t t)
               (list
                act-keywords
                act-types
                act-functions
                act-fontlock
                (length
                 (seq-filter
                  (lambda (entry)
                    (equal entry
                           '("\\.act\\'" . act-mode)))
                  auto-mode-alist))))
           (setq act-keywords
                 original-keywords
                 act-types
                 original-types
                 act-functions
                 original-functions
                 act-fontlock
                 original-fontlock)))"##;
    let expect = expect![
        "OK ((sentinel-keyword) (sentinel-type) (sentinel-function) ((sentinel-regexp . sentinel-face)) 1)"
    ];
    assert_act_mode_parity(elisp_form, expect);
}
