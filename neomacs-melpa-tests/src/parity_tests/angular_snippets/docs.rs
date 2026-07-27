use expect_test::expect;

use super::assert_angular_snippets_parity;

#[test]
fn angular_snippets_builds_complete_directive_documentation_registry() {
    let elisp_form = r##"(list
  (length ng-directive-docstrings)
  (length ng-docs)
  (mapcar
   (lambda (entry)
     (list
      (car entry)
      (ng-snip/docs-value
       (car entry)
       :docstring)
      (ng-snip/docs-value
       (car entry)
       :docurl)))
   ng-directive-docstrings))"##;
    let expect = expect![[
        r#"OK (43 43 (("ng-app" "Auto-bootstraps an application, with optional module to load." "http://docs.angularjs.org/api/ng.directive:ngApp") ("ng-bind" "Replace text content of element with value of given expression." "http://docs.angularjs.org/api/ng.directive:ngBind") ("ng-bind-html-unsafe" "Set innerHTML of element to unsanitized value of given expression." "http://docs.angularjs.org/api/ng.directive:ngBindHtmlUnsafe") ("ng-bind-template" "Replace text content of element with given template." "http://docs.angularjs.org/api/ng.directive:ngBindTemplate") ("ng-change" "Eval the given expression when user changes the input. Requires ng-model." "http://docs.angularjs.org/api/ng.directive:ngChange") ("ng-checked" "Uses given expression to determine checked-state of checkbox." "http://docs.angularjs.org/api/ng.directive:ngChecked") ("ng-class" "Sets class names on element based on given expression." "http://docs.angularjs.org/api/ng.directive:ngClass") ("ng-class-even" "Like ng-class, but only on even rows. Requires ng-repeat." "http://docs.angularjs.org/api/ng.directive:ngClassEven") ("ng-class-odd" "Like ng-class, but only on odd rows. Requires ng-repeat." "http://docs.angularjs.org/api/ng.directive:ngClassOdd") ("ng-click" "Eval the given expression when element is clicked." "http://docs.angularjs.org/api/ng.directive:ngClick") ("ng-cloak" "Hides the element contents until compiled by angular." "http://docs.angularjs.org/api/ng.directive:ngCloak") ("ng-controller" "Assign controller to this element, along with a new scope." "http://docs.angularjs.org/api/ng.directive:ngController") ("ng-csp" "Enables Content Security Policy support. Should be on same element as ng-app." "http://docs.angularjs.org/api/ng.directive:ngCsp") ("ng-dblclick" "Eval the given expression when element is double clicked." "http://docs.angularjs.org/api/ng.directive:ngDblclick") ("ng-disabled" "Uses given expression to determine disabled-state of element." "http://docs.angularjs.org/api/ng.directive:ngDisabled") ("ng-form" "Nestable alias of the form directive." "http://docs.angularjs.org/api/ng.directive:ngForm") ("ng-hide" "Hides the element if the expression is truthy." "http://docs.angularjs.org/api/ng.directive:ngHide") ("ng-href" "Avoids bad URLs on links that are clicked before angular compiles them." "http://docs.angularjs.org/api/ng.directive:ngHref") ("ng-include" "Fetches, compiles and includes an external HTML fragment." "http://docs.angularjs.org/api/ng.directive:ngInclude") ("ng-init" "Evals expression before executing template during bootstrap." "http://docs.angularjs.org/api/ng.directive:ngInit") ("ng-list" "Text input that converts between comma-separated string and an array of strings." "http://docs.angularjs.org/api/ng.directive:ngList") ("ng-model" "Sets up two-way data binding. Works with input, select and textarea." "http://docs.angularjs.org/api/ng.directive:ngModel") ("ng-mousedown" "Eval the given expression on mousedown." "http://docs.angularjs.org/api/ng.directive:ngMousedown") ("ng-mouseenter" "Eval the given expression on mouseenter." "http://docs.angularjs.org/api/ng.directive:ngMouseenter") ("ng-mouseleave" "Eval the given expression on mouseleave." "http://docs.angularjs.org/api/ng.directive:ngMouseleave") ("ng-mousemove" "Eval the given expression on mousemove." "http://docs.angularjs.org/api/ng.directive:ngMousemove") ("ng-mouseover" "Eval the given expression on mouseover." "http://docs.angularjs.org/api/ng.directive:ngMouseover") ("ng-mouseup" "Eval the given expression on mouseup." "http://docs.angularjs.org/api/ng.directive:ngMouseup") ("ng-multiple" "Uses given expression to determine multiple-state of select element." "http://docs.angularjs.org/api/ng.directive:ngMultiple") ("ng-non-bindable" "Makes angular ignore {{bindings}} inside element." "http://docs.angularjs.org/api/ng.directive:ngNonBindable") ("ng-options" "Populates select options from a list or object." "http://docs.angularjs.org/api/ng.directive:select") ("ng-pluralize" "Helps change wording based on a number." "http://docs.angularjs.org/api/ng.directive:ngPluralize") ("ng-readonly" "Uses given expression to determine readonly-state of element." "http://docs.angularjs.org/api/ng.directive:ngReadonly") ("ng-repeat" "Repeats template for every item in a list." "http://docs.angularjs.org/api/ng.directive:ngRepeat") ("ng-selected" "Uses given expression to determine selected-state of option element." "http://docs.angularjs.org/api/ng.directive:ngSelected") ("ng-show" "Hides the element if the expression is falsy." "http://docs.angularjs.org/api/ng.directive:ngShow") ("ng-src" "Stops browser from fetching images with {{templates}} in the URL." "http://docs.angularjs.org/api/ng.directive:ngSrc") ("ng-style" "Sets style attributes from an object of DOM style properties. " "http://docs.angularjs.org/api/ng.directive:ngStyle") ("ng-submit" "Eval the given expression when form is submitted, and prevent default." "http://docs.angularjs.org/api/ng.directive:ngSubmit") ("ng-switch" "Switch on given expression to conditionally change DOM structure." "http://docs.angularjs.org/api/ng.directive:ngSwitch") ("ng-switch-when" "Include this element if value matches ng-switch on expression." "http://docs.angularjs.org/api/ng.directive:ngSwitch") ("ng-transclude" "Signifies where to insert transcluded DOM." "http://docs.angularjs.org/api/ng.directive:ngTransclude") ("ng-view" "Signifies where route views are shown." "http://docs.angularjs.org/api/ng.directive:ngView")))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_maps_standard_and_indirected_directives_to_exact_urls() {
    let elisp_form = r##"(mapcar
  #'ng-snip/directive-to-docs
  '(("ng-bind" . "bind")
    ("ng-options" . "options")
    ("ng-switch-when" . "switch")
    ("ng-bind-html-unsafe" . "html")
    ("ng-custom-long-name" . "custom")))"##;
    let expect = expect![[
        r#"OK (("ng-bind" :docstring "bind" :docurl "http://docs.angularjs.org/api/ng.directive:ngBind") ("ng-options" :docstring "options" :docurl "http://docs.angularjs.org/api/ng.directive:select") ("ng-switch-when" :docstring "switch" :docurl "http://docs.angularjs.org/api/ng.directive:ngSwitch") ("ng-bind-html-unsafe" :docstring "html" :docurl "http://docs.angularjs.org/api/ng.directive:ngBindHtmlUnsafe") ("ng-custom-long-name" :docstring "custom" :docurl "http://docs.angularjs.org/api/ng.directive:ngCustomLongName"))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_docs_value_handles_known_unknown_and_absent_properties() {
    let elisp_form = r##"(list
  (ng-snip/docs-value
   "ng-repeat"
   :docstring)
  (ng-snip/docs-value
   "ng-repeat"
   :docurl)
  (ng-snip/docs-value
   "ng-repeat"
   :missing)
  (ng-snip/docs-value
   "ng-does-not-exist"
   :docstring)
  (-aget
   '(("first" . 1)
     ("second" . 2)
     ("first" . 3))
   "first")
  (-aget nil "none"))"##;
    let expect = expect![[
        r#"OK ("Repeats template for every item in a list." "http://docs.angularjs.org/api/ng.directive:ngRepeat" nil nil 1 nil)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_docs_messages_user_records_state_and_schedules_expiry() {
    let elisp_form = r##"(let (scheduled)
  (cl-letf
      (((symbol-function
         'run-with-timer)
        (lambda
          (seconds repeat function
                   &rest arguments)
          (setq scheduled
                (list seconds repeat
                      function
                      arguments))
          'angular-doc-timer)))
    (setq ng-snip/last-docs-message
          "older")
    (let ((result
           (ng-snip/docs
            "ng-model")))
      (list
       result
       ng-snip/last-docs-message
       (current-message)
       scheduled))))"##;
    let expect =
        expect![[r#"OK (nil "ng-model" nil (10.0 nil ng-snip/forget-last-docs-message nil))"#]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_show_or_browse_docs_uses_second_invocation_for_browser() {
    let elisp_form = r##"(let (messages urls timers)
  (cl-letf
      (((symbol-function 'message)
        (lambda (format-string
                 &rest arguments)
          (let ((rendered
                 (apply #'format
                        format-string
                        arguments)))
            (push rendered messages)
            rendered)))
       ((symbol-function 'browse-url)
        (lambda (url &rest arguments)
          (push (cons url arguments)
                urls)
          'opened))
       ((symbol-function 'run-with-timer)
        (lambda (&rest arguments)
          (push arguments timers)
          'timer)))
    (setq ng-snip/last-docs-message nil)
    (let ((first
           (ng-snip/show-or-browse-docs
            "ng-options"))
          (second
           (ng-snip/show-or-browse-docs
            "ng-options"))
          (third
           (ng-snip/show-or-browse-docs
            "ng-repeat")))
      (list first second third
            (nreverse messages)
            (nreverse urls)
            (nreverse timers)
            ng-snip/last-docs-message))))"##;
    let expect = expect![[
        r#"OK (nil opened nil ("Populates select options from a list or object." "Repeats template for every item in a list.") (("http://docs.angularjs.org/api/ng.directive:select")) ((10.0 nil ng-snip/forget-last-docs-message) (10.0 nil ng-snip/forget-last-docs-message)) "ng-repeat")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_show_docs_at_point_drives_message_then_browser_from_markup() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<section ng-controller=\"StoreCtrl\" "
   "ng-repeat=\"product in products\">")
  (search-backward "product")
  (let (messages urls)
    (cl-letf
        (((symbol-function 'message)
          (lambda (text &rest arguments)
            (push
             (apply #'format text arguments)
             messages)))
         ((symbol-function 'browse-url)
          (lambda (url &rest _arguments)
            (push url urls)
            'opened))
         ((symbol-function 'run-with-timer)
          (lambda (&rest _arguments)
            'timer)))
      (setq ng-snip/last-docs-message nil)
      (let ((first
             (ng-snip-show-docs-at-point))
            (second
             (ng-snip-show-docs-at-point)))
        (list
         first second
         (nreverse messages)
         (nreverse urls)
         ng-snip/last-docs-message)))))"##;
    let expect = expect![[
        r#"OK (nil opened ("Repeats template for every item in a list.") ("http://docs.angularjs.org/api/ng.directive:ngRepeat") "ng-repeat")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_forget_last_docs_message_resets_repeat_action() {
    let elisp_form = r##"(progn
  (setq ng-snip/last-docs-message
        "ng-view")
  (list
   (ng-snip/forget-last-docs-message)
   ng-snip/last-docs-message
   (s-equals?
    ng-snip/last-docs-message
    "ng-view")))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_angular_snippets_parity(elisp_form, expect);
}
