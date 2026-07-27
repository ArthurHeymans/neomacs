use expect_test::expect;

use super::assert_angular_snippets_parity;

#[test]
fn angular_snippets_expands_controller_with_dependency_and_function_body() {
    let elisp_form = r##"(with-temp-buffer
  (js-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/js-mode/ngc.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'js-mode
                         (list definition))
    (insert "app.")
    (yas-expand-snippet
     (yas-lookup-snippet "ngc" 'js-mode))
    (dolist
        (replacement
         '("CatalogController"
           "CatalogService"))
      (let ((field (yas-current-field)))
        (delete-region
         (yas--field-start field)
         (yas--field-end field))
        (goto-char (yas--field-start field))
        (insert replacement))
      (yas-next-field-or-maybe-expand))
    (insert
     "$scope.products = "
     "CatalogService.all();")
    (yas-next-field-or-maybe-expand)
    (list
     (buffer-string)
     (point)
     (null (yas-active-snippets)))))"##;
    let expect = expect![[
        r#"OK ("app.controller('CatalogController', function ($scope, CatalogService) {\n    $scope.products = CatalogService.all();\n});" 77 nil)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_directive_link_function_with_all_runtime_parameters() {
    let elisp_form = r##"(with-temp-buffer
  (js-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/js-mode/ngd.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'js-mode
                         (list definition))
    (insert "app.")
    (yas-expand-snippet
     (yas-lookup-snippet "ngd" 'js-mode))
    (dolist
        (replacement
         '("focusWhen"
           "$timeout"
           "scope.$watch(attrs.focusWhen, "
           ", controller"))
      (let ((field (yas-current-field)))
        (delete-region
         (yas--field-start field)
         (yas--field-end field))
        (goto-char (yas--field-start field))
        (insert replacement))
      (yas-next-field-or-maybe-expand))
    (list
     (buffer-string)
     (null (yas-active-snippets)))))"##;
    let expect = expect![[
        r#"OK ("app.directive('focusWhen', function ($timeout) {\n    return function (scope, element, attrs, controller) {\n\11scope.$watch(attrs.focusWhen, \n    };\n});\n" t)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_filter_and_preserves_upstream_shared_third_field() {
    let elisp_form = r##"(with-temp-buffer
  (js-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/js-mode/ngfi.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'js-mode
                         (list definition))
    (insert "app.")
    (yas-expand-snippet
     (yas-lookup-snippet "ngfi" 'js-mode))
    (dolist
        (replacement
         '("currencyLabel"
           "$locale"
           "symbol"))
      (let ((field (yas-current-field)))
        (delete-region
         (yas--field-start field)
         (yas--field-end field))
        (goto-char (yas--field-start field))
        (insert replacement))
      (yas-next-field-or-maybe-expand))
    (list
     (buffer-string)
     (null (yas-active-snippets)))))"##;
    let expect = expect![[
        r#"OK ("app.filter(\"currencyLabel\", function ($locale) {\n    return function (input, symbol) {\n\11symbol\n    };\n});\n" t)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_route_with_resolve_block_and_continuation() {
    let elisp_form = r##"(with-temp-buffer
  (js-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/js-mode/ngrwr.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'js-mode
                         (list definition))
    (yas-expand-snippet
     (yas-lookup-snippet
      "ngrwr" 'js-mode))
    (dolist
        (replacement
         '("/products/:id"
           "product/show.html"
           "ProductController"))
      (let ((field (yas-current-field)))
        (delete-region
         (yas--field-start field)
         (yas--field-end field))
        (goto-char (yas--field-start field))
        (insert replacement))
      (yas-next-field-or-maybe-expand))
    (insert
     "\n    product: function "
     "(Product, $route) {\n"
     "      return Product.get("
     "$route.current.params.id);\n"
     "    }")
    (yas-next-field-or-maybe-expand)
    (insert "\nnextRoute();")
    (list
     (buffer-string)
     (point)
     (null (yas-active-snippets)))))"##;
    let expect = expect![[
        r#"OK ("$routeProvider.when(\"/products/:id\", {\n    templateUrl: \"product/show.html\",\n    controller: \"ProductController\",\n    resolve: {\nnextRoute();\n    product: function (Product, $route) {\n      return Product.get($route.current.params.id);\n    }\n    }\n});\n" 142 nil)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_scope_event_broadcast_emit_and_listener_workflow() {
    let elisp_form = r##"(let (results)
  (dolist
      (case
       '(("$b"
          "cart:updated"
          "cart")
         ("$e"
          "checkout:complete"
          "receipt")
         ("$on"
          "session:expired"
          "reason")))
    (with-temp-buffer
      (js-mode)
      (yas-minor-mode 1)
      (let* ((name (car case))
             (file
              (expand-file-name
               (format
                "snippets/js-mode/%s.yasnippet"
                name)
               angular-snippets-root))
             (definition
              (with-temp-buffer
                (insert-file-contents file)
                (yas--parse-template file))))
        (yas-define-snippets
         'js-mode
         (list definition))
        (yas-expand-snippet
         (yas-lookup-snippet name 'js-mode))
        (dolist (replacement (cdr case))
          (let ((field (yas-current-field)))
            (delete-region
             (yas--field-start field)
             (yas--field-end field))
            (goto-char
             (yas--field-start field))
            (insert replacement))
          (yas-next-field-or-maybe-expand))
        (when
            (string= name "$on")
          (insert
           "notify(reason);")
          (yas-next-field-or-maybe-expand))
        (push
         (list name
               (buffer-string)
               (null
                (yas-active-snippets)))
         results))))
  (nreverse results))"##;
    let expect = expect![[
        r#"OK (("$b" "$scope.$broadcast(\"cart:updated\", cart);\n" t) ("$e" "$scope.$emit(\"checkout:complete\", receipt);\n" t) ("$on" "$scope.$on(\"session:expired\", function (event, reason) {\n    notify(reason);\n});" nil))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_scope_watch_with_expression_and_change_handler() {
    let elisp_form = r##"(with-temp-buffer
  (js-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/js-mode/$w.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'js-mode
                         (list definition))
    (yas-expand-snippet
     (yas-lookup-snippet "$w" 'js-mode))
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "cart.total"))
    (yas-next-field-or-maybe-expand)
    (insert
     "if (newValue !== oldValue) {\n"
     "    recalculateTax(newValue);\n"
     "  }")
    (yas-next-field-or-maybe-expand)
    (list
     (buffer-string)
     (null (yas-active-snippets)))))"##;
    let expect = expect![[
        r#"OK ("$scope.$watch(\"cart.total\", function (newValue, oldValue) {\n    if (newValue !== oldValue) {\n    recalculateTax(newValue);\n  }\n});" nil)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_scope_value_assignment_mirrors_property_name_into_value() {
    let elisp_form = r##"(with-temp-buffer
  (js-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/js-mode/$va.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'js-mode
                         (list definition))
    (yas-expand-snippet
     (yas-lookup-snippet "$va" 'js-mode))
    (let ((field (yas-current-field)))
      (delete-region
       (yas--field-start field)
       (yas--field-end field))
      (goto-char (yas--field-start field))
      (insert "selectedProduct"))
    (yas-next-field-or-maybe-expand)
    (insert
     "$scope.persistSelection();")
    (list
     (buffer-string)
     (null (yas-active-snippets)))))"##;
    let expect = expect![[
        r#"OK ("$scope.selectedProduct = selectedProduct;\n$scope.persistSelection();" t)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_module_service_factory_and_otherwise_composition() {
    let elisp_form = r##"(let (results)
  (dolist
      (case
       '(("ngm"
          "checkout"
          "'ngRoute', 'ngAnimate'")
         ("ngs"
          "CartService"
          "this.total = function () { return 42; };")
         ("ngfa"
          "catalog"
          "$http"
          "return { all: function () { return $http.get('/products'); } };")
         ("ngro"
          "/not-found")))
    (with-temp-buffer
      (js-mode)
      (yas-minor-mode 1)
      (let* ((name (car case))
             (file
              (expand-file-name
               (format
                "snippets/js-mode/%s.yasnippet"
                name)
               angular-snippets-root))
             (definition
              (with-temp-buffer
                (insert-file-contents file)
                (yas--parse-template file))))
        (yas-define-snippets
         'js-mode
         (list definition))
        (insert "app.")
        (yas-expand-snippet
         (yas-lookup-snippet name 'js-mode))
        (dolist (replacement (cdr case))
          (let ((field (yas-current-field)))
            (delete-region
             (yas--field-start field)
             (yas--field-end field))
            (goto-char
             (yas--field-start field))
            (insert replacement))
          (yas-next-field-or-maybe-expand))
        (yas-exit-all-snippets)
        (push
         (list name (buffer-string))
         results))))
  (nreverse results))"##;
    let expect = expect![[
        r#"OK (("ngm" "app.angular.module(\"checkout\", ['ngRoute', 'ngAnimate']);\n") ("ngs" "app.service(\"CartService\", function () {\n    this.total = function () { return 42; };\n});\n") ("ngfa" "app.factory(\"catalog\", function ($http) {\n    return { all: function () { return $http.get('/products'); } };\n})\n") ("ngro" "app.$routeProvider.otherwise({redirectTo: \"/not-found\"});\n"))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_from_real_javascript_trigger_and_removes_key() {
    let elisp_form = r##"(with-temp-buffer
  (js-mode)
  (yas-minor-mode 1)
  (setq yas--tables
        (make-hash-table))
  (let* ((file
          (expand-file-name
           "snippets/js-mode/ngm.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'js-mode
                         (list definition))
    (insert "const app = ngm")
    (let ((expanded (yas-expand)))
      (insert "inventory")
      (yas-next-field-or-maybe-expand)
      (insert "'ngResource'")
      (yas-next-field-or-maybe-expand)
      (list
       expanded
       (buffer-string)
       (point)
       (null
        (yas-active-snippets))))))"##;
    let expect =
        expect![[r#"OK (t "const app = angular.module(\"inventory\", ['ngResource']);\n" 58 t)"#]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_javascript_lookup_recovers_every_installed_template_by_exact_name() {
    let elisp_form = r##"(let* ((directory
         (expand-file-name
          "snippets/js-mode"
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
   'js-mode
   (nreverse definitions))
  (mapcar
   (lambda (file)
     (let* ((name
             (file-name-base file))
            (template
             (yas-lookup-snippet
              name 'js-mode)))
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
        r#"OK (("$b" "$b" "$b" "$b.yasnippet" 31) ("$e" "$e" "$e" "$e.yasnippet" 26) ("$f" "$f" "$f" "$f.yasnippet" 36) ("$on" "$on" "$on" "$on.yasnippet" 48) ("$v" "$v" "$v" "$v.yasnippet" 18) ("$va" "$va" "$va" "$va.yasnippet" 18) ("$w" "$w" "$w" "$w.yasnippet" 60) ("ngc" "ngc" "ngc" "ngc.yasnippet" 49) ("ngd" "ngd" "ngd" "ngd.yasnippet" 94) ("ngfa" "ngfa" "ngfa" "ngfa.yasnippet" 38) ("ngfi" "ngfi" "ngfi" "ngfi.yasnippet" 77) ("ngm" "ngm" "ngm" "ngm.yasnippet" 28) ("ngro" "ngro" "ngro" "ngro.yasnippet" 48) ("ngrw" "ngrw" "ngrw" "ngrw.yasnippet" 74) ("ngrwr" "ngrwr" "ngrwr" "ngrwr.yasnippet" 92) ("ngs" "ngs" "ngs" "ngs.yasnippet" 37) ("ngw" "ngw" "ngw" "ngw.yasnippet" 56))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_js2_mode_inherits_javascript_template_and_expands_controller() {
    let elisp_form = r##"(with-temp-buffer
  (setq major-mode 'js2-mode)
  (yas-minor-mode 1)
  (setq yas--tables
        (make-hash-table))
  (let* ((parent-file
          (expand-file-name
           "snippets/js2-mode/.yas-parents"
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
           "snippets/js-mode/ngc.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents
             snippet-file)
            (yas--parse-template
             snippet-file))))
    (yas--define-parents
     'js2-mode
     (list parent))
    (yas-define-snippets
     parent
     (list definition))
    (insert "app.")
    (yas-expand-snippet
     (yas-lookup-snippet
      "ngc"
      'js2-mode))
    (insert "OrdersController")
    (yas-next-field-or-maybe-expand)
    (insert "OrderService")
    (yas-next-field-or-maybe-expand)
    (insert
     "$scope.orders = "
     "OrderService.pending();")
    (yas-next-field-or-maybe-expand)
    (list
     parent
     (gethash 'js2-mode yas--parents)
     (buffer-string)
     (null
      (yas-active-snippets)))))"##;
    let expect = expect![[
        r#"OK (js-mode (js-mode) "app.controller('OrdersController', function ($scope, OrderService) {\n$scope.orders = OrderService.pending();\n});" nil)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}
