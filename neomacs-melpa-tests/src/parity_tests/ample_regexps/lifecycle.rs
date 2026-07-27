use expect_test::expect;

use super::{assert_ample_regexps_parity, assert_ample_regexps_signal_parity};

#[test]
fn arx_minor_mode_adds_and_removes_buffer_local_eldoc_advice_without_leaking() {
    let elisp_form = r##"(let ((global-value
       (and (boundp 'eldoc-documentation-function)
            eldoc-documentation-function)))
  (with-temp-buffer
    (emacs-lisp-mode)
    (let ((before
           (list
            arx-minor-mode
            (local-variable-p
             'eldoc-documentation-function)
            eldoc-documentation-function)))
      (arx-minor-mode 1)
      (let ((enabled
             (list
              arx-minor-mode
              (local-variable-p
               'eldoc-documentation-function)
              eldoc-documentation-function
              (memq
               'arx-documentation-function
               (if (listp eldoc-documentation-function)
                   eldoc-documentation-function
                 (list eldoc-documentation-function))))))
        (arx-minor-mode -1)
        (list
         global-value before enabled
         (list
          arx-minor-mode
          (local-variable-p
           'eldoc-documentation-function)
          eldoc-documentation-function))))))"##;
    let expect = expect![[
        r#"OK (eldoc-documentation-default (nil nil eldoc-documentation-default) (t t #[128 "����\2\"��\13\0����\2\"��" [arx-documentation-function #[128 "������!\2\"��" [eldoc-documentation-function apply default-value] 4 advice--forward] :before-until nil apply] 4 advice] nil) (nil nil eldoc-documentation-default))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn independent_generated_arx_families_do_not_leak_aliases_or_runtime_bindings() {
    let elisp_form = r##"(progn
  (define-arx request-rx
    '((verb (or "GET" "POST"))
      (path (regexp "/[[:alnum:]_/-]+"))
      (request (seq verb blank path))))
  (define-arx metric-rx
    '((name
       (regexp "[[:alpha:]_][[:alnum:]_]*"))
      (value (+ digit))
      (metric (seq name "=" value))))
  (list
   (request-rx request)
   (metric-rx metric)
   (request-rx-to-string 'verb t)
   (metric-rx-to-string 'name t)
   (get 'request-rx 'arx-form-defs)
   (get 'metric-rx 'arx-form-defs)
   (condition-case error-data
       (request-rx-to-string 'metric t)
     (error (list 'error error-data)))
   (condition-case error-data
       (metric-rx-to-string 'request t)
     (error (list 'error error-data)))))"##;
    let expect = expect![[
        r#"OK ("\\(?:\\(?:GE\\|POS\\)T\\)[[:blank:]]\\(?:/[[:alnum:]_/-]+\\)" "\\(?:[[:alpha:]_][[:alnum:]_]*\\)=[[:digit:]]+" "\\(?:\\(?:GE\\|POS\\)T\\)" "[[:alpha:]_][[:alnum:]_]*" ((verb (or "GET" "POST")) (path (regexp "/[[:alnum:]_/-]+")) (request (seq verb blank path))) ((name (regexp "[[:alpha:]_][[:alnum:]_]*")) (value (+ digit)) (metric (seq name "=" value))) (error (error "Unknown rx symbol ‘metric’")) (error (error "Unknown rx symbol ‘request’")))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn compiled_macro_use_keeps_old_constant_while_runtime_function_observes_redefinition() {
    let elisp_form = r##"(progn
  (define-arx release-rx
    '((channel "stable")
      (record (seq "channel=" channel))))
  (let* ((expanded-before
          (macroexpand '(release-rx record)))
         (compiled-before
          (byte-compile
           '(lambda () (release-rx record))))
         (runtime-before
          (release-rx-to-string 'record t)))
    (eval
     '(define-arx release-rx
        '((channel "nightly")
          (record (seq "channel=" channel)))))
    (list
     expanded-before
     (funcall compiled-before)
     runtime-before
     (macroexpand '(release-rx record))
     (release-rx-to-string 'record t)
     (string-match-p
      (funcall compiled-before)
      "channel=stable")
     (string-match-p
      (release-rx-to-string 'record t)
      "channel=nightly"))))"##;
    let expect = expect![[
        r#"OK ((progn "channel=stable") "channel=stable" "channel=stable" (progn "channel=nightly") "channel=nightly" 0 0)"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn repeatedly_defining_same_arx_refreshes_docs_properties_and_behavior_in_place() {
    let elisp_form = r##"(progn
  (define-arx lifecycle-rx
    '((first "one")))
  (let ((first-function
         (symbol-function 'lifecycle-rx-to-string))
        (first-macro
         (symbol-function 'lifecycle-rx)))
    (eval
     '(define-arx lifecycle-rx
        '((second "two")
          (third (regexp "three+")))))
    (list
     (eq first-function
         (symbol-function 'lifecycle-rx-to-string))
     (eq first-macro
         (symbol-function 'lifecycle-rx))
     lifecycle-rx-bindings
     (get 'lifecycle-rx 'arx-form-defs)
     (documentation 'lifecycle-rx)
     (lifecycle-rx-to-string 'second t)
     (lifecycle-rx-to-string 'third t))))"##;
    let expect = expect![[
        r#"OK (nil nil ((second "two") (third #1=(regexp "three+"))) ((second "two") (third #1#)) "Translate regular expressions REGEXPS in sexp form to a regexp string.\n\nSee macro ‘rx’ for more documentation on REGEXPS parameter.\nThis macro additionally supports the following forms:\n\n‘second’\n    A regexp matching literal string: \"two\".\n\n‘third’\n    An alias for (regexp \"three+\").\n\nUse function ‘lifecycle-rx-to-string’ to do such a translation at run-time." "two" "three+")"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn arx_builder_uses_generated_definition_name_and_exposes_current_modern_rx_result() {
    let elisp_form = r##"(progn
  (define-arx builder-rx
    '((word (regexp "[[:alpha:]]+"))
      (assignment (seq word "=" word))))
  (arx-builder "builder-rx"))"##;
    let expect = expect!["ERR (void-variable builder-rx-constituents)"];
    assert_ample_regexps_signal_parity(elisp_form, expect);
}

#[test]
fn source_reload_keeps_package_feature_and_generated_user_definitions_operational() {
    let elisp_form = r##"(progn
  (define-arx persistent-rx
    '((token (regexp "[[:upper:]]+"))
      (record (seq "ID:" token))))
  (let ((before
         (list
          (persistent-rx record)
          (persistent-rx-to-string 'record t)
          (get 'persistent-rx 'arx-form-defs))))
    (load "ample-regexps" nil t)
    (list
     before
     (featurep 'ample-regexps)
     (persistent-rx record)
     (persistent-rx-to-string 'record t)
     (get 'persistent-rx 'arx-form-defs))))"##;
    let expect = expect![[
        r#"OK (("ID:\\(?:[[:upper:]]+\\)" "ID:\\(?:[[:upper:]]+\\)" #1=((token (regexp "[[:upper:]]+")) (record (seq "ID:" token)))) t "ID:\\(?:[[:upper:]]+\\)" "ID:\\(?:[[:upper:]]+\\)" #1#)"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}
