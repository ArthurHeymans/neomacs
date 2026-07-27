use expect_test::expect;

use super::assert_angular_snippets_parity;

#[test]
fn angular_snippets_expands_ng_app_with_nested_module_field_and_adjacent_markup() {
    let elisp_form = r##"(with-temp-buffer
  (html-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/html-mode/ng-app.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'html-mode
                         (list definition))
    (insert "<main ")
    (yas-expand-snippet
     (yas-lookup-snippet
      "ng-app"
      'html-mode))
    (yas-next-field-or-maybe-expand)
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "store.frontend"))
    (yas-next-field-or-maybe-expand)
    (insert "class=\"shell\">")
    (list
     (buffer-string)
     (point)
     (null (yas-active-snippets))
     ng-snip/last-docs-message)))"##;
    let expect =
        expect![[r#"OK ("<main ng-app=\"store.frontend\"class=\"shell\">\n" 44 nil "ng-app")"#]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_ng_class_and_preserves_two_independent_fields() {
    let elisp_form = r##"(with-temp-buffer
  (html-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/html-mode/ng-class.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'html-mode
                         (list definition))
    (insert "<tr ")
    (yas-expand-snippet
     (yas-lookup-snippet
      "ng-class"
      'html-mode))
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "selected"))
    (yas-next-field-or-maybe-expand)
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "row.id == selectedId"))
    (yas-next-field-or-maybe-expand)
    (insert "data-row=\"42\">")
    (list
     (buffer-string)
     (point)
     (null (yas-active-snippets))
     ng-snip/last-docs-message)))"##;
    let expect = expect![[
        r#"OK ("<tr ng-class=\"{'selected': row.id == selectedIddata-row=\"42\">}\"" 62 nil "ng-class")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_ng_options_and_updates_all_mirrored_item_fields() {
    let elisp_form = r##"(with-temp-buffer
  (html-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/html-mode/ng-options.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'html-mode
                         (list definition))
    (insert "<select ")
    (yas-expand-snippet
     (yas-lookup-snippet
      "ng-options"
      'html-mode))
    (yas-next-field-or-maybe-expand)
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "product"))
    (yas-next-field-or-maybe-expand)
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "id"))
    (yas-next-field-or-maybe-expand)
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "displayName"))
    (yas-next-field-or-maybe-expand)
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "catalog.products"))
    (yas-next-field-or-maybe-expand)
    (insert "ng-model=\"selectedProduct\">")
    (list
     (buffer-string)
     (null (yas-active-snippets))
     ng-snip/last-docs-message)))"##;
    let expect = expect![[
        r#"OK ("<select ng-options=\"product.id as product.displayName for product in catalog.products\"ng-model=\"selectedProduct\">" t "ng-options")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_multiline_ng_pluralize_and_edits_each_phrase() {
    let elisp_form = r##"(with-temp-buffer
  (html-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/html-mode/ng-pluralize.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'html-mode
                         (list definition))
    (insert "<span ")
    (yas-expand-snippet
     (yas-lookup-snippet
      "ng-pluralize"
      'html-mode))
    (dolist
        (replacement
         '("cart.items.length"
           "No products"
           "One product"))
      (let ((field (yas-current-field)))
        (delete-region
         (yas--field-start field)
         (yas--field-end field))
        (goto-char (yas--field-start field))
        (insert replacement))
      (yas-next-field-or-maybe-expand))
    (insert "products")
    (yas-next-field-or-maybe-expand)
    (insert "class=\"count\">")
    (list
     (buffer-string)
     (null (yas-active-snippets))
     ng-snip/last-docs-message)))"##;
    let expect = expect![[
        r#"OK ("<span ng-pluralize count=\"cart.items.length\" when=\"{\n\11\11\11\11       '0': 'No products',\n\11\11\11\11       'one': 'One product',\n\11\11\11\11       'other': '{} class=\"count\">products'\n\11\11\11\11       }\"" nil "ng-pluralize")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_representative_quoted_directives_from_installed_files() {
    let elisp_form = r##"(let (results)
  (dolist
      (name
       '("ng-bind"
         "ng-change"
         "ng-controller"
         "ng-hide"
         "ng-href"
         "ng-model"
         "ng-submit"
         "ng-switch"))
    (with-temp-buffer
      (html-mode)
      (yas-minor-mode 1)
      (let* ((file
              (expand-file-name
               (format
                "snippets/html-mode/%s.yasnippet"
                name)
               angular-snippets-root))
             (definition
              (with-temp-buffer
                (insert-file-contents file)
                (yas--parse-template file))))
        (yas-define-snippets
         'html-mode
         (list definition))
        (insert "<div ")
        (yas-expand-snippet
         (yas-lookup-snippet
          name 'html-mode))
        (insert
         (format "%s-expression" name))
        (yas-exit-all-snippets)
        (insert "data-tail=\"kept\">")
        (push
         (list name
               (buffer-string)
               ng-snip/last-docs-message)
         results))))
  (nreverse results))"##;
    let expect = expect![[
        r#"OK (("ng-bind" "<div ng-bind=\"data-tail=\"kept\">ng-bind-expression\"" "ng-bind") ("ng-change" "<div ng-change=\"data-tail=\"kept\">ng-change-expression\"" "ng-change") ("ng-controller" "<div ng-controller=\"data-tail=\"kept\">ng-controller-expression\"" "ng-controller") ("ng-hide" "<div ng-hide=\"data-tail=\"kept\">ng-hide-expression\"" "ng-hide") ("ng-href" "<div ng-href=\"data-tail=\"kept\">ng-href-expression\"" "ng-href") ("ng-model" "<div ng-model=\"data-tail=\"kept\">ng-model-expression\"" "ng-model") ("ng-submit" "<div ng-submit=\"data-tail=\"kept\">ng-submit-expression\"" "ng-submit") ("ng-switch" "<div ng-switch=\"data-tail=\"kept\">ng-switch-expression\"" "ng-switch"))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_representative_flag_directives_without_quotes() {
    let elisp_form = r##"(let (results)
  (dolist
      (name
       '("ng-cloak"
         "ng-csp"
         "ng-form"
         "ng-list"
         "ng-readonly"
         "ng-transclude"
         "ng-view"))
    (with-temp-buffer
      (html-mode)
      (yas-minor-mode 1)
      (let* ((file
              (expand-file-name
               (format
                "snippets/html-mode/%s.yasnippet"
                name)
               angular-snippets-root))
             (definition
              (with-temp-buffer
                (insert-file-contents file)
                (yas--parse-template file))))
        (yas-define-snippets
         'html-mode
         (list definition))
        (insert "<section ")
        (yas-expand-snippet
         (yas-lookup-snippet
          name 'html-mode))
        (insert "aria-hidden=\"false\">")
        (push
         (list name
               (buffer-string)
               (null
                (yas-active-snippets))
               ng-snip/last-docs-message)
         results))))
  (nreverse results))"##;
    let expect = expect![[
        r#"OK (("ng-cloak" "<section ng-cloakaria-hidden=\"false\">" t "ng-cloak") ("ng-csp" "<section ng-csparia-hidden=\"false\">" t "ng-csp") ("ng-form" "<section ng-formaria-hidden=\"false\">" t "ng-form") ("ng-list" "<section ng-listaria-hidden=\"false\">" t "ng-list") ("ng-readonly" "<section ng-readonlyaria-hidden=\"false\">" t "ng-readonly") ("ng-transclude" "<section ng-transcludearia-hidden=\"false\">" t "ng-transclude") ("ng-view" "<section ng-viewaria-hidden=\"false\">" t "ng-view"))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_ng_include_with_nested_template_quotes() {
    let elisp_form = r##"(with-temp-buffer
  (html-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/html-mode/ng-include.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'html-mode
                         (list definition))
    (insert "<aside ")
    (yas-expand-snippet
     (yas-lookup-snippet
      "ng-include"
      'html-mode))
    (insert "partials/product-card.html")
    (yas-exit-all-snippets)
    (insert " class=\"card-host\">")
    (list
     (buffer-string)
     (point)
     ng-snip/last-docs-message)))"##;
    let expect = expect![[
        r#"OK ("<aside ng-include=\"' class=\"card-host\">partials/product-card.html'\"" 40 "ng-include")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_from_real_ng_trigger_after_deterministic_candidate_selection() {
    let elisp_form = r##"(with-temp-buffer
  (html-mode)
  (yas-minor-mode 1)
  (setq yas--tables
        (make-hash-table))
  (let* ((file
          (expand-file-name
           "snippets/html-mode/ng-repeat.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'html-mode
                         (list definition))
    (insert "<li ng")
    (let ((expanded (yas-expand)))
      (insert "order")
      (yas-next-field-or-maybe-expand)
      (insert "orders.pending")
      (yas-next-field-or-maybe-expand)
      (insert ">{{ order.total }}</li>")
      (list
       expanded
       (buffer-string)
       (point)
       (null (yas-active-snippets))
       ng-snip/last-docs-message))))"##;
    let expect = expect![[
        r#"OK (t "<li ng-repeat=\"order in orders.pending\">{{ order.total }}</li>" 63 t "ng-repeat")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_html_lookup_recovers_every_installed_template_by_exact_name() {
    let elisp_form = r##"(let* ((directory
         (expand-file-name
          "snippets/html-mode"
          angular-snippets-root))
        (files
         (sort
          (directory-files
           directory t
           "\\.yasnippet\\'")
          #'string<))
        definitions)
  (dolist (file files)
    (push
     (with-temp-buffer
       (insert-file-contents file)
       (yas--parse-template file))
     definitions))
  (yas-define-snippets
   'html-mode
   (nreverse definitions))
  (mapcar
   (lambda (file)
     (let* ((name
             (file-name-base file))
            (template
             (yas-lookup-snippet
              name 'html-mode)))
       (list
        name
        (yas--template-key template)
        (yas--template-name template)
        (file-name-nondirectory
         (yas--template-load-file
          template))
        (length
         (yas--template-content
          template)))))
   files))"##;
    let expect = expect![[
        r#"OK (("ng-app" "ng" "ng-app" "ng-app.yasnippet" 86) ("ng-bind-html-unsafe" "ng" "ng-bind-html-unsafe" "ng-bind-html-unsafe.yasnippet" 96) ("ng-bind-template" "ng" "ng-bind-template" "ng-bind-template.yasnippet" 90) ("ng-bind" "ng" "ng-bind" "ng-bind.yasnippet" 72) ("ng-change" "ng" "ng-change" "ng-change.yasnippet" 76) ("ng-checked" "ng" "ng-checked" "ng-checked.yasnippet" 78) ("ng-class-even" "ng" "ng-class-even" "ng-class-even.yasnippet" 84) ("ng-class-odd" "ng" "ng-class-odd" "ng-class-odd.yasnippet" 82) ("ng-class" "ng" "ng-class" "ng-class.yasnippet" 104) ("ng-click" "ng" "ng-click" "ng-click.yasnippet" 74) ("ng-cloak" "ng" "ng-cloak" "ng-cloak.yasnippet" 69) ("ng-controller" "ng" "ng-controller" "ng-controller.yasnippet" 84) ("ng-csp" "ng" "ng-csp" "ng-csp.yasnippet" 65) ("ng-dblclick" "ng" "ng-dblclick" "ng-dblclick.yasnippet" 80) ("ng-disabled" "ng" "ng-disabled" "ng-disabled.yasnippet" 80) ("ng-form" "ng" "ng-form" "ng-form.yasnippet" 67) ("ng-hide" "ng" "ng-hide" "ng-hide.yasnippet" 72) ("ng-href" "ng" "ng-href" "ng-href.yasnippet" 72) ("ng-include" "ng" "ng-include" "ng-include.yasnippet" 80) ("ng-init" "ng" "ng-init" "ng-init.yasnippet" 72) ("ng-list" "ng" "ng-list" "ng-list.yasnippet" 67) ("ng-model" "ng" "ng-model" "ng-model.yasnippet" 74) ("ng-mousedown" "ng" "ng-mousedown" "ng-mousedown.yasnippet" 82) ("ng-mouseenter" "ng" "ng-mouseenter" "ng-mouseenter.yasnippet" 84) ("ng-mouseleave" "ng" "ng-mouseleave" "ng-mouseleave.yasnippet" 84) ("ng-mousemove" "ng" "ng-mousemove" "ng-mousemove.yasnippet" 82) ("ng-mouseover" "ng" "ng-mouseover" "ng-mouseover.yasnippet" 82) ("ng-mouseup" "ng" "ng-mouseup" "ng-mouseup.yasnippet" 78) ("ng-multiple" "ng" "ng-multiple" "ng-multiple.yasnippet" 80) ("ng-non-bindable" "ng" "ng-non-bindable" "ng-non-bindable.yasnippet" 88) ("ng-options" "ng" "ng-options" "ng-options.yasnippet" 137) ("ng-pluralize" "ng" "ng-pluralize" "ng-pluralize.yasnippet" 167) ("ng-readonly" "ng" "ng-readonly" "ng-readonly.yasnippet" 75) ("ng-repeat" "ng" "ng-repeat" "ng-repeat.yasnippet" 96) ("ng-selected" "ng" "ng-selected" "ng-selected.yasnippet" 80) ("ng-show" "ng" "ng-show" "ng-show.yasnippet" 72) ("ng-src" "ng" "ng-src" "ng-src.yasnippet" 70) ("ng-style" "ng" "ng-style" "ng-style.yasnippet" 74) ("ng-submit" "ng" "ng-submit" "ng-submit.yasnippet" 76) ("ng-switch" "ng" "ng-switch" "ng-switch.yasnippet" 76) ("ng-transclude" "ng" "ng-transclude" "ng-transclude.yasnippet" 79) ("ng-view" "ng" "ng-view" "ng-view.yasnippet" 67))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_web_mode_inherits_html_template_and_expands_real_markup() {
    let elisp_form = r##"(with-temp-buffer
  (setq major-mode 'web-mode)
  (yas-minor-mode 1)
  (setq yas--tables
        (make-hash-table))
  (let* ((parent-file
          (expand-file-name
           "snippets/web-mode/.yas-parents"
           angular-snippets-root))
         (parent
          (with-temp-buffer
            (insert-file-contents
             parent-file)
            (intern
             (string-trim
              (buffer-string)))))
         (snippet-file
          (expand-file-name
           "snippets/html-mode/ng-model.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents
             snippet-file)
            (yas--parse-template
             snippet-file))))
    (yas--define-parents
     'web-mode
     (list parent))
    (yas-define-snippets
     parent
     (list definition))
    (insert "<input ")
    (yas-expand-snippet
     (yas-lookup-snippet
      "ng-model"
      'web-mode))
    (insert "checkout.email")
    (yas-exit-all-snippets)
    (insert " type=\"email\" />")
    (list
     parent
     (gethash 'web-mode yas--parents)
     (buffer-string)
     (null (yas-active-snippets))
     ng-snip/last-docs-message)))"##;
    let expect = expect![[
        r#"OK (html-mode (html-mode) "<input ng-model=\" type=\"email\" />checkout.email\"" t "ng-model")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}
