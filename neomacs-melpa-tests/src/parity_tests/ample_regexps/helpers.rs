use expect_test::expect;

use super::assert_ample_regexps_parity;

#[test]
fn bound_interval_intersects_open_closed_and_unbounded_arity_ranges() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (let ((interval (nth 0 case))
         (lower (nth 1 case))
         (upper (nth 2 case)))
     (list
      interval lower upper
      (arx--bound-interval interval lower upper))))
 (list
  (list (list 0 most-positive-fixnum) nil nil)
  (list (list 0 most-positive-fixnum) 2 nil)
  (list (list 1 5) nil 3)
  (list (list 1 5) 2 4)
  (list (list 0 0) 0 0)
  (list (list 0 8) 10 3)
  (list (list 4 most-positive-fixnum) nil 4)))"##;
    let expect = expect![
        "OK (((0 2305843009213693951) nil nil (nil nil)) ((0 2305843009213693951) 2 nil (2 nil)) ((1 5) nil 3 (1 3)) ((1 5) 2 4 (2 4)) ((0 0) 0 0 (nil 0)) ((0 8) 10 3 (10 3)) ((4 2305843009213693951) nil 4 (4 4)))"
    ];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn function_arity_reports_custom_form_arguments_after_removing_form_name() {
    let elisp_form = r##"(progn
  (defun ample-regexps-test--required
      (_form left right)
    (list left right))
  (defun ample-regexps-test--optional
      (_form left &optional right third)
    (list left right third))
  (defun ample-regexps-test--rest
      (_form left &rest remaining)
    (cons left remaining))
  (mapcar
   (lambda (function)
     (list
      function
      (help-function-arglist function t)
      (arx--function-arity function)))
   '(ample-regexps-test--required
     ample-regexps-test--optional
     ample-regexps-test--rest)))"##;
    let expect = expect![
        "OK ((ample-regexps-test--required (_form left right) (2 2)) (ample-regexps-test--optional (_form left &optional right third) (1 3)) (ample-regexps-test--rest (_form left &rest remaining) (1 2305843009213693951)))"
    ];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn apply_func_post_27_distinguishes_raw_regexp_strings_from_structural_forms() {
    let elisp_form = r##"(list
 (arx--apply-func-post-27
  '(nil nil)
  nil
  (lambda (_form token)
    (format "<%s>" (regexp-quote token)))
  'tag
  '("api[v2]"))
 (arx--apply-func-post-27
  '(1 2)
  nil
  (lambda (_form first &optional second)
    `(seq ,first ,@(and second (list ":" second))))
  'pair
  '("left" "right"))
 (arx--apply-func-post-27
  '(0 0)
  nil
  (lambda (_form) "")
  'empty
  '()))"##;
    let expect = expect![[r#"OK ((regexp "<api\\[v2]>") (seq "left" ":" "right") (regexp ""))"#]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn form_to_rx_binding_handles_literal_regexp_symbol_alias_and_callable_shapes() {
    let elisp_form = r##"(mapcar
 (lambda (definition)
   (let* ((binding (arx--form-to-rx-binding definition))
          (body (cdr binding)))
     (list
      definition
      (car binding)
      (length body)
      (cond
       ((and (consp (car body))
             (eq (caar body) '&rest))
        (list
         'callable
         (length (car body))
         (car-safe (cadr body))))
       (t body)))))
 '((literal "a.b")
   (raw (regexp "^ready$"))
   (alias word)
   (nested (seq word ":" (+ digit)))
   (custom
    (:func
     (lambda (_form first &optional second)
       `(seq ,first ,@(and second (list second))))
     :min-args 1
     :max-args 2))))"##;
    let expect = expect![[
        r#"OK (((literal "a.b") literal 1 ("a.b")) ((raw #1=(regexp "^ready$")) raw 1 (#1#)) ((alias word) alias 1 (word)) ((nested #2=(seq word ":" (+ digit))) nested 1 (#2#)) ((custom (:func (lambda (_form first &optional second) `(seq ,first ,@(and second (list second)))) :min-args 1 :max-args 2)) custom 2 (callable 2 eval)))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn form_docstring_describes_every_supported_definition_kind_and_function_signature() {
    let elisp_form = r##"(progn
  (defun ample-regexps-test--documented-form
      (_form required &optional optional)
    "Construct a documented form from REQUIRED and OPTIONAL."
    `(seq ,required ,optional))
  (mapcar
   #'arx--form-make-docstring
   '((word word)
     (literal "a.b")
     (identifier (regexp "[[:alpha:]]+"))
     (documented
      (:func ample-regexps-test--documented-form))
     (undocumented
      (:func
       (lambda (_form &rest values)
         `(seq ,@values)))))))"##;
    let expect = expect![[
        r#"OK ("`word'\n    An alias for word." "`literal'\n    A regexp matching literal string: \"a.b\"." "`identifier'\n    An alias for (regexp \"[[:alpha:]]+\")." "`(documented required &optional optional)'\n    Construct a documented form from REQUIRED and OPTIONAL." "`(undocumented &rest values)'\n    Function without documentation.")"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn generated_documentation_helpers_preserve_exact_public_guidance() {
    let elisp_form = r##"(list
 (arx--make-macro-bindings-docstring "service-rx")
 (arx--make-macro-constituents-docstring "service-rx")
 (arx--make-macro-to-string-docstring "service-rx")
 (arx--make-macro-docstring
  "service-rx"
  (list
   "`service'\n    Match a service identifier."
   "`environment'\n    Match a deployment environment.")))"##;
    let expect = expect![[
        r#"OK ("List of bindings for `service-rx' and `service-rx-to-string' functions.\n\nSee `service-rx' for a human readable list of defined forms.\n\nSee parameter BINDINGS for function `rx-let' for more information\nabout format of elements of this list." "List of form definitions for `service-rx' and `service-rx-to-string' functions.\n\nSee `service-rx' for a human readable list of defined forms.\n\nSee variable `rx-constituents' for more information about format\nof elements of this list." "Parse and produce code for regular expression FORM.\n\nFORM is a regular expression in sexp form as supported by `service-rx'.\nNO-GROUP non-nil means don't put shy groups around the result." "Translate regular expressions REGEXPS in sexp form to a regexp string.\n\nSee macro `rx' for more documentation on REGEXPS parameter.\nThis macro additionally supports the following forms:\n\n`service'\n    Match a service identifier.\n\n`environment'\n    Match a deployment environment.\n\nUse function `service-rx-to-string' to do such a translation at run-time.")"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn arx_and_and_arx_or_cover_empty_single_sequence_and_alternative_inputs() {
    let elisp_form = r##"(mapcar
 (lambda (entry)
   (let ((function (car entry))
         (forms (cadr entry)))
     (condition-case error-data
         (list function forms
               'ok
               (funcall function forms))
       (error
        (list function forms
              'error
              error-data)))))
 '((arx-and ())
   (arx-and ("alpha"))
   (arx-and ("alpha" (seq "-" "beta")))
   (arx-and ("alpha" (or "beta" "gamma")))
   (arx-or ())
   (arx-or ("alpha"))
   (arx-or ("alpha" (seq "beta" "gamma")))
   (arx-or ("alpha" (or "beta" "gamma")))))"##;
    let expect = expect![[
        r#"OK ((arx-and nil ok "") (arx-and ("alpha") error (void-function rx-and)) (arx-and ("alpha" (seq "-" "beta")) error (void-function rx-and)) (arx-and ("alpha" (or "beta" "gamma")) error (void-function rx-and)) (arx-or nil ok "") (arx-or ("alpha") error (void-function rx-and)) (arx-or ("alpha" (seq "beta" "gamma")) error (void-function rx-or)) (arx-or ("alpha" (or "beta" "gamma")) error (void-function rx-or)))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn form_function_lookup_resolves_direct_and_chained_aliases_from_legacy_metadata() {
    let elisp_form = r##"(progn
  (defun ample-regexps-test--form
      (_form left right)
    `(seq ,left ,right))
  (put
   'manual-rx-constituents
   'arx-form-defs
   '((direct
      (:func ample-regexps-test--form))
     (alias direct)
     (second-alias alias)
     (literal "literal")
     (structural (seq "x" "y"))))
  (mapcar
   (lambda (symbol)
     (list
      symbol
      (arx--get-form-func "manual-rx" symbol)))
   '(direct alias second-alias
     literal structural missing)))"##;
    let expect = expect![
        "OK ((direct ample-regexps-test--form) (alias ample-regexps-test--form) (second-alias ample-regexps-test--form) (literal nil) (structural nil) (missing nil))"
    ];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn buffer_context_helpers_find_generated_arx_name_depth_and_custom_form_symbol() {
    let elisp_form = r##"(progn
  (define-arx query-rx
    '((field
       (:func
        (lambda (_form name value)
          `(seq ,name "=" ,value))))
      (alias field)))
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert
     "(query-rx (seq line-start (alias \"team\" \"runtime\") line-end))")
    (goto-char (point-min))
    (search-forward "\"runtime\"")
    (list
     (arx--name-and-depth)
     (arx--fnsym-in-current-sexp)
     (arx-documentation-function))))"##;
    let expect = expect![[r#"OK (("query-rx" . 3) (alias 2) nil)"#]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn include_if_macro_evaluates_condition_while_expanding_and_preserves_body_order() {
    let elisp_form = r##"(list
 (macroexpand-1
  '(arx--include-if t
     (setq first 1)
     (setq second 2)))
 (macroexpand-1
  '(arx--include-if nil
     (setq unreachable t)))
 (progn
   (setq ample-regexps-test--feature-flag t)
   (macroexpand-1
    '(arx--include-if
         ample-regexps-test--feature-flag
       (list
        'enabled
        ample-regexps-test--feature-flag)))))"##;
    let expect = expect![
        "OK ((progn (setq first 1) (setq second 2)) nil (progn (list 'enabled ample-regexps-test--feature-flag)))"
    ];
    assert_ample_regexps_parity(elisp_form, expect);
}
