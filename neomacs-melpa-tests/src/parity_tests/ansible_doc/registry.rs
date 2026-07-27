use expect_test::expect;

use super::{assert_ansible_doc_autoload_parity, assert_ansible_doc_parity};

#[test]
fn ansible_doc_registers_exact_feature_groups_faces_variables_and_functions() {
    let elisp_form = r##"(list
         (featurep 'ansible-doc)
         (get 'ansible 'custom-group)
         (get 'ansible-doc 'custom-group)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (facep symbol)
                  (get symbol 'face-defface-spec)
                  (get symbol 'custom-group)))
          '(ansible-doc-header ansible-doc-section ansible-doc-option
            ansible-doc-mandatory-option ansible-doc-label
            ansible-doc-default ansible-doc-choices
            ansible-doc-literal ansible-doc-module-xref))
         (mapcar
          (lambda (symbol)
            (list symbol (boundp symbol)
                  (and (boundp symbol) (symbol-value symbol))))
          '(ansible-doc--buffer-name ansible-doc--modules
            ansible-doc-current-module
            ansible-doc-module-font-lock-keywords
            ansible-doc-module-imenu-generic-expression))
         (mapcar
          (lambda (symbol)
            (list symbol (fboundp symbol) (commandp symbol)
                  (help-function-arglist symbol t)))
          '(ansible-doc-modules ansible-doc-read-module
            ansible-doc-follow-module-xref ansible-doc-current-module
            ansible-doc-fontify-module-xrefs ansible-doc-fontify-yaml
            ansible-doc-fontify-yaml-examples
            ansible-doc-revert-module-buffer
            ansible-doc-make-module-bookmark
            ansible-doc-jump-module-bookmark ansible-doc-module-mode
            ansible-doc-buffer ansible-doc ansible-doc-mode)))"##;
    let expect = expect![[
        r#"OK (t ((ansible-doc custom-group)) ((ansible-doc-header custom-face) (ansible-doc-section custom-face) (ansible-doc-option custom-face) (ansible-doc-mandatory-option custom-face) (ansible-doc-label custom-face) (ansible-doc-default custom-face) (ansible-doc-choices custom-face) (ansible-doc-literal custom-face) (ansible-doc-module-xref custom-face)) ((ansible-doc-header [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit bold)) nil) (ansible-doc-section [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-keyword-face)) nil) (ansible-doc-option [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-function-name-face)) nil) (ansible-doc-mandatory-option [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-type-face)) nil) (ansible-doc-label [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-doc-face)) nil) (ansible-doc-default [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-constant-face)) nil) (ansible-doc-choices [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-constant-face)) nil) (ansible-doc-literal [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-string-face)) nil) (ansible-doc-module-xref [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-type-face :underline t)) nil)) ((ansible-doc--buffer-name t "*ansible-doc %s*") (ansible-doc--modules t nil) (ansible-doc-current-module t nil) (ansible-doc-module-font-lock-keywords t (("\\`> .+$" 0 'ansible-doc-header) ("^Options (.+):$" 0 'ansible-doc-section) ("^\\(?:\\(?:Note\\|Requirement\\)s:\\)  " 0 'ansible-doc-section) ("^- [^[:space:]]+$" 0 'ansible-doc-option) ("^= [^[:space:]]+$" 0 'ansible-doc-mandatory-option) ("\\[\\(Default:\\)[[:space:]]+\\([^]]+\\)]" (1 'ansible-doc-label) (2 'ansible-doc-default)) ("(\\(Choices:\\)[[:space:]]+\\([^)]+\\))" (1 'ansible-doc-label) (2 'ansible-doc-choices)) ("`\\([^']+\\)'" 1 'ansible-doc-literal))) (ansible-doc-module-imenu-generic-expression t (("Options" "^[=-] \\([^[:space:]]+\\)$" 1)))) ((ansible-doc-modules t nil nil) (ansible-doc-read-module t nil (prompt)) (ansible-doc-follow-module-xref t nil (button)) (ansible-doc-current-module t nil nil) (ansible-doc-fontify-module-xrefs t nil (beg end)) (ansible-doc-fontify-yaml t nil (text)) (ansible-doc-fontify-yaml-examples t nil nil) (ansible-doc-revert-module-buffer t nil (_ignore-auto noconfirm)) (ansible-doc-make-module-bookmark t nil nil) (ansible-doc-jump-module-bookmark t nil (bookmark)) (ansible-doc-module-mode t t nil) (ansible-doc-buffer t nil (module)) (ansible-doc t t (module)) (ansible-doc-mode t t (&optional arg))))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_keymaps_button_type_and_mode_metadata_are_exact() {
    let elisp_form = r##"(list
         (keymapp ansible-doc-mode-map)
         (lookup-key ansible-doc-mode-map (kbd "C-c ?"))
         (keymapp ansible-doc-module-mode-map)
         (keymap-parent ansible-doc-module-mode-map)
         (get 'ansible-doc-module-xref 'button-category-symbol)
         (mapcar
          (lambda (property)
            (cons property
                  (button-type-get 'ansible-doc-module-xref property)))
          '(face action help-echo))
         (get 'ansible-doc-mode 'custom-group)
         (get 'ansible-doc-mode 'variable-documentation)
         (assq 'ansible-doc-mode minor-mode-alist)
         (assq 'ansible-doc-mode minor-mode-map-alist))"##;
    let expect = expect![[
        r#"OK (t ansible-doc t (keymap (keymap (backtab . backward-button) (27 keymap (9 . backward-button)) (9 . forward-button)) keymap (103 . revert-buffer) (60 . beginning-of-buffer) (62 . end-of-buffer) (104 . describe-mode) (63 . describe-mode) (127 . scroll-down-command) (33554464 . scroll-down-command) (32 . scroll-up-command) (113 . quit-window) (57 . digit-argument) (56 . digit-argument) (55 . digit-argument) (54 . digit-argument) (53 . digit-argument) (52 . digit-argument) (51 . digit-argument) (50 . digit-argument) (49 . digit-argument) (48 . digit-argument) (45 . negative-argument) (remap keymap (self-insert-command . undefined))) ansible-doc-module-xref-button ((face . ansible-doc-module-xref) (action . ansible-doc-follow-module-xref) (help-echo . "mouse-2, RET: visit module")) nil "Non-nil if Ansible-Doc mode is enabled.\nUse the command `ansible-doc-mode' to change this variable." (ansible-doc-mode " ADoc") (ansible-doc-mode keymap (3 keymap (63 . ansible-doc))))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_font_lock_and_imenu_registries_match_exactly() {
    let elisp_form = r##"(list
         ansible-doc-module-font-lock-keywords
         ansible-doc-module-imenu-generic-expression
         (format ansible-doc--buffer-name "copy")
         (get 'ansible-doc-current-module 'variable-documentation)
         (local-variable-if-set-p 'ansible-doc-current-module))"##;
    let expect = expect![[
        r#"OK ((("\\`> .+$" 0 'ansible-doc-header) ("^Options (.+):$" 0 'ansible-doc-section) ("^\\(?:\\(?:Note\\|Requirement\\)s:\\)  " 0 'ansible-doc-section) ("^- [^[:space:]]+$" 0 'ansible-doc-option) ("^= [^[:space:]]+$" 0 'ansible-doc-mandatory-option) ("\\[\\(Default:\\)[[:space:]]+\\([^]]+\\)]" (1 'ansible-doc-label) (2 'ansible-doc-default)) ("(\\(Choices:\\)[[:space:]]+\\([^)]+\\))" (1 'ansible-doc-label) (2 'ansible-doc-choices)) ("`\\([^']+\\)'" 1 'ansible-doc-literal)) (("Options" "^[=-] \\([^[:space:]]+\\)$" 1)) "*ansible-doc copy*" "The module documented by this buffer." t)"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_descriptor_records_exact_pin_requirement_and_payload() {
    let elisp_form = r##"(let* ((description (cadr (assq 'ansible-doc package-alist)))
               (directory (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar
            (lambda (file)
              (let ((relative (file-relative-name file directory)))
                (list relative
                      (file-attribute-size (file-attributes file))
                      (secure-hash 'sha256 file))))
            (directory-files-recursively directory "." nil))
           (lambda (a b) (string< (car a) (car b))))))"##;
    let expect = expect![[
        r#"OK (ansible-doc "20160924.824" nil "Ansible documentation Minor Mode." ((emacs (24 3))) (("README-elpa" 272 "3fe5af1e5c8592b29849fb8ac1cd22cdf5aa3b66782e1f2058771e14d87ab8f3") ("ansible-doc-autoloads.el" 1338 "75fc04a15426766a8fe5bbe8ab13ac9796c2103e502cce002dc164bd60f12631") ("ansible-doc-pkg.el" 440 "79e27bcf804ce0a20fe71cdafacdb3541dfcb6f5c5b7708f421ba4c1a87081e9") ("ansible-doc.el" 13601 "411df3c42a5180395a1ba383d6d33c7f57d1edea878cef3e7693fba2b7b06ea5") ("ansible-doc.elc" 13239 "d8c78b7745be65ff3a55e14ba1384a6c95c55002fb1b015b77195d7e997b7cfe")))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_autoloads_expose_commands_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'ansible-doc)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (commandp symbol)
                  (autoloadp (symbol-function symbol))
                  (symbol-function symbol)))
          '(ansible-doc ansible-doc-mode))
         (fboundp 'ansible-doc-buffer)
         (boundp 'ansible-doc-mode-map)
         (boundp 'ansible-doc--modules))"##;
    let expect = expect![[
        r#"OK (nil ((ansible-doc t t t (autoload "ansible-doc" "Show ansible documentation for MODULE.\n\n(fn MODULE)" t nil)) (ansible-doc-mode t t t (autoload "ansible-doc" "Minor mode for Ansible documentation.\n\nWhen called interactively, toggle `ansible-doc-mode'.  With\nprefix ARG, enable `ansible-doc-mode' if ARG is positive,\notherwise disable it.\n\nWhen called from Lisp, enable `ansible-doc-mode' if ARG is\nomitted, nil or positive.  If ARG is `toggle', toggle\n`ansible-doc-mode'.  Otherwise behave as if called interactively.\n\nIn `ansible-doc-mode' provide the following keybindings for\nAnsible documentation lookup:\n\n\\{ansible-doc-mode-map}\n\n(fn &optional ARG)" t nil))) nil nil nil)"#
    ]];
    assert_ansible_doc_autoload_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_reloads_idempotently_and_preserves_cached_modules() {
    let elisp_form = r##"(let ((ansible-doc--modules '("cached" "copy"))
               (source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (list ansible-doc--modules
               (length
                (cl-remove-if-not
                 (lambda (feature) (eq feature 'ansible-doc))
                 features))
               (lookup-key ansible-doc-mode-map (kbd "C-c ?"))
               (button-type-get 'ansible-doc-module-xref 'action)))"##;
    let expect =
        expect![[r#"OK (("cached" "copy") 1 ansible-doc ansible-doc-follow-module-xref)"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}
