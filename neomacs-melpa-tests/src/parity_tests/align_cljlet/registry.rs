use expect_test::expect;

use super::{assert_align_cljlet_autoload_parity, assert_align_cljlet_parity};

#[test]
fn align_cljlet_registers_the_complete_callable_surface_with_exact_signatures() {
    let elisp_form = r##"(mapcar
 (lambda (fn)
   (list fn (help-function-arglist fn t) (commandp fn)
         (documentation fn)))
 '(acl-found-alignable-form acl-try-go-up acl-find-alignable-form
   acl-skip-commented acl-is-commented? acl-forward-sexp
   acl-goto-next-pair acl-get-width acl-has-next-sexp acl-next-sexp
   acl-calc-route-widths acl-check-for-another-sexp acl-calc-width
   acl-lines-correctly-paired acl-respace-single-let acl-respace-subform
   acl-respace-defroute-form acl-respace-form acl-take-n
   acl-start-align-defroute acl-position-to-start acl-align-form
   acl-backward-to-code align-cljlet))"##;
    let expect = expect![[
        r#"OK ((acl-found-alignable-form nil nil "Check if we are currently looking at a let form") (acl-try-go-up nil nil "Go upwards if possible.  If we can’t then we’re obviously not in an\n   alignable form.") (acl-find-alignable-form nil nil "Find the let form by moving looking upwards until nowhere to go") (acl-skip-commented nil nil nil) (acl-is-commented? nil nil nil) (acl-forward-sexp (&optional dont-skip-comments) nil "Jumps the cursor forward to the end of the current sexp or to\nthe end of the next sexp if already positioned at the\nend. Commented forms are skipped by default unless\ndont-skip-comments is true.") (acl-goto-next-pair nil nil "Skip ahead to the next definition") (acl-get-width nil nil "Get the width of the current definition") (acl-has-next-sexp nil nil "Checks if there is another sexp after the point") (acl-next-sexp nil nil "Goes to the next sexp, returning true or false if there is no next") (acl-calc-route-widths nil nil "Calculate the widths required to align a defroutes macro") (acl-check-for-another-sexp nil nil "Is there another sexp after this") (acl-calc-width nil nil "Calculate the width needed for all the definitions in the form") (acl-lines-correctly-paired nil nil "Determine if all the pairs are on different lines") (acl-respace-single-let (max-width) nil "Respace the current definition") (acl-respace-subform (widths) nil "Respace a subform using the widths given. Point must\nbe positioned on the first s-exp in the subform.") (acl-respace-defroute-form (widths) nil "Respace the entire defroute definition. Point must be\npositioned on the defroute form.") (acl-respace-form (width) nil "Respace the entire definition") (acl-take-n (n xs) nil "Take n elements from a list returning a new list") (acl-start-align-defroute nil nil nil) (acl-position-to-start nil nil nil) (acl-align-form nil nil "Determine what type of form we are currently positioned at and align it") (acl-backward-to-code nil nil "Move point back to the start of a preceding sexp form.\nThis gets out of strings, comments, backslash quotes, etc, to a\nplace where it makes sense to start examining sexp code forms.\n\nThe preceding form is found by a ‘parse-partial-sexp’ starting\nfrom ‘beginning-of-defun’.  If it finds nothing then just go to\n‘beginning-of-defun’.") (align-cljlet nil t "Align a let form so that the bindings neatly align into columns"))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_custom_variable_and_feature_metadata_match_the_frozen_package() {
    let elisp_form = r##"(list
 (featurep 'align-cljlet)
 (featurep 'clojure-mode)
 defroute-columns
 (default-value 'defroute-columns)
 (get 'defroute-columns 'custom-type)
 (get 'defroute-columns 'custom-group)
 (get 'defroute-columns 'variable-documentation)
 (get 'align-cljlet 'custom-group)
 (commandp 'align-cljlet)
 (interactive-form 'align-cljlet)
 (package-get-version)
 (file-name-nondirectory (getenv "NEOMACS_PACKAGE_SOURCE")))"##;
    let expect = expect![[
        r#"OK (t t 1 1 integer nil "The number of columns to align in a defroute call" ((defroute-columns custom-variable)) t (interactive nil) nil "align-cljlet.el")"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_autoload_exposes_only_the_public_command_without_loading_runtime() {
    let elisp_form = r##"(list
 (featurep 'align-cljlet)
 (featurep 'clojure-mode)
 (fboundp 'align-cljlet)
 (autoloadp (symbol-function 'align-cljlet))
 (commandp 'align-cljlet)
 (help-function-arglist 'align-cljlet t)
 (fboundp 'acl-align-form)
 (boundp 'defroute-columns)
 (file-name-nondirectory
  (nth 1 (symbol-function 'align-cljlet))))"##;
    let expect = expect![[
        r#"OK (nil t t t t "[Arg list not available until function definition is loaded.]" nil nil "align-cljlet")"#
    ]];
    assert_align_cljlet_autoload_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_installed_payload_contains_runtime_metadata_but_no_upstream_tests_or_docs() {
    let elisp_form = r##"(let* ((directory
         (file-name-directory (getenv "NEOMACS_PACKAGE_SOURCE")))
        (files (sort (directory-files directory nil "\\`[^.]") #'string-lessp)))
  (list
   files
   (mapcar
    (lambda (file)
      (let ((path (expand-file-name file directory)))
        (list file
              (file-attribute-size (file-attributes path))
              (file-readable-p path))))
    files)
   (seq-filter
    (lambda (file)
      (string-match-p "\\(?:test\\|README\\.md\\|\\.tar\\)\\'" file))
    files)))"##;
    let expect = expect![[
        r#"OK (("align-cljlet-autoloads.el" "align-cljlet-pkg.el" "align-cljlet.el" "align-cljlet.elc") (("align-cljlet-autoloads.el" 825 t) ("align-cljlet-pkg.el" 541 t) ("align-cljlet.el" 11546 t) ("align-cljlet.elc" 6927 t)) nil)"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_frozen_runtime_carries_exact_revision_dependency_and_autoload_markers() {
    let elisp_form = r##"(with-temp-buffer
  (insert-file-contents-literally (getenv "NEOMACS_PACKAGE_SOURCE"))
  (list
   (buffer-size)
   (line-number-at-pos (point-max))
   (goto-char (point-min))
   (re-search-forward "^;; Package-Version: \\(.+\\)$" nil t)
   (match-string-no-properties 1)
   (re-search-forward "^;; Package-Revision: \\(.+\\)$" nil t)
   (match-string-no-properties 1)
   (re-search-forward "^;; Package-Requires: \\(.+\\)$" nil t)
   (match-string-no-properties 1)
   (how-many "^;;;###autoload$" (point-min) (point-max))
   (how-many "^(defun " (point-min) (point-max))
   (how-many "^(defcustom " (point-min) (point-max))))"##;
    let expect = expect![[
        r#"OK (11546 372 1 233 "20160112.2101" 267 "ebcf0a912e83" 314 "((clojure-mode \"1.11.5\"))" 1 24 1)"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}
