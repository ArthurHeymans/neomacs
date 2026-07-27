use expect_test::expect;

use super::{
    assert_all_the_icons_completion_autoload_parity, assert_all_the_icons_completion_parity,
};

#[test]
fn exact_release_descriptor_and_installed_dependency_graph_are_stable() {
    let elisp_form = r##"
(let* ((descriptor
        (cadr (assq 'all-the-icons-completion package-alist)))
       (dependency
        (cadr (assq 'all-the-icons package-alist)))
       (extras (package-desc-extras descriptor)))
  (list
   (package-desc-name descriptor)
   (package-version-join (package-desc-version descriptor))
   (package-desc-reqs descriptor)
   (alist-get :commit extras)
   (alist-get :url extras)
   (package-version-join (package-desc-version dependency))
   (package-installed-p 'all-the-icons '(5 0))
   (featurep 'all-the-icons-completion)
   (featurep 'all-the-icons)
   (file-name-nondirectory (locate-library "all-the-icons-completion"))
   (file-name-nondirectory (locate-library "all-the-icons"))))
"##;
    let expect = expect![[
        r#"OK (all-the-icons-completion "20240128.2048" ((emacs (26 1)) (all-the-icons (5 0))) "4c8bcad8033f5d0868ce82ea3807c6cd46c4a198" "https://github.com/iyefrat/all-the-icons-completion" "20250527.927" t t t "all-the-icons-completion.el" "all-the-icons.el")"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn public_function_signatures_interactive_contracts_and_documentation_are_stable() {
    let elisp_form = r##"
(mapcar
 (lambda (function)
   (list
    function
    (help-function-arglist function t)
    (interactive-form function)
    (secure-hash 'sha256 (documentation function))))
 '(all-the-icons-completion-get-icon
   all-the-icons-completion-completion-metadata-get
   all-the-icons-completion-marginalia-setup
   all-the-icons-completion-mode))
"##;
    let expect = expect![[
        r#"OK ((all-the-icons-completion-get-icon (cand cat) nil "e7a36e9911179d987344532a1b6afe11e284334dda15fe1b7ebbbefb535024fa") (all-the-icons-completion-completion-metadata-get (orig metadata prop) nil "a940556797c35ac9d5903b9039409aea15dd1858f14f2519685d3c98ebeb3d0d") (all-the-icons-completion-marginalia-setup nil nil "e20ae5a7bf5e6a7336eeaec9f915adfcb68dd1e5474a4e3c6b48e938189d816d") (all-the-icons-completion-mode (&optional arg) (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "8774c780dabd49b33bdf2161349fa456ba9177a9f7afdb12f260a493903acb89"))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn customization_group_directory_face_and_minor_mode_metadata_match_release_contract() {
    let elisp_form = r##"
(list
 (get 'all-the-icons-completion 'custom-group)
 (get 'all-the-icons-completion 'group-documentation)
 (facep 'all-the-icons-completion-dir-face)
 (get 'all-the-icons-completion-dir-face 'face-defface-spec)
 (get 'all-the-icons-completion-dir-face 'custom-group)
 (get 'all-the-icons-completion-dir-face 'face-documentation)
 (custom-variable-p 'all-the-icons-completion-mode)
 (get 'all-the-icons-completion-mode 'custom-type)
 (get 'all-the-icons-completion-mode 'custom-group))
"##;
    let expect = expect![[
        r#"OK (((all-the-icons-completion-mode custom-variable)) "Add icons to completion candidates." [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t nil)) nil "Face for the directory icon." ((funcall #'#[nil (nil) (marginalia-mode t)])) boolean nil)"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn source_load_registers_generic_methods_without_enabling_global_advice() {
    let elisp_form = r##"
(list
 all-the-icons-completion-mode
 (and
  (advice-member-p
   #'all-the-icons-completion-completion-metadata-get
   #'completion-metadata-get)
  t)
 (all-the-icons-completion-get-icon "candidate" 'unknown-category)
 (mapcar
  (lambda (case)
    (condition-case error-data
        (list
         (car case)
         'value
         (apply #'all-the-icons-completion-get-icon case))
      (error
       (list
        (car case)
        'signal
        (car error-data)
        (cadr error-data)))))
  '(("plain" nil)
    ("plain" integer)
    (nil unknown-category))))
"##;
    let expect =
        expect![[r#"OK (nil nil "" (("plain" value "") ("plain" value "") (nil value "")))"#]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_expose_only_documented_entry_points_without_eager_source_load() {
    let elisp_form = r##"
(mapcar
 (lambda (symbol)
   (let ((definition (and (fboundp symbol) (symbol-function symbol))))
     (list
      symbol
      (and (autoloadp definition) t)
      (and (autoloadp definition) (nth 1 definition))
      (and (autoloadp definition) (nth 3 definition))
      (and (autoloadp definition) (nth 4 definition)))))
 '(all-the-icons-completion-marginalia-setup
   all-the-icons-completion-mode
   all-the-icons-completion-get-icon
   all-the-icons-completion-completion-metadata-get))
"##;
    let expect = expect![[
        r#"OK ((all-the-icons-completion-marginalia-setup t "all-the-icons-completion" nil nil) (all-the-icons-completion-mode t "all-the-icons-completion" t nil) (all-the-icons-completion-get-icon nil nil nil nil) (all-the-icons-completion-completion-metadata-get nil nil nil nil))"#
    ]];
    assert_all_the_icons_completion_autoload_parity(elisp_form, expect);
}

#[test]
fn autoloaded_mode_loads_exact_source_and_installs_then_removes_real_advice() {
    let elisp_form = r##"
(let ((before (featurep 'all-the-icons-completion)))
  (unwind-protect
      (progn
        (all-the-icons-completion-mode 1)
        (list
         before
         (featurep 'all-the-icons-completion)
         all-the-icons-completion-mode
         (and
          (advice-member-p
           #'all-the-icons-completion-completion-metadata-get
           #'completion-metadata-get)
          t)
         (progn
           (all-the-icons-completion-mode -1)
           all-the-icons-completion-mode)
         (advice-member-p
          #'all-the-icons-completion-completion-metadata-get
          #'completion-metadata-get)))
    (when (featurep 'all-the-icons-completion)
      (all-the-icons-completion-mode -1))))
"##;
    let expect = expect!["OK (nil t t t nil nil)"];
    assert_all_the_icons_completion_autoload_parity(elisp_form, expect);
}
