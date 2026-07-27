use expect_test::expect;

use super::{assert_ample_regexps_parity, assert_ample_regexps_signal_parity};

#[test]
fn literal_regexp_symbol_and_nested_aliases_build_a_real_configuration_parser() {
    let elisp_form = r##"(progn
  (define-arx config-rx
    '((key (regexp "[[:alpha:]_][[:alnum:]_-]*"))
      (ws (* blank))
      (assignment
       (seq line-start ws
            (group key) ws "=" ws
            (group (* nonl)) line-end))))
  (let* ((regexp (config-rx assignment))
         (line "  release_channel = stable-2026 ")
         (matched (string-match regexp line)))
    (list
     regexp
     matched
     (and matched (match-string 1 line))
     (and matched (match-string 2 line))
     (string-match-p regexp "9invalid = value")
     (config-rx-to-string 'assignment t))))"##;
    let expect = expect![[
        r#"OK ("^[[:blank:]]*\\([[:alpha:]_][[:alnum:]_-]*\\)[[:blank:]]*=[[:blank:]]*\\(.*\\)$" 0 "release_channel" "stable-2026 " nil "^[[:blank:]]*\\([[:alpha:]_][[:alnum:]_-]*\\)[[:blank:]]*=[[:blank:]]*\\(.*\\)$")"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn literal_aliases_quote_metacharacters_while_regexp_aliases_remain_active() {
    let elisp_form = r##"(progn
  (define-arx delimiter-rx
    '((literal-boundary "^$.[]")
      (empty-line (regexp "^$"))
      (either-boundary
       (or literal-boundary empty-line))))
  (let ((literal (delimiter-rx literal-boundary))
        (raw (delimiter-rx empty-line))
        (either (delimiter-rx either-boundary)))
    (list
     literal raw either
     (string-match-p literal "^$.[]")
     (string-match-p literal "anything")
     (string-match-p raw "")
     (string-match-p raw "^$ text")
     (mapcar
      (lambda (text) (and (string-match-p either text) t))
      '("" "^$.[]" "not-a-boundary")))))"##;
    let expect =
        expect![[r#"OK ("\\^\\$\\.\\[]" "^$" "\\^\\$\\.\\[]\\|^$" 0 nil 0 nil (t t nil))"#]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn custom_form_returning_raw_regexp_parses_delimited_identifiers_in_context() {
    let elisp_form = r##"(progn
  (define-arx tagged-rx
    '((tag
       (:func
        (lambda (_form name)
          (format "<%s>" (regexp-quote name)))
        :min-args 1
        :max-args 1))
      (entry
       (seq line-start
            (group (tag "service.prod"))
            blank
            (group (+ digit))
            line-end))))
  (let* ((regexp (tagged-rx entry))
         (text "<service.prod> 2048")
         (matched (string-match regexp text)))
    (list
     regexp matched
     (and matched (match-string 1 text))
     (and matched (match-string 2 text))
     (string-match-p regexp "<serviceXprod> 2048")
     (tagged-rx-to-string '(tag "api[v2]") t))))"##;
    let expect = expect![[
        r#"OK ("^\\(<service\\.prod>\\)[[:blank:]]\\([[:digit:]]+\\)$" 0 "<service.prod>" "2048" nil "<api\\[v2]>")"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn custom_form_returning_rx_form_expands_repeated_alternatives_and_matches_records() {
    let elisp_form = r##"(progn
  (define-arx record-rx
    '((repeat-fields
       (:func
        (lambda (_form count field separator)
          `(seq
            ,field
            ,@(apply
               #'append
               (make-list
                (1- count)
                (list separator field)))))
        :min-args 3
        :max-args 3))
      (identifier (regexp "[[:alpha:]][[:alnum:]_]*"))
      (record
       (seq line-start
            (repeat-fields 3 identifier ",")
            line-end))))
  (let ((regexp (record-rx record)))
    (list
     regexp
     (mapcar
      (lambda (text) (and (string-match-p regexp text) t))
      '("alpha,beta,gamma"
        "alpha,beta"
        "alpha,2beta,gamma"
        "alpha,beta,gamma,delta")))))"##;
    let expect = expect![[
        r#"OK ("^\\(?:[[:alpha:]][[:alnum:]_]*\\),\\(?:[[:alpha:]][[:alnum:]_]*\\),\\(?:[[:alpha:]][[:alnum:]_]*\\)$" (t nil nil nil))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn recursive_custom_form_builds_bounded_optional_csv_and_captures_each_field() {
    let elisp_form = r##"(progn
  (defun ample-regexps-test--optional-fields
      (_form count field &optional accumulator)
    (cond
     ((<= count 0) accumulator)
     ((null accumulator)
      (list _form
            (1- count)
            field
            (list 'group-n count field)))
     (t
      (list _form
            (1- count)
            field
            (list 'group-n count field
                  (list 'opt "," accumulator))))))
  (define-arx csv-rx
    '((optional-fields
       (:func
        (lambda (&rest arguments)
          (apply
           #'ample-regexps-test--optional-fields
           arguments))))
      (field (regexp "[[:alpha:]]+"))
      (row
       (seq line-start
            (optional-fields 3 field)
            line-end))))
  (let ((regexp (csv-rx row)))
    (mapcar
     (lambda (text)
       (let ((matched (string-match regexp text)))
         (list
          text
          (and matched t)
          (and matched (match-string 1 text))
          (and matched (match-string 2 text))
          (and matched (match-string 3 text)))))
     '("one" "one,two" "one,two,three"
       "one,two,three,four" ""))))"##;
    let expect = expect![[
        r#"OK (("one" t "one" nil nil) ("one,two" t "one,two" "two" nil) ("one,two,three" t "one,two,three" "two,three" "three") ("one,two,three,four" nil nil nil nil) ("" nil nil nil nil))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn named_function_custom_form_exposes_current_modern_rx_invocation_behavior() {
    let elisp_form = r##"(progn
  (defun ample-regexps-test--named-form
      (_form left right)
    `(seq ,left "=" ,right))
  (define-arx named-rx
    '((pair
       (:func ample-regexps-test--named-form))))
  (named-rx-to-string '(pair "left" "right") t))"##;
    let expect = expect!["ERR (void-variable ample-regexps-test--named-form)"];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn predicate_metadata_exposes_current_modern_rx_invocation_behavior() {
    let elisp_form = r##"(progn
  (define-arx predicate-rx
    '((text-only
       (:func
        (lambda (_form _argument)
          "accepted")
        :min-args 1
        :max-args 1
        :predicate stringp))))
  (predicate-rx-to-string '(text-only "text") t))"##;
    let expect = expect!["ERR (void-variable stringp)"];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn generated_macro_and_runtime_function_agree_across_grouping_modes() {
    let elisp_form = r##"(progn
  (define-arx route-rx
    '((method (or "GET" "POST" "PATCH"))
      (segment (regexp "[[:alnum:]_-]+"))
      (route
       (seq method blank
            "/" segment
            (opt "/" segment)))))
  (let ((macro-regexp (route-rx route))
        (runtime-grouped
         (route-rx-to-string 'route))
        (runtime-ungrouped
         (route-rx-to-string 'route t)))
    (list
     macro-regexp runtime-grouped runtime-ungrouped
     (equal macro-regexp runtime-ungrouped)
     (mapcar
      (lambda (line)
        (list
         line
         (and (string-match-p
               (concat "\\`" macro-regexp "\\'") line)
              t)))
      '("GET /users"
        "PATCH /users/42"
        "DELETE /users"
        "GET /users/42/extra")))))"##;
    let expect = expect![[
        r#"OK ("\\(?:GET\\|P\\(?:ATCH\\|OST\\)\\)[[:blank:]]/\\(?:[[:alnum:]_-]+\\)\\(?:/\\(?:[[:alnum:]_-]+\\)\\)?" "\\(?:\\(?:GET\\|P\\(?:ATCH\\|OST\\)\\)[[:blank:]]/\\(?:[[:alnum:]_-]+\\)\\(?:/\\(?:[[:alnum:]_-]+\\)\\)?\\)" "\\(?:GET\\|P\\(?:ATCH\\|OST\\)\\)[[:blank:]]/\\(?:[[:alnum:]_-]+\\)\\(?:/\\(?:[[:alnum:]_-]+\\)\\)?" t (("GET /users" t) ("PATCH /users/42" t) ("DELETE /users" nil) ("GET /users/42/extra" nil)))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn conditional_definitions_drop_nil_entries_and_redefinition_refreshes_bindings() {
    let elisp_form = r##"(progn
  (define-arx state-rx
    `((queued "queued")
      ,(when t '(running "running"))
      ,(when nil '(removed "removed"))))
  (let ((before
         (list
          (state-rx-to-string 'queued t)
          (state-rx-to-string 'running t)
          (length state-rx-bindings))))
    (eval
     '(define-arx state-rx
        '((ready "ready")
          (failed "failed"))))
    (list
     before
     (state-rx-to-string 'ready t)
     (state-rx-to-string 'failed t)
     (length state-rx-bindings)
     (get 'state-rx 'arx-form-defs))))"##;
    let expect = expect![[
        r#"OK (("queued" "running" 2) "ready" "failed" 2 ((ready "ready") (failed "failed")))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn redefining_an_arx_removes_obsolete_forms_from_runtime_lookup() {
    let elisp_form = r##"(progn
  (define-arx state-rx '((old-state "old")))
  (eval '(define-arx state-rx '((new-state "new"))))
  (state-rx-to-string 'old-state t))"##;
    let expect = expect![[r#"ERR (error "Unknown rx symbol ‘old-state’")"#]];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn custom_form_enforces_single_required_argument_with_specific_error() {
    let elisp_form = r##"(progn
  (define-arx capture-rx
    '((capture
       (:func
        (lambda (_form value) `(group ,value))))))
  (capture-rx-to-string '(capture) t))"##;
    let expect = expect![[r#"ERR (error "rx form ‘capture’ requires at least 1 arg")"#]];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn custom_form_enforces_multiple_required_arguments_with_plural_error() {
    let elisp_form = r##"(progn
  (define-arx pair-rx
    '((pair
       (:func
        (lambda (_form left right)
          `(seq ,left "=" ,right))))))
  (pair-rx-to-string '(pair "left") t))"##;
    let expect = expect![[r#"ERR (error "rx form ‘pair’ requires at least 2 args")"#]];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn explicit_max_args_overrides_rest_arity_and_rejects_excess_arguments() {
    let elisp_form = r##"(progn
  (define-arx numbered-rx
    '((numbered
       (:func
        (lambda (_form index &rest pieces)
          (format "\\(?%d:%s\\)"
                  index
                  (mapconcat #'identity pieces "")))
        :max-args 3))))
  (numbered-rx-to-string
   '(numbered 1 "alpha" "beta" "gamma")
   t))"##;
    let expect = expect![[r#"ERR (error "rx form ‘numbered’ accepts at most 3 args")"#]];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn explicit_min_and_max_bounds_accept_every_in_range_custom_form_shape() {
    let elisp_form = r##"(progn
  (define-arx joined-rx
    '((joined
       (:func
        (lambda (_form first &optional second third)
          `(seq ,first
                ,@(delq nil
                        (list
                         (and second (list 'seq ":" second))
                         (and third (list 'seq ":" third))))))
        :min-args 1
        :max-args 3))))
  (list
   (joined-rx-to-string '(joined "one") t)
   (joined-rx-to-string '(joined "one" "two") t)
   (joined-rx-to-string '(joined "one" "two" "three") t)
   (mapcar
    (lambda (text)
      (and
       (string-match-p
        (concat
         "\\`"
         (joined-rx-to-string
          '(joined "one" "two" "three")
          t)
         "\\'")
        text)
       t))
    '("one:two:three" "one:two" "one::three"))))"##;
    let expect = expect![[r#"OK ("one" "one:two" "one:two:three" (t nil nil))"#]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn malformed_definition_rejects_non_list_form_entry() {
    let elisp_form = r##"(arx--form-to-rx-binding 'not-a-list)"##;
    let expect = expect![[r#"ERR (error "Form is not a list: not-a-list")"#]];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn malformed_definition_rejects_non_function_func_property() {
    let elisp_form = r##"(define-arx--fn
 'broken-rx
 '((broken
    (:func ample-regexps--missing-function))))"##;
    let expect = expect![[r#"ERR (error "Not a function: ample-regexps--missing-function")"#]];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn malformed_definition_rejects_unsupported_definition_value() {
    let elisp_form = r##"(arx--form-to-rx-binding '(broken 42))"##;
    let expect = expect![[r#"ERR (error "Incorrect arx-form: (broken 42)")"#]];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn generated_macro_accepts_an_empty_regexp_invocation_as_empty_pattern() {
    let elisp_form = r##"(progn
  (define-arx empty-call-rx '())
  (eval '(empty-call-rx)))"##;
    let expect = expect![[r#"OK """#]];
    assert_ample_regexps_parity(elisp_form, expect);
}
