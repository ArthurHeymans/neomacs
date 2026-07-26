use expect_test::expect;

use super::assert_ac_html_parity;

#[test]
fn ac_html_tag_candidates_concatenate_provider_results_in_enabled_order() {
    let elisp_form = r##"(let* ((ac-html-enabled-data-providers
                    '(first-provider
                      empty-provider
                      second-provider))
                   calls)
               (put 'first-provider
                    :tag-func
                    (lambda ()
                      (push 'first calls)
                      '("a" "shared")))
               (put 'second-provider
                    :tag-func
                    (lambda ()
                      (push 'second calls)
                      '("shared" "z")))
               (unwind-protect
                   (list
                    (ac-html-all-tag-candidates)
                    (nreverse calls))
                 (put 'first-provider :tag-func nil)
                 (put 'second-provider :tag-func nil)))"##;
    let expect = expect![[r#"OK (("a" "shared" "shared" "z") (first second))"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_attr_candidates_resolve_the_live_tag_once_per_provider() {
    let elisp_form = r##"(let* ((ac-html-enabled-data-providers
                    '(first-provider
                      missing-provider
                      second-provider))
                   calls
                   (tag-call 0)
                   (ac-html-current-tag-function
                    (lambda ()
                      (setq tag-call
                            (1+ tag-call))
                      (push
                       (list 'tag tag-call)
                       calls)
                      (format "tag-%d"
                              tag-call))))
               (put 'first-provider
                    :attr-func
                    (lambda (tag)
                      (push
                       (list 'first tag)
                       calls)
                      '("first-attr")))
               (put 'second-provider
                    :attr-func
                    (lambda (tag)
                      (push
                       (list 'second tag)
                       calls)
                      '("second-attr")))
               (unwind-protect
                   (list
                    (ac-html-all-attr-candidates)
                    (nreverse calls)
                    tag-call)
                 (put 'first-provider :attr-func nil)
                 (put 'second-provider :attr-func nil)))"##;
    let expect = expect![[
        r#"OK (("first-attr" "second-attr") ((tag 1) (first "tag-1") (tag 2) (tag 3) (second "tag-3")) 3)"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_attrv_candidates_append_class_candidates_after_provider_values() {
    let elisp_form = r##"(let* ((ac-html-enabled-data-providers
                    '(value-provider
                      class-provider))
                   calls
                   (ac-html-current-tag-function
                    (lambda ()
                      (push 'tag calls)
                      "div"))
                   (ac-html-current-attr-function
                    (lambda ()
                      (push 'attr calls)
                      "class")))
               (put 'value-provider
                    :attrv-func
                    (lambda (tag attr)
                      (push
                       (list 'values tag attr)
                       calls)
                      '("native")))
               (put 'class-provider
                    :class-func
                    (lambda ()
                      (push 'classes calls)
                      '("utility" "component")))
               (unwind-protect
                   (list
                    (ac-html-all-attrv-candidates)
                    (nreverse calls))
                 (put 'value-provider :attrv-func nil)
                 (put 'class-provider :class-func nil)))"##;
    let expect = expect![[
        r#"OK (("native" "utility" "component") (tag attr (values "div" "class") tag attr classes))"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_attrv_candidates_append_id_candidates_and_preserve_duplicates() {
    let elisp_form = r##"(let ((ac-html-enabled-data-providers
                    '(value-provider
                      id-provider))
                   (ac-html-current-tag-function
                    (lambda () "label"))
                   (ac-html-current-attr-function
                    (lambda () "id")))
               (put 'value-provider
                    :attrv-func
                    (lambda (_tag _attr)
                      '("shared" "native")))
               (put 'id-provider
                    :id-func
                    (lambda ()
                      '("shared" "fixture-id")))
               (unwind-protect
                   (ac-html-all-attrv-candidates)
                 (put 'value-provider :attrv-func nil)
                 (put 'id-provider :id-func nil)))"##;
    let expect = expect![[r#"OK ("shared" "native" "shared" "fixture-id")"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_attrv_candidates_do_not_query_class_or_id_for_other_attributes() {
    let elisp_form = r##"(let ((ac-html-enabled-data-providers
                    '(fixture-provider))
                   calls
                   (ac-html-current-tag-function
                    (lambda () "input"))
                   (ac-html-current-attr-function
                    (lambda () "type")))
               (put 'fixture-provider
                    :attrv-func
                    (lambda (tag attr)
                      (push
                       (list 'attrv tag attr)
                       calls)
                      '("text" "email")))
               (put 'fixture-provider
                    :class-func
                    (lambda ()
                      (push 'unexpected-class calls)))
               (put 'fixture-provider
                    :id-func
                    (lambda ()
                      (push 'unexpected-id calls)))
               (unwind-protect
                   (list
                    (ac-html-all-attrv-candidates)
                    (nreverse calls))
                 (put 'fixture-provider :attrv-func nil)
                 (put 'fixture-provider :class-func nil)
                 (put 'fixture-provider :id-func nil)))"##;
    let expect = expect![[r#"OK (("text" "email") ((attrv "input" "type")))"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_id_and_class_candidates_skip_missing_callbacks_and_keep_order() {
    let elisp_form = r##"(let ((ac-html-enabled-data-providers
                    '(first-provider
                      missing-provider
                      second-provider)))
               (put 'first-provider
                    :id-func
                    (lambda ()
                      '("first-id")))
               (put 'second-provider
                    :id-func
                    (lambda ()
                      '("second-id")))
               (put 'first-provider
                    :class-func
                    (lambda ()
                      '("first-class")))
               (put 'second-provider
                    :class-func
                    (lambda ()
                      '("second-class")))
               (unwind-protect
                   (list
                    (ac-html-all-id-candidates)
                    (ac-html-all-class-candidates))
                 (dolist
                     (provider
                      '(first-provider
                        second-provider))
                   (put provider :id-func nil)
                   (put provider :class-func nil))))"##;
    let expect = expect![[r#"OK (("first-id" "second-id") ("first-class" "second-class"))"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_candidate_queries_return_nil_with_no_enabled_providers() {
    let elisp_form = r##"(let ((ac-html-enabled-data-providers nil)
                   (ac-html-current-tag-function
                    (lambda ()
                      (error "unexpected tag query")))
                   (ac-html-current-attr-function
                    (lambda ()
                      (error "unexpected attr query"))))
               (list
                (ac-html-all-tag-candidates)
                (ac-html-all-attr-candidates)
                (ac-html-all-attrv-candidates)
                (ac-html-all-id-candidates)
                (ac-html-all-class-candidates)))"##;
    let expect = expect!["OK (nil nil nil nil nil)"];

    assert_ac_html_parity(elisp_form, expect);
}
