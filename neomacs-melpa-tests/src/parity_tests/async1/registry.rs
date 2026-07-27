use expect_test::expect;

use super::{assert_async1_autoload_parity, assert_async1_parity};

#[test]
fn async1_exact_package_descriptor_origin_dependency_and_feature_contract_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq 'async1 package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-summary descriptor)
          (package-desc-kind descriptor)
          (package-desc-reqs descriptor)
          (package-desc-extras descriptor)
          (featurep 'async1)
          (package-installed-p
           'async1
           '(20260421 2116))
          (file-name-nondirectory
           (locate-library "async1"))))"##;
    let expect = expect![[
        r#"OK (async1 "20260421.2116" "Unroll chain of async callbacks, parallel and sequencial." nil ((emacs (24 1)) (compat (30 1))) ((:maintainers (nil . "github.com/Anoncheg1,codeberg.org/Anoncheg")) (:authors (nil . "github.com/Anoncheg1,codeberg.org/Anoncheg")) (:keywords "tools" "async" "callback" "lisp" "extensions") (:revdesc . "88cccffe14bd") (:commit . "88cccffe14bdd0a61dbb2e33edf8c335706f24dc") (:url . "https://github.com/Anoncheg1/emacs-async1")) t t "async1.el")"#
    ]];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_installed_payload_inventory_hashes_archive_files_not_generated_artifacts() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq 'async1 package-alist)))
                 (directory
                  (package-desc-dir descriptor))
                 (archive-files
                  '("async1-pkg.el"
                    "async1.el")))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name file directory)))
              (if
                  (member file archive-files)
                  (list
                   file
                   :archive
                   (file-attribute-size
                    (file-attributes path))
                   (with-temp-buffer
                     (insert-file-contents-literally path)
                     (secure-hash
                      'sha256
                      (current-buffer))))
                (list
                 file
                 :generated
                 (file-readable-p path)))))
          (sort
           (seq-filter
            (lambda (file)
              (file-regular-p
               (expand-file-name file directory)))
            (directory-files directory nil "\\`[^.]"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (("async1-autoloads.el" :generated t) ("async1-pkg.el" :archive 527 "7548e340551a8ce89d08b81537cf42f0f50109114ead378698a945cb234312b0") ("async1.el" :archive 11529 "97ef51118ed5c11fa4df75e97a41f323a9f77bda89a1b76c0f23464ba1d213ef") ("async1.elc" :generated t))"#
    ]];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_compat_dependency_is_satisfied_by_builtin_or_installed_runtime_support() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq 'compat package-alist))))
         (list
          (and descriptor
               (package-desc-name descriptor))
          (and descriptor
               (package-version-join
                (package-desc-version descriptor)))
          (package-installed-p
           'compat
           '(30 1))
          (package-built-in-p
           'compat)
          (package-built-in-p
           'compat
           '(30 1))
          (featurep 'compat)
          (file-name-nondirectory
           (or
            (locate-library "compat")
            ""))
          (mapcar
           (lambda (requirement)
             (list
              (car requirement)
              (package-version-join
               (cadr requirement))))
           (package-desc-reqs
            (cadr
             (assq 'async1 package-alist))))))"##;
    let expect =
        expect![[r#"OK (nil nil t t t nil "compat.el" ((emacs "24.1") (compat "30.1")))"#]];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_complete_callable_command_arglist_and_source_surface_matches() {
    let elisp_form = r##"(let (symbols)
         (mapatoms
          (lambda (symbol)
            (when
                (and
                 (string-prefix-p
                  "async1"
                  (symbol-name symbol))
                 (not
                  (string-suffix-p
                   "--inliner"
                   (symbol-name symbol)))
                 (not
                  (string-suffix-p
                   "--cmacro"
                   (symbol-name symbol)))
                 (fboundp symbol)
                 (let ((file
                        (symbol-file symbol 'defun)))
                   (and file
                        (string=
                         (file-name-nondirectory file)
                         "async1.el"))))
              (push symbol symbols))))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (commandp symbol)
             (interactive-form symbol)
             (prin1-to-string
              (help-function-arglist symbol t))
             (file-name-nondirectory
              (symbol-file symbol 'defun))))
          (sort symbols
                (lambda (left right)
                  (string<
                   (symbol-name left)
                   (symbol-name right))))))"##;
    let expect = expect![[
        r#"OK ((async1--handle-parallel-step nil nil "(specs data chain-step current-index)" "async1.el") (async1--handle-sequential-step nil nil "(step data chain-step current-index)" "async1.el") (async1-create-function nil nil "(spec)" "async1.el") (async1-default-aggregator nil nil "(results)" "async1.el") (async1-default-template nil nil "(data callback delay result-suffix)" "async1.el") (async1-plist-get nil nil "(plist key &optional default)" "async1.el") (async1-plist-remove nil nil "(plist key)" "async1.el") (async1-start nil nil "(initial-data sequence &optional final-callback)" "async1.el"))"#
    ]];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_declares_no_package_variables_constants_custom_options_or_macros() {
    let elisp_form = r##"(let (variables macros)
         (mapatoms
          (lambda (symbol)
            (when
                (and
                 (string-prefix-p
                  "async1"
                  (symbol-name symbol))
                 (boundp symbol)
                 (let ((file
                        (symbol-file symbol 'defvar)))
                   (and file
                        (string=
                         (file-name-nondirectory file)
                         "async1.el"))))
              (push symbol variables))
            (when
                (and
                 (string-prefix-p
                  "async1"
                  (symbol-name symbol))
                 (macrop symbol)
                 (let ((file
                        (symbol-file symbol 'defun)))
                   (and file
                        (string=
                         (file-name-nondirectory file)
                         "async1.el"))))
              (push symbol macros))))
         (list
          (sort variables
                (lambda (left right)
                  (string<
                   (symbol-name left)
                   (symbol-name right))))
          (sort macros
                (lambda (left right)
                  (string<
                   (symbol-name left)
                   (symbol-name right))))
          (featurep 'async1)
          (get 'async1 'custom-group)))"##;
    let expect = expect!["OK (nil nil t nil)"];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_generated_autoload_surface_contains_only_the_five_documented_entries() {
    let elisp_form = r##"(list
         (featurep 'async1)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (and
              (fboundp symbol)
              (autoloadp
               (symbol-function symbol)))
             (symbol-file symbol 'defun)))
          '(async1-default-template
            async1-default-aggregator
            async1-plist-remove
            async1-plist-get
            async1-start
            async1-create-function
            async1--handle-parallel-step
            async1--handle-sequential-step)))"##;
    let expect = expect![[
        r#"OK (nil ((async1-default-template t t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache-frozen-melpa/async1/20260421.2116/home/.emacs.d/elpa/async1-20260421.2116/async1.el") (async1-default-aggregator t t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache-frozen-melpa/async1/20260421.2116/home/.emacs.d/elpa/async1-20260421.2116/async1.el") (async1-plist-remove t t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache-frozen-melpa/async1/20260421.2116/home/.emacs.d/elpa/async1-20260421.2116/async1.el") (async1-plist-get t t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache-frozen-melpa/async1/20260421.2116/home/.emacs.d/elpa/async1-20260421.2116/async1.el") (async1-start t t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache-frozen-melpa/async1/20260421.2116/home/.emacs.d/elpa/async1-20260421.2116/async1.el") (async1-create-function nil nil nil) (async1--handle-parallel-step nil nil nil) (async1--handle-sequential-step nil nil nil)))"#
    ]];

    assert_async1_autoload_parity(elisp_form, expect);
}
