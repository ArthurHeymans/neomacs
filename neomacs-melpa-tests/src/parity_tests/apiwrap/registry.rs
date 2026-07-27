use expect_test::expect;

use super::assert_apiwrap_parity;

#[test]
fn apiwrap_descriptor_records_exact_pin_dependency_and_payload() {
    let elisp_form = r##"(let* ((desc (cadr (assq 'apiwrap package-alist)))
              (dir (package-desc-dir desc)))
         (list (package-version-join (package-desc-version desc))
               (package-desc-reqs desc)
               (package-desc-kind desc)
               (sort (mapcar #'file-name-nondirectory
                             (directory-files dir t "^[^.].*"))
                     #'string<)))"##;
    let expect = expect![[
        r#"OK ("20180602.2231" ((emacs (25))) nil ("README-elpa" "apiwrap-autoloads.el" "apiwrap-pkg.el" "apiwrap.el" "apiwrap.elc"))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_complete_callable_surface_arities_and_command_flags_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (macrop symbol)
                 (stringp (documentation symbol t))))
         '(apiwrap-genform-resolve-api-params
           apiwrap--encode-url
           apiwrap-plist->alist
           apiwrap--kw->sym
           apiwrap--docfn
           apiwrap--docmacro
           apiwrap-gensym
           apiwrap-stdgenlink
           apiwrap-genmacros
           apiwrap--maybe-apply
           apiwrap-gendefun
           apiwrap-new-backend
           apropos-api-endpoint))"##;
    let expect = expect![
        "OK ((apiwrap-genform-resolve-api-params (object url) nil nil t) (apiwrap--encode-url (thing) nil nil nil) (apiwrap-plist->alist (plist) nil nil t) (apiwrap--kw->sym (kw) nil nil t) (apiwrap--docfn (service-name doc object-param-doc method external-resource link) nil nil t) (apiwrap--docmacro (service-name method) nil nil t) (apiwrap-gensym (prefix api-method &optional resource) nil nil t) (apiwrap-stdgenlink (alist) nil nil t) (apiwrap-genmacros (name prefix standard-parameters functions) nil nil t) (apiwrap--maybe-apply (func value) nil nil t) (apiwrap-gendefun (name prefix standard-parameters method resource doc link objects internal-resource std-functions override-functions) nil nil t) (apiwrap-new-backend (name prefix standard-parameters &rest config) nil t t) (apropos-api-endpoint (backend pattern) t nil t))"
    ];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_feature_defaults_and_primitive_order_are_exact() {
    let elisp_form = r##"(list
         (featurep 'apiwrap)
         apiwrap-backends
         apiwrap-primitives
         (custom-variable-p 'apiwrap-backends)
         (get 'apiwrap-primitives 'risky-local-variable)
         (bound-and-true-p byte-compile-current-file))"##;
    let expect = expect!["OK (t nil (get put head post patch delete) nil t nil)"];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_source_load_registers_expected_definition_prefixes_only() {
    let elisp_form = r##"(let ((prefixes
                (sort
                 (delq nil
                       (mapcar
                        (lambda (symbol)
                          (when (string-prefix-p "apiwrap" (symbol-name symbol))
                            (symbol-name symbol)))
                        (apropos-internal "^apiwrap")))
                 #'string<)))
         prefixes)"##;
    let expect = expect![[
        r#"OK ("apiwrap" "apiwrap--docfn" "apiwrap--docmacro" "apiwrap--encode-url" "apiwrap--kw->sym" "apiwrap--maybe-apply" "apiwrap-autoloads" "apiwrap-backends" "apiwrap-gendefun" "apiwrap-genform-resolve-api-params" "apiwrap-genmacros" "apiwrap-gensym" "apiwrap-new-backend" "apiwrap-plist->alist" "apiwrap-primitives" "apiwrap-stdgenlink")"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_reload_preserves_backend_registry_and_does_not_duplicate_feature() {
    let elisp_form = r##"(let* ((source (symbol-file 'apiwrap-new-backend 'defun))
              (apiwrap-backends '(("Kept" . "kept")))
              (before-features (cl-count 'apiwrap features)))
         (load source nil 'nomessage)
         (list apiwrap-backends
               before-features
               (cl-count 'apiwrap features)
               (eq (car (memq 'apiwrap features)) 'apiwrap)))"##;
    let expect = expect![[r#"OK ((("Kept" . "kept")) 1 1 t)"#]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_indent_and_documentation_metadata_match_declared_contract() {
    let elisp_form = r##"(list
         (get 'apiwrap-genform-resolve-api-params 'lisp-indent-function)
         (get 'apiwrap-new-backend 'lisp-indent-function)
         (substring (documentation 'apiwrap-new-backend) 0 65)
         (documentation 'apropos-api-endpoint)
         (interactive-form 'apropos-api-endpoint))"##;
    let expect = expect![[
        r#"OK (1 2 "Define a new API backend.\n\nSERVICE-NAME is the name of the servic" "Apropos for API endpoints of BACKEND matching PATTERN." (interactive (let* ((b (completing-read "Search backend: " (mapcar #'car apiwrap-backends))) (b (assoc-string b apiwrap-backends)) (name (car b)) (prefix (cdr b))) (list prefix (apropos-read-pattern (concat name " API endpoints"))))))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_keyword_conversion_handles_keywords_symbols_numbers_and_nil() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (list value
                 (apiwrap--kw->sym value)
                 (eq value (apiwrap--kw->sym value))))
         '(:owner owner :two-words two-words 17 nil))"##;
    let expect = expect![
        "OK ((:owner owner nil) (owner owner t) (:two-words two-words nil) (two-words two-words t) (17 17 t) (nil nil t))"
    ];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_plist_conversion_preserves_reverse_pair_order_and_values() {
    let elisp_form = r##"(list
         (apiwrap-plist->alist
          '(:state closed :labels ("bug" "help wanted") raw-key 7 :nil nil))
         (apiwrap-plist->alist nil)
         (let ((value (list :mutable (list 1 2))))
           (let ((converted (apiwrap-plist->alist value)))
             (setcar (cdr (assq 'mutable converted)) 9)
             (list value converted))))"##;
    let expect = expect![[
        r#"OK (((nil) (raw-key . 7) (labels "bug" "help wanted") (state . closed)) nil ((:mutable #1=(9 2)) ((mutable . #1#))))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}
