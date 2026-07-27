use expect_test::expect;

use super::{assert_agda_lib_mode_autoload_parity, assert_agda_lib_mode_parity};

#[test]
fn agda_lib_mode_exact_pin_metadata_feature_and_installed_payload_match() {
    let elisp_form = r##"(progn
         (require 'lisp-mnt)
         (let* ((descriptor
                 (cadr
                  (assq 'agda-lib-mode
                        package-alist)))
                (package-directory
                 (file-name-directory
                  (getenv "NEOMACS_PACKAGE_SOURCE")))
                (files
                 (mapcar
                  (lambda (file)
                    (file-relative-name
                     file
                     package-directory))
                  (directory-files-recursively
                   package-directory
                   "."))))
           (list
            (package-desc-name descriptor)
            (package-version-join
             (package-desc-version descriptor))
            (package-desc-summary descriptor)
            (package-desc-kind descriptor)
            (package-desc-reqs descriptor)
            (copy-tree
             (package-desc-extras descriptor))
            (featurep 'agda-lib-mode)
            (with-temp-buffer
              (insert-file-contents
               (getenv "NEOMACS_PACKAGE_SOURCE"))
              (list
               (lm-header "version")
               (lm-header "keywords")
               (lm-header "url")
               (lm-header "package-requires")))
            files)))"##;
    let expect = expect![[
        r#"OK (agda-lib-mode "20251013.2307" "Major mode for Agda library files." nil ((emacs (24 3))) ((:maintainers ("Nicholas Coltharp" . "mail@heraplem.xyz")) (:authors ("Nicholas Coltharp" . "mail@heraplem.xyz")) (:keywords "text") (:revdesc . "1cf7d4867538") (:commit . "1cf7d486753887736eef6cae2688b2a05f9c1854") (:url . "https://codeberg.org/heraplem/agda-lib-mode")) t (nil "text" "https://codeberg.org/heraplem/agda-lib-mode" "((emacs \"24.3\"))") ("README-elpa" "agda-lib-mode-autoloads.el" "agda-lib-mode-pkg.el" "agda-lib-mode.el" "agda-lib-mode.elc"))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_complete_declared_surface_and_mode_contract_match() {
    let elisp_form = r##"(list
         (list
          'agda-lib-mode
          (fboundp 'agda-lib-mode)
          (commandp 'agda-lib-mode)
          (help-function-arglist
           'agda-lib-mode
           t)
          (interactive-form
           'agda-lib-mode)
          (get 'agda-lib-mode
               'derived-mode-parent)
          (car
           (split-string
            (documentation
             'agda-lib-mode)
            "\n")))
         (list
          'agda-lib-font-lock-keywords
          (boundp
           'agda-lib-font-lock-keywords)
          (default-boundp
           'agda-lib-font-lock-keywords)
          (local-variable-if-set-p
           'agda-lib-font-lock-keywords)
          (documentation-property
           'agda-lib-font-lock-keywords
           'variable-documentation
           t)
          (copy-tree
           agda-lib-font-lock-keywords))
         (mapcar
          #'featurep
          '(agda-lib-mode text-mode))
         (file-name-nondirectory
          (symbol-file
           'agda-lib-mode
           'defun))
         (file-name-nondirectory
          (symbol-file
           'agda-lib-font-lock-keywords
           'defvar)))"##;
    let expect = expect![[
        r#"OK ((agda-lib-mode t t nil (interactive nil) text-mode "Major mode derived from ‘text-mode’ by ‘define-derived-mode’.") (agda-lib-font-lock-keywords t t nil nil (("\\(^\\| \\)-- .*" . font-lock-comment-face) ("^\\([^ ]+:\\)" (1 font-lock-keyword-face)))) (t t) "agda-lib-mode.el" "agda-lib-mode.el")"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_font_lock_regexps_cover_exact_boundary_and_capture_semantics() {
    let elisp_form = r##"(condition-case problem
         (let ((comment-regexp
                (car
                 (nth 0
                      agda-lib-font-lock-keywords)))
               (field-regexp
                (car
                 (nth 1
                      agda-lib-font-lock-keywords))))
           (list
            (mapcar
             (lambda (text)
               (let ((position
                      (string-match
                       comment-regexp
                       text)))
                 (list
                  text
                  position
                  (and position
                       (match-string
                        0 text)))))
             '("-- comment"
               "--  comment"
               "--compact"
               "x -- comment"
               "x-- comment"
               "  -- comment"
               "\t-- comment"
               "x\n-- comment"))
            (mapcar
             (lambda (text)
               (let ((position
                      (string-match
                       field-regexp
                       text)))
                 (list
                  text
                  position
                  (and position
                       (match-string
                        0 text))
                  (and position
                       (match-string
                        1 text)))))
             '("name: standard-library"
               "include:src"
               "flags:"
               "Upper-Field: value"
               "with space: value"
               " leading: value"
               ": value"
               "x\nname: value"))))
       (error
        (list
         'captured-error
         problem)))"##;
    let expect = expect![[
        r#"OK ((("-- comment" 0 "-- comment") ("--  comment" 0 "--  comment") ("--compact" nil nil) ("x -- comment" 1 " -- comment") ("x-- comment" nil nil) ("  -- comment" 1 " -- comment") ("\11-- comment" nil nil) ("x\n-- comment" 2 "-- comment")) (("name: standard-library" 0 "name:" "name:") ("include:src" 0 "include:" "include:") ("flags:" 0 "flags:" "flags:") ("Upper-Field: value" 0 "Upper-Field:" "Upper-Field:") ("with space: value" nil nil nil) (" leading: value" nil nil nil) (": value" nil nil nil) ("x\nname: value" 0 "x\nname:" "x\nname:")))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_autoload_registry_is_lazy_and_idempotently_claims_agda_library_files() {
    let elisp_form = r##"(let* ((definition
                 (symbol-function
                  'agda-lib-mode))
                (entries
                 (seq-filter
                  (lambda (entry)
                    (equal
                     (cdr entry)
                     'agda-lib-mode))
                  auto-mode-alist)))
         (list
          (featurep 'agda-lib-mode)
          (autoloadp definition)
          (nth 1 definition)
          (nth 3 definition)
          (nth 4 definition)
          entries
          (length entries)
          (boundp
           'agda-lib-font-lock-keywords)
          (get 'agda-lib-mode
               'derived-mode-parent)))"##;
    let expect = expect![[
        r#"OK (nil t "agda-lib-mode" t nil (("\\.agda-lib\\'" . agda-lib-mode)) 1 nil nil)"#
    ]];

    assert_agda_lib_mode_autoload_parity(elisp_form, expect);
}
