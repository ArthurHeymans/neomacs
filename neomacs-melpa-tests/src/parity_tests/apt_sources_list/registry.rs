use expect_test::expect;

use super::{assert_apt_sources_list_autoload_parity, assert_apt_sources_list_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_authors_and_requirement() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'apt-sources-list package-alist)))
       (directory (package-desc-dir description)))
  (list
   (featurep 'apt-sources-list)
   (package-installed-p 'apt-sources-list)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (package-desc-extras description)
   (file-name-nondirectory
    (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t t apt-sources-list "20180527.1241" "Mode for editing APT source.list files." ((emacs (24 4))) ((:maintainers ("Joe Wreschnig" . "joe.wreschnig@gmail.com")) (:authors ("Dr. Rafael Sepúlveda" . "drs@gnulinux.org.mx")) (:revdesc . "44112833b3fa") (:commit . "44112833b3fa7f4d7e43708e5996782e22bb2fa3") (:url . "https://git.korewanetadesu.com/apt-sources-list.git")) "apt-sources-list-20180527.1241")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn installed_archive_contains_only_the_recipe_selected_runtime_and_descriptor() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'apt-sources-list package-alist)))
       (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((path (expand-file-name name directory)))
       (list name
             (file-attribute-size (file-attributes path)))))
   (sort
    (seq-remove
     (lambda (name)
       (or (member name '("." ".." "README-elpa"))
           (string-suffix-p ".elc" name)
           (string-suffix-p "-autoloads.el" name)))
     (directory-files directory))
    #'string-lessp)))"##;
    let expect = expect![[r#"OK (("apt-sources-list-pkg.el" 436) ("apt-sources-list.el" 15146))"#]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn installed_runtime_and_descriptor_match_the_exact_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'apt-sources-list package-alist)))
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
   '("apt-sources-list.el"
     "apt-sources-list-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("apt-sources-list.el" 15146 "d3b700c4fb3a239953ea11fe970f3e26764d2485bdb54b946894c507447222b6") ("apt-sources-list-pkg.el" 436 "96fb78c5eef749c533146c256ce4e513b6d4bf3b588005a74fcf8089d10f347d"))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_preserves_arguments_interactivity_and_origins() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (macrop symbol)
    (commandp symbol)
    (copy-tree (help-function-arglist symbol t))
    (interactive-form symbol)
    (file-name-nondirectory
     (symbol-file symbol 'defun))))
 '(apt-sources-list-insert
   apt-sources-list-forward-source
   apt-sources-list-backward-source
   apt-sources-list-source-p
   apt-sources-list-match-source
   apt-sources-list-change-type
   apt-sources-list-change-options
   apt-sources-list-change-uri
   apt-sources-list--read-components
   apt-sources-list-change-suite
   apt-sources-list-change-components
   apt-sources-list-replicate
   apt-sources-list-mode))"##;
    let expect = expect![[
        r#"OK ((apt-sources-list-insert t nil t (uri &rest --cl-rest--) (interactive (let* ((_ (barf-if-buffer-read-only)) (name (read-string "Source name: ")) (type (if current-prefix-arg (completing-read "Type: " '("deb" "deb-src") nil t "deb") "deb")) (options (if current-prefix-arg (read-string "Options: ") "")) (uri (read-string "URI: " "https://")) (suite (completing-read "Suite: " apt-sources-list-suites nil nil (car apt-sources-list-suites))) (components (if (string-suffix-p "/" suite) nil (apt-sources-list--read-components)))) (list uri :name (if (string-blank-p name) nil name) :type type :options (if (string-blank-p options) nil options) :suite suite :components components))) "apt-sources-list.el") (apt-sources-list-forward-source t nil t (&optional n) (interactive "p") "apt-sources-list.el") (apt-sources-list-backward-source t nil t (&optional n) (interactive "p") "apt-sources-list.el") (apt-sources-list-source-p t nil nil nil nil "apt-sources-list.el") (apt-sources-list-match-source t nil nil nil nil "apt-sources-list.el") (apt-sources-list-change-type t nil t (&optional type) (interactive "*") "apt-sources-list.el") (apt-sources-list-change-options t nil t (options) (interactive (list (let ((saved-match-data #1=(match-data))) (unwind-protect (progn (barf-if-buffer-read-only) (apt-sources-list-match-source) (read-string "Options: " (match-string 2))) (set-match-data saved-match-data t))))) "apt-sources-list.el") (apt-sources-list-change-uri t nil t (uri) (interactive (list (let ((saved-match-data #1#)) (unwind-protect (progn (barf-if-buffer-read-only) (apt-sources-list-match-source) (read-string "URI: " (match-string 3))) (set-match-data saved-match-data t))))) "apt-sources-list.el") (apt-sources-list--read-components t nil nil (&optional initial) nil "apt-sources-list.el") (apt-sources-list-change-suite t nil t (suite &optional default-components) (interactive (let ((saved-match-data #1#)) (unwind-protect (progn (barf-if-buffer-read-only) (apt-sources-list-match-source) (let ((components (match-string 5)) (suite (completing-read "Suite: " apt-sources-list-suites))) (if (not (string-suffix-p "/" suite)) (list suite (apt-sources-list--read-components)) (list suite)))) (set-match-data saved-match-data t)))) "apt-sources-list.el") (apt-sources-list-change-components t nil t (components) (interactive (let ((saved-match-data #1#)) (unwind-protect (progn (barf-if-buffer-read-only) (apt-sources-list-match-source) (if (string-suffix-p "/" (match-string 4)) (progn (signal 'apt-sources-list-suite-component-mismatch nil))) (list (apt-sources-list--read-components (substring-no-properties (match-string 5))))) (set-match-data saved-match-data t)))) "apt-sources-list.el") (apt-sources-list-replicate t nil t nil (interactive "*") "apt-sources-list.el") (apt-sources-list-mode t nil t nil (interactive nil) "apt-sources-list.el"))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn customization_errors_and_parser_constants_preserve_their_complete_contracts() {
    let elisp_form = r##"(list
 (get 'apt-sources-list 'custom-group)
 (get 'apt-sources-list 'group-documentation)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (default-value symbol)
     (eval (car (get symbol 'standard-value)))
     (get symbol 'custom-type)
     (get symbol 'custom-group)
     (get symbol 'variable-documentation)))
  '(apt-sources-list-suites
    apt-sources-list-components
    apt-sources-list-name-format))
 (mapcar
  (lambda (symbol)
    (list symbol
          (get symbol 'error-conditions)
          (get symbol 'error-message)))
  '(apt-sources-list-not-found
    apt-sources-list-suite-component-mismatch))
 (list
  (stringp apt-sources-list-one-line)
  (length apt-sources-list-one-line)
  (secure-hash 'sha256 apt-sources-list-one-line))
 (copy-tree apt-sources-list-font-lock-keywords))"##;
    let expect = expect![[
        r##"OK (((apt-sources-list-type custom-face) (apt-sources-list-uri custom-face) (apt-sources-list-suite custom-face) (apt-sources-list-options custom-face) (apt-sources-list-components custom-face) (apt-sources-list-suites custom-variable) (apt-sources-list-components custom-variable) (apt-sources-list-name-format custom-variable)) "Mode for editing APT sources.list file." ((apt-sources-list-suites #1=("stable" "testing" "unstable" "oldstable" "jessie" "stretch" "sid") #1# (repeat string) nil "Suites to offer for completion.\n\nThe first item in this list is used as the default value when\nediting sources.") (apt-sources-list-components #2=("main" "contrib" "non-free") #2# (repeat string) nil "Components to offer for completion.\n\nThe first item in this list is used as the default value when\nediting sources.") (apt-sources-list-name-format "# %s" "# %s" string nil "Format used in the name of a new source line.\n\nThis is used by ‘apt-sources-list-insert’.  It should contain a\nsingle “%s” which will be replaced with the source name.")) ((apt-sources-list-not-found (apt-sources-list-not-found error) "The point is not on an APT source line") (apt-sources-list-suite-component-mismatch (apt-sources-list-suite-component-mismatch error) "Exact suite paths (ending with “/”) may not specify components")) (t 226 "baab5a267896b91e213ac2e8b078faf5359110772d0b8c0e5e1337337a4ca400") (("^[[:blank:]]*\\(\\(?:deb\\(?:-src\\)?\\)\\)[[:blank:]]+\\(?:\\[\\([^]\n#]+\\)][[:blank:]]+\\)?\\([.0-9A-Z_a-z-]+:[^\11\n #]+\\)[[:blank:]]+\\([^\11\n #]*/\\|[^\11\n #]*[^\11\n #/][[:blank:]]+\\([^\11\n #]+\\(?:[[:blank:]]+[^\11\n #]+\\)*\\)\\)[[:blank:]]*\\(?:$\\|#\\)" (1 'apt-sources-list-type) (2 'apt-sources-list-options nil t) (3 'apt-sources-list-uri) (4 'apt-sources-list-suite) (5 'apt-sources-list-components t t))))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn all_five_semantic_faces_preserve_specs_documentation_and_source_ownership() {
    let elisp_form = r##"(mapcar
 (lambda (face)
   (list
    face
    (facep face)
    (get face 'face-defface-spec)
    (get face 'face-documentation)
    (file-name-nondirectory
     (symbol-file face 'defface))))
 '(apt-sources-list-type
   apt-sources-list-uri
   apt-sources-list-suite
   apt-sources-list-options
   apt-sources-list-components))"##;
    let expect = expect![[
        r#"OK ((apt-sources-list-type [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit font-lock-constant-face))) "Face for a source’s type (i.e. “deb” or “deb-src”)." "apt-sources-list.el") (apt-sources-list-uri [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit font-lock-variable-name-face))) "Face for a source’s URI." "apt-sources-list.el") (apt-sources-list-suite [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit font-lock-type-face))) "Face for a source’s suite (e.g. “unstable”, “stretch/updates”)." "apt-sources-list.el") (apt-sources-list-options [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit font-lock-builtin-face))) "Face for a package source’s options (e.g. “[arch=amd64]”)." "apt-sources-list.el") (apt-sources-list-components [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit font-lock-keyword-face))) "Face for a package source’s components (e.g. “main”, “non-free”)." "apt-sources-list.el"))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_only_the_mode_and_registers_exact_list_filenames() {
    let elisp_form = r##"(list
 (featurep 'apt-sources-list)
 (featurep 'apt-sources-list-autoloads)
 (mapcar
  (lambda (symbol)
    (list symbol
          (fboundp symbol)
          (and (fboundp symbol)
               (autoloadp (symbol-function symbol)))
          (commandp symbol)))
  '(apt-sources-list-mode
    apt-sources-list-insert
    apt-sources-list-change-suite))
 (seq-filter
  (lambda (entry)
    (eq (cdr entry) 'apt-sources-list-mode))
  auto-mode-alist)
 (mapcar
  (lambda (filename)
    (cons
     filename
     (seq-some
      (lambda (entry)
        (and (eq (cdr entry)
                 'apt-sources-list-mode)
             (string-match-p
              (car entry) filename)))
      auto-mode-alist)))
  '("/etc/apt/sources.list"
    "./sources.list"
    "/etc/apt/sources.list.d/debian.list"
    "/srv/sources.list.d/vendor.list"
    "/etc/apt/sources.list.d/not-a-list.conf"
    "/workspace/sources.list.backup")))"##;
    let expect = expect![[
        r#"OK (nil t ((apt-sources-list-mode t t t) (apt-sources-list-insert nil nil nil) (apt-sources-list-change-suite nil nil nil)) (("\\(?:[./]sources\\.list\\|/sources\\.list\\.d/[^z-a]+\\.list\\)\\'" . apt-sources-list-mode)) (("/etc/apt/sources.list" . 8) ("./sources.list" . 1) ("/etc/apt/sources.list.d/debian.list" . 8) ("/srv/sources.list.d/vendor.list" . 4) ("/etc/apt/sources.list.d/not-a-list.conf") ("/workspace/sources.list.backup")))"#
    ]];
    assert_apt_sources_list_autoload_parity(elisp_form, expect);
}
