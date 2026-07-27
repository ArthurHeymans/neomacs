use expect_test::expect;

use super::{assert_apib_mode_autoload_parity, assert_apib_mode_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_dependency_and_revision() {
    let elisp_form = r##"(let* ((description (cadr (assq 'apib-mode package-alist)))
       (markdown-description (cadr (assq 'markdown-mode package-alist)))
       (directory (package-desc-dir description)))
  (list
   (featurep 'apib-mode)
   (featurep 'markdown-mode)
   (package-installed-p 'apib-mode)
   (package-installed-p 'markdown-mode)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (package-desc-extras description)
   (package-version-join (package-desc-version markdown-description))
   (file-name-nondirectory (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t t t t apib-mode "20200101.1017" "Major mode for API Blueprint files." ((markdown-mode (2 1))) ((:maintainers ("Vilibald Wanča" . "vilibald@wvi.cz")) (:authors ("Vilibald Wanča" . "vilibald@wvi.cz")) (:keywords "tools" "api-blueprint") (:revdesc . "c6dd05201f6e") (:commit . "c6dd05201f6eb9295736d8668a79a7510d11159e") (:url . "https://github.com/w-vi/apib-mode")) "20260722.40" "apib-mode-20200101.1017")"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn installed_archive_contains_only_the_recipe_selected_runtime_and_descriptor() {
    let elisp_form = r##"(let* ((description (cadr (assq 'apib-mode package-alist)))
       (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((path (expand-file-name name directory)))
       (list name (file-attribute-size (file-attributes path)))))
   (sort
    (seq-remove
     (lambda (name)
       (or (member name '("." ".." "README-elpa"))
           (string-suffix-p ".elc" name)
           (string-suffix-p "-autoloads.el" name)))
     (directory-files directory))
    #'string-lessp)))"##;
    let expect = expect![[r#"OK (("apib-mode-pkg.el" 437) ("apib-mode.el" 9212))"#]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn installed_runtime_and_descriptor_match_the_exact_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description (cadr (assq 'apib-mode package-alist)))
       (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((file (expand-file-name name directory)))
       (list
        name
        (file-attribute-size (file-attributes file))
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert-file-contents-literally file)
          (secure-hash 'sha256 (current-buffer))))))
   '("apib-mode.el" "apib-mode-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("apib-mode.el" 9212 "4d14e7662a852d349be9647d399a2fc9de5ff20906ec8a665bdc98556709f302") ("apib-mode-pkg.el" 437 "22ad7d9f38cf0eaf6018a1d68e161bb46b2124e43463013cf382f70a8c877d4b"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn complete_callable_and_macro_surface_preserves_contracts_interactivity_and_origins() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (macrop symbol)
    (commandp symbol)
    (copy-tree (help-function-arglist symbol t))
    (interactive-form symbol)
    (file-name-nondirectory (symbol-file symbol 'defun))))
 '(apib-with-drafter
   apib-refract-element-p
   apib-refract-mapc
   apib-get-assets
   apib-print-assets
   apib-parse-to-plist
   apib-compile-with-drafter
   apib-validate
   apib-valid-p
   apib-get-json-schema
   apib-get-json
   apib-parse
   apib-error-filename
   apib-mode))"##;
    let expect = expect![[
        r#"OK ((apib-with-drafter t t nil (&rest exp) nil "apib-mode.el") (apib-refract-element-p t nil nil (element type) nil "apib-mode.el") (apib-refract-mapc t nil nil (func element) nil "apib-mode.el") (apib-get-assets t nil nil (content-type) nil "apib-mode.el") (apib-print-assets t nil nil (content-type) nil "apib-mode.el") (apib-parse-to-plist t nil nil (filename) nil "apib-mode.el") (apib-compile-with-drafter t nil nil (filename &rest args) nil "apib-mode.el") (apib-validate t nil t nil (interactive nil) "apib-mode.el") (apib-valid-p t nil t nil (interactive nil) "apib-mode.el") (apib-get-json-schema t nil t nil (interactive nil) "apib-mode.el") (apib-get-json t nil t nil (interactive nil) "apib-mode.el") (apib-parse t nil t nil (interactive nil) "apib-mode.el") (apib-error-filename t nil nil nil nil "apib-mode.el") (apib-mode t nil t nil (interactive nil) "apib-mode.el"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn customization_group_variables_keymap_and_compilation_registry_are_complete() {
    let elisp_form = r##"(list
 (get 'api-blueprint 'custom-group)
 (get 'api-blueprint 'group-documentation)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (default-value symbol)
     (eval (car (get symbol 'standard-value)))
     (get symbol 'custom-type)
     (get symbol 'custom-group)
     (get symbol 'variable-documentation)))
  '(apib-drafter-executable apib-asset-buffer apib-result-buffer))
 (mapcar
  (lambda (key)
    (list key (lookup-key apib-mode-map (kbd key))))
  '("C-c C-x p" "C-c C-x v" "C-c C-x j" "C-c C-x s"))
 (assq 'apib compilation-error-regexp-alist-alist)
 (memq 'apib compilation-error-regexp-alist)
 (get 'apib-mode 'derived-mode-parent)
 (get 'apib-mode 'mode-class))"##;
    let expect = expect![[
        r#"OK (((apib-drafter-executable custom-variable) (apib-asset-buffer custom-variable) (apib-result-buffer custom-variable)) "Major mode for editing API Blueprint files." ((apib-drafter-executable "drafter" "drafter" file nil "Location of the drafter API Blueprint parser executable.") (apib-asset-buffer "*apib-assets*" "*apib-assets*" string nil "Name of the buffer to output json and json schema assets.") (apib-result-buffer "*apib-parse-result*" "*apib-parse-result*" string nil "Name of the buffer to output drafter parse result.")) (("C-c C-x p" apib-parse) ("C-c C-x v" apib-validate) ("C-c C-x j" apib-get-json) ("C-c C-x s" apib-get-json-schema)) (apib "^\\(?:warning\\|error\\):.+?line \\([0-9]+\\), column \\([0-9]+\\) - line \\([0-9]+\\), column \\([0-9]+\\).*$" apib-error-filename 3 4) (apib absoft ada aix ant bash borland python-tracebacks-and-caml cmake cmake-info comma msft edg-1 edg-2 epc ftnchek gradle-kotlin gradle-kotlin-legacy gradle-android iar ibm irix java javac jikes-file maven jikes-line clang-include gcc-include ruby-Test::Unit rust-panic rust lua lua-stack gmake gnu cucumber lcc makepp mips-1 mips-2 oracle perl php rxp shellcheck sparc-pascal-file sparc-pascal-line sparc-pascal-example sun sun-ada watcom 4bsd gcov-file gcov-header gcov-nomark gcov-called-line gcov-never-called perl--Pod::Checker perl--Test perl--Test2 perl--Test::Harness weblint guile-file guile-line typescript-tsc-plain typescript-tsc-pretty) markdown-mode nil)"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_only_the_mode_and_does_not_claim_file_extensions() {
    let elisp_form = r##"(list
 (featurep 'apib-mode)
 (featurep 'apib-mode-autoloads)
 (fboundp 'apib-mode)
 (autoloadp (symbol-function 'apib-mode))
 (commandp 'apib-mode)
 (mapcar
  (lambda (symbol)
    (list symbol (fboundp symbol)))
  '(apib-parse apib-validate apib-get-json apib-get-json-schema))
 (seq-filter
  (lambda (entry)
    (and (stringp (car entry))
         (string-match-p "apib" (car entry))))
  auto-mode-alist)
 (get 'apib-mode 'custom-autoload))"##;
    let expect = expect![
        "OK (nil t t t t ((apib-parse nil) (apib-validate nil) (apib-get-json nil) (apib-get-json-schema nil)) nil nil)"
    ];
    assert_apib_mode_autoload_parity(elisp_form, expect);
}
