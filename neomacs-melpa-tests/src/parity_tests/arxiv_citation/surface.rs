use expect_test::expect;

use super::{assert_arxiv_citation_autoload_parity, assert_arxiv_citation_parity};

#[test]
fn installed_descriptor_runtime_bytes_and_file_set_identify_the_exact_melpa_build() {
    let elisp_form = r##"(let* ((descriptor
         (cadr (assq 'arxiv-citation package-alist)))
       (directory (package-desc-dir descriptor)))
  (list
   (featurep 'arxiv-citation)
   (package-installed-p 'arxiv-citation)
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
    '("arxiv-citation.el"
      "arxiv-citation-pkg.el"))))"##;
    let expect = expect![[
        r#"OK (t t arxiv-citation "20230713.627" ((emacs (25 1)) (dash (2 19 1)) (s (1 12 0))) "Utility functions for dealing with arXiv papers." ((:maintainers ("Tony Zorman" . "soliditsallgood@mailbox.org")) (:authors ("Tony Zorman" . "soliditsallgood@mailbox.org")) (:keywords "convenience") (:revdesc . "04de0dae1121") (:commit . "04de0dae1121fb92c30b393449c6f8d6d940dbed") (:url . "https://gitlab.com/slotThe/arXiv-citation")) ("arxiv-citation-autoloads.el" "arxiv-citation-pkg.el" "arxiv-citation.el") (("arxiv-citation.el" 13741 "99aebc6af957a0b5d22a69e9b1f601ced716b0ea714aabce0a85d6970870b25f") ("arxiv-citation-pkg.el" 503 "b100d74cf10f53300a67b4cc5eaf1d1825214201b77af24ccf5c0cf1f825e535")))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_exactly_the_four_documented_commands_without_loading_runtime() {
    let elisp_form = r##"(list
 (featurep 'arxiv-citation)
 (featurep 'arxiv-citation-autoloads)
 (featurep 'dash)
 (featurep 's)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (fboundp symbol)
     (and
      (fboundp symbol)
      (autoloadp (symbol-function symbol)))
     (commandp symbol)
     (copy-tree
      (help-function-arglist symbol t))))
  '(arxiv-citation
    arxiv-citation-gui
    arxiv-citation-download-and-open
    arxiv-citation-elfeed
    arxiv-citation-get-details
    arxiv-citation-pdf-name))
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (boundp symbol)
     (and (boundp symbol)
          (symbol-value symbol))
     (custom-variable-p symbol)))
  '(arxiv-citation-bibtex-files
    arxiv-citation-library
    arxiv-citation-open-pdf-function
    arxiv-citation-max-authors
    arxiv-citation-overwrite-file)))"##;
    let expect = expect![[
        r#"OK (nil t nil nil ((arxiv-citation t t t "[Arg list not available until function definition is loaded.]") (arxiv-citation-gui t t t "[Arg list not available until function definition is loaded.]") (arxiv-citation-download-and-open t t nil "[Arg list not available until function definition is loaded.]") (arxiv-citation-elfeed t t t "[Arg list not available until function definition is loaded.]") (arxiv-citation-get-details nil nil nil t) (arxiv-citation-pdf-name nil nil nil t)) ((arxiv-citation-bibtex-files nil nil nil) (arxiv-citation-library nil nil nil) (arxiv-citation-open-pdf-function nil nil nil) (arxiv-citation-max-authors nil nil nil) (arxiv-citation-overwrite-file nil nil nil)))"#
    ]];
    assert_arxiv_citation_autoload_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_has_exact_arguments_interactivity_macro_status_and_origin() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (macrop symbol)
    (commandp symbol)
    (copy-tree
     (help-function-arglist symbol t))
    (interactive-form symbol)
    (file-name-nondirectory
     (symbol-file symbol 'defun))))
 '(arxiv-citation-arXiv-id
   arxiv-citation-pdf-link
   arxiv-citation-parse
   arxiv-citation-pdf-name
   arxiv-citation-generate-autokey
   arxiv-citation-get-details
   arxiv-citation-get-citation
   arxiv-citation-get-zbmath-citation
   arxiv-citation-get-arxiv-citation
   arxiv-citation
   arxiv-citation-gui
   arxiv-citation-download-and-open
   arxiv-citation-elfeed))"##;
    let expect = expect![[
        r#"OK ((arxiv-citation-arXiv-id t nil nil (url) nil "arxiv-citation.el") (arxiv-citation-pdf-link t nil nil (url) nil "arxiv-citation.el") (arxiv-citation-parse t nil nil (method) nil "arxiv-citation.el") (arxiv-citation-pdf-name t nil nil (info) nil "arxiv-citation.el") (arxiv-citation-generate-autokey t nil nil nil nil "arxiv-citation.el") (arxiv-citation-get-details t nil nil (link) nil "arxiv-citation.el") (arxiv-citation-get-citation t nil nil (url) nil "arxiv-citation.el") (arxiv-citation-get-zbmath-citation t nil nil (url) nil "arxiv-citation.el") (arxiv-citation-get-arxiv-citation t nil nil (url) nil "arxiv-citation.el") (arxiv-citation t nil t (url) (interactive nil) "arxiv-citation.el") (arxiv-citation-gui t nil t nil (interactive nil) "arxiv-citation.el") (arxiv-citation-download-and-open t nil nil (url) nil "arxiv-citation.el") (arxiv-citation-elfeed t nil t nil (interactive nil) "arxiv-citation.el"))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn every_custom_variable_preserves_default_runtime_value_type_group_and_documentation() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (default-value symbol)
    (eval (car (get symbol 'standard-value)))
    (get symbol 'custom-type)
    (get symbol 'custom-group)
    (documentation-property
     symbol 'variable-documentation t)
    (let ((file
           (symbol-file symbol 'defvar)))
      (and
       file
       (file-name-nondirectory file)))))
 '(arxiv-citation-bibtex-files
   arxiv-citation-library
   arxiv-citation-open-pdf-function
   arxiv-citation-max-authors
   arxiv-citation-overwrite-file))"##;
    let expect = expect![[
        r#"OK ((arxiv-citation-bibtex-files nil nil (repeat string) nil "List of files to insert bibtex information into." "arxiv-citation.el") (arxiv-citation-library "~/.emacs.d/" "~/.emacs.d/" string nil "Path to the library.\nI.e., the place where all files should be downloaded to." "arxiv-citation.el") (arxiv-citation-open-pdf-function browse-url-xdg-open browse-url-xdg-open function nil "Function with which to open PDF files." "arxiv-citation.el") (arxiv-citation-max-authors nil nil (choice (natnum :tag "Only show this many authors") (const :tag "Show all authors" nil)) nil "Maximum number of authors to show in the PDF title.\nIf this is nil, show all authors instead." "arxiv-citation.el") (arxiv-citation-overwrite-file nil nil boolean nil "Whether to overwrite an existing file in non-interactive mode.\nWhen downloading a file, and one of the same name already exists,\nthen do the following:\n\n  - If the variable is nil (the default), ask for confirmation\n    whether to overwrite the file in interactive usage, and do\n    NOT overwrite the file on non-interactive (batch) mode.\n\n  - If the variable is t, always overwrite the file and do not\n    ask for confirmation, even in interactive usage." "arxiv-citation.el"))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}

#[test]
fn customization_group_and_elfeed_declaration_complete_the_package_variable_surface() {
    let elisp_form = r##"(list
 (get 'arxiv-citation 'custom-group)
 (get 'arxiv-citation 'group-documentation)
 (sort
  (copy-tree (get 'arxiv-citation 'custom-group))
  (lambda (left right)
    (string<
     (symbol-name (car left))
     (symbol-name (car right)))))
 (list
  (boundp 'elfeed-show-entry)
  (and
   (boundp 'elfeed-show-entry)
   elfeed-show-entry)
  (custom-variable-p 'elfeed-show-entry)
  (let ((file
         (symbol-file
          'elfeed-show-entry 'defvar)))
    (and
     file
     (file-name-nondirectory file))))
 (seq-filter
  (lambda (symbol)
    (and
     (or (fboundp symbol)
         (boundp symbol))
     (string-prefix-p
      "arxiv-citation-" (symbol-name symbol))))
  (sort
   (apropos-internal "^arxiv-citation-")
   (lambda (left right)
     (string< (symbol-name left)
              (symbol-name right))))))"##;
    let expect = expect![[
        r#"OK (((arxiv-citation-bibtex-files custom-variable) (arxiv-citation-library custom-variable) (arxiv-citation-open-pdf-function custom-variable) (arxiv-citation-max-authors custom-variable) (arxiv-citation-overwrite-file custom-variable)) "Utility functions for dealing with arXiv papers." ((arxiv-citation-bibtex-files custom-variable) (arxiv-citation-library custom-variable) (arxiv-citation-max-authors custom-variable) (arxiv-citation-open-pdf-function custom-variable) (arxiv-citation-overwrite-file custom-variable)) (nil nil nil nil) (arxiv-citation-arXiv-id arxiv-citation-bibtex-files arxiv-citation-download-and-open arxiv-citation-elfeed arxiv-citation-generate-autokey arxiv-citation-get-arxiv-citation arxiv-citation-get-citation arxiv-citation-get-details arxiv-citation-get-zbmath-citation arxiv-citation-gui arxiv-citation-library arxiv-citation-max-authors arxiv-citation-open-pdf-function arxiv-citation-overwrite-file arxiv-citation-parse arxiv-citation-pdf-link arxiv-citation-pdf-name))"#
    ]];
    assert_arxiv_citation_parity(elisp_form, expect);
}
