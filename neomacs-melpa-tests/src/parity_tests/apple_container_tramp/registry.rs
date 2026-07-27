use expect_test::expect;

use super::{assert_apple_container_tramp_autoload_parity, assert_apple_container_tramp_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_and_emacs_requirement() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'apple-container-tramp package-alist)))
       (directory (package-desc-dir description)))
  (list
   (featurep 'apple-container-tramp)
   (package-installed-p 'apple-container-tramp)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (package-desc-extras description)
   (file-name-nondirectory
    (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t t apple-container-tramp "20260504.1350" "TRAMP integration for apple container." ((emacs (24 3))) ((:revdesc . "f47d58d029c5") (:commit . "f47d58d029c594f4c9e9b1cfff79630de68a9cb5") (:url . "https://github.com/major1201/apple-container-tramp.el")) "apple-container-tramp-20260504.1350")"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn installed_archive_contains_only_the_recipe_selected_runtime_and_descriptor() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'apple-container-tramp package-alist)))
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
    let expect =
        expect![[r#"OK (("apple-container-tramp-pkg.el" 314) ("apple-container-tramp.el" 5238))"#]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn installed_runtime_and_descriptor_match_the_exact_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'apple-container-tramp package-alist)))
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
   '("apple-container-tramp.el"
     "apple-container-tramp-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("apple-container-tramp.el" 5238 "f9844f79f95743b99789487fc819bb5c775135ae40ca22172dbf5064b7f2406b") ("apple-container-tramp-pkg.el" 314 "179b005abddd3304d01aa7f74a750f2d042ae89aac71b43b1ff94fe69c330547"))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
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
 '(apple-container-tramp--running-containers
   apple-container-tramp--parse-running-containers
   apple-container-tramp-cleanup
   apple-container-tramp-add-method
   apple-container-tramp-setup))"##;
    let expect = expect![[
        r#"OK ((apple-container-tramp--running-containers t nil nil nil nil "apple-container-tramp.el") (apple-container-tramp--parse-running-containers t nil nil (&optional _) nil "apple-container-tramp.el") (apple-container-tramp-cleanup t nil t nil (interactive nil) "apple-container-tramp.el") (apple-container-tramp-add-method t nil nil nil nil "apple-container-tramp.el") (apple-container-tramp-setup t nil nil nil nil "apple-container-tramp.el"))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn customization_group_option_and_public_constants_preserve_their_contracts() {
    let elisp_form = r##"(list
 (get 'apple-container-tramp 'custom-group)
 (get 'apple-container-tramp 'group-documentation)
 (get 'apple-container-tramp 'custom-prefix)
 (list
  'apple-container-tramp-container-options
  apple-container-tramp-container-options
  (eval
   (car
    (get 'apple-container-tramp-container-options
         'standard-value)))
  (get 'apple-container-tramp-container-options
       'custom-type)
  (get 'apple-container-tramp-container-options
       'custom-group)
  (get 'apple-container-tramp-container-options
       'variable-documentation))
 (mapcar
  (lambda (symbol)
    (list symbol
          (symbol-value symbol)
          (get symbol 'variable-documentation)))
  '(apple-container-tramp-completion-function-alist
    apple-container-tramp-method)))"##;
    let expect = expect![[
        r#"OK (((apple-container-tramp-container-options custom-variable)) "TRAMP integration for Apple containers." "apple-container-tramp-" (apple-container-tramp-container-options nil nil (repeat string) nil "List of container options.") ((apple-container-tramp-completion-function-alist ((apple-container-tramp--parse-running-containers "")) "Default list of (FUNCTION FILE) pairs to be examined for container method.") (apple-container-tramp-method "container" "Method to connect containers.")))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_preserves_options_constants_commands_and_deferred_setup() {
    let elisp_form = r##"(list
 (featurep 'apple-container-tramp)
 (featurep 'apple-container-tramp-autoloads)
 (boundp 'apple-container-tramp-container-options)
 apple-container-tramp-container-options
 (get 'apple-container-tramp-container-options
      'custom-autoload)
 (mapcar
  (lambda (symbol)
    (list symbol
          (boundp symbol)
          (and (boundp symbol) (symbol-value symbol))))
  '(apple-container-tramp-completion-function-alist
    apple-container-tramp-method))
 (mapcar
  (lambda (symbol)
    (list symbol
          (fboundp symbol)
          (autoloadp (symbol-function symbol))
          (commandp symbol)))
  '(apple-container-tramp-cleanup
    apple-container-tramp-add-method
    apple-container-tramp-setup))
 (memq #'apple-container-tramp-setup tramp-load-hook))"##;
    let expect = expect![[
        r#"OK (nil t t nil noset ((apple-container-tramp-completion-function-alist t ((apple-container-tramp--parse-running-containers ""))) (apple-container-tramp-method t "container")) ((apple-container-tramp-cleanup t t t) (apple-container-tramp-add-method t t nil) (apple-container-tramp-setup nil nil nil)) (apple-container-tramp-setup))"#
    ]];
    assert_apple_container_tramp_autoload_parity(elisp_form, expect);
}
