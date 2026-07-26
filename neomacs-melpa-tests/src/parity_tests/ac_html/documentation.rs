use expect_test::expect;

use super::assert_ac_html_parity;

#[test]
fn ac_html_tag_documentation_returns_the_first_non_nil_provider_result() {
    let elisp_form = r##"(let* ((ac-html-enabled-data-providers
                    '(nil-provider
                      winning-provider
                      unreachable-provider))
                   calls)
               (put 'nil-provider
                    :tag-doc-func
                    (lambda (tag)
                      (push
                       (list 'nil tag)
                       calls)
                      nil))
               (put 'winning-provider
                    :tag-doc-func
                    (lambda (tag)
                      (push
                       (list 'winning tag)
                       calls)
                      "winning documentation"))
               (put 'unreachable-provider
                    :tag-doc-func
                    (lambda (tag)
                      (push
                       (list 'unreachable tag)
                       calls)
                      "wrong"))
               (unwind-protect
                   (list
                    (ac-html-tag-documentation
                     "article")
                    (nreverse calls))
                 (dolist
                     (provider
                      '(nil-provider
                        winning-provider
                        unreachable-provider))
                   (put provider :tag-doc-func nil))))"##;
    let expect = expect![[r#"OK ("winning documentation" ((nil "article") (winning "article")))"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_attr_documentation_queries_current_tag_only_for_documenting_providers() {
    let elisp_form = r##"(let* ((ac-html-enabled-data-providers
                    '(missing-provider
                      nil-provider
                      winning-provider))
                   calls
                   (tag-count 0)
                   (ac-html-current-tag-function
                    (lambda ()
                      (setq tag-count
                            (1+ tag-count))
                      (format "tag-%d"
                              tag-count))))
               (put 'nil-provider
                    :attr-doc-func
                    (lambda (tag attr)
                      (push
                       (list 'nil tag attr)
                       calls)
                      nil))
               (put 'winning-provider
                    :attr-doc-func
                    (lambda (tag attr)
                      (push
                       (list 'winning tag attr)
                       calls)
                      "attribute documentation"))
               (unwind-protect
                   (list
                    (ac-html-attr-documentation
                     "href")
                    tag-count
                    (nreverse calls))
                 (put 'nil-provider :attr-doc-func nil)
                 (put 'winning-provider :attr-doc-func nil)))"##;
    let expect = expect![[
        r#"OK ("attribute documentation" 2 ((nil "tag-1" "href") (winning "tag-2" "href")))"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_attrv_documentation_prefers_direct_provider_over_class_fallback() {
    let elisp_form = r##"(let* ((ac-html-enabled-data-providers
                    '(fixture-provider))
                   calls
                   (ac-html-current-tag-function
                    (lambda () "div"))
                   (ac-html-current-attr-function
                    (lambda () "class")))
               (put 'fixture-provider
                    :attrv-doc-func
                    (lambda (tag attr value)
                      (push
                       (list 'direct tag attr value)
                       calls)
                      "direct documentation"))
               (put 'fixture-provider
                    :class-doc-func
                    (lambda (class)
                      (push
                       (list 'class class)
                       calls)
                      "fallback documentation"))
               (unwind-protect
                   (list
                    (ac-html-attrv-documentation
                     "button")
                    (nreverse calls))
                 (put 'fixture-provider :attrv-doc-func nil)
                 (put 'fixture-provider :class-doc-func nil)))"##;
    let expect = expect![[r#"OK ("direct documentation" ((direct "div" "class" "button")))"#]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_attrv_documentation_uses_class_and_id_fallbacks_after_direct_misses() {
    let elisp_form = r##"(let* ((ac-html-enabled-data-providers
                    '(fixture-provider))
                   (attribute "class")
                   calls
                   (ac-html-current-tag-function
                    (lambda () "div"))
                   (ac-html-current-attr-function
                    (lambda () attribute)))
               (put 'fixture-provider
                    :attrv-doc-func
                    (lambda (tag attr value)
                      (push
                       (list 'direct tag attr value)
                       calls)
                      nil))
               (put 'fixture-provider
                    :class-doc-func
                    (lambda (class)
                      (push
                       (list 'class class)
                       calls)
                      "class documentation"))
               (put 'fixture-provider
                    :id-doc-func
                    (lambda (id)
                      (push
                       (list 'id id)
                       calls)
                      "id documentation"))
               (unwind-protect
                   (let ((class-result
                          (ac-html-attrv-documentation
                           "utility")))
                     (setq attribute "id")
                     (let ((id-result
                            (ac-html-attrv-documentation
                             "hero")))
                       (list
                        class-result
                        id-result
                        (nreverse calls))))
                 (put 'fixture-provider :attrv-doc-func nil)
                 (put 'fixture-provider :class-doc-func nil)
                 (put 'fixture-provider :id-doc-func nil)))"##;
    let expect = expect![[
        r#"OK ("class documentation" "id documentation" ((direct "div" "class" "utility") (class "utility") (direct "div" "id" "hero") (id "hero")))"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_id_and_class_documentation_return_nil_after_all_providers_miss() {
    let elisp_form = r##"(let ((ac-html-enabled-data-providers
                    '(missing-provider
                      nil-provider))
                   calls)
               (put 'nil-provider
                    :id-doc-func
                    (lambda (id)
                      (push
                       (list 'id id)
                       calls)
                      nil))
               (put 'nil-provider
                    :class-doc-func
                    (lambda (class)
                      (push
                       (list 'class class)
                       calls)
                      nil))
               (unwind-protect
                   (list
                    (ac-html-id-documentation
                     "hero")
                    (ac-html-class-documentation
                     "utility")
                    (nreverse calls))
                 (put 'nil-provider :id-doc-func nil)
                 (put 'nil-provider :class-doc-func nil)))"##;
    let expect = expect![[r#"OK (nil nil ((id "hero") (class "utility")))"#]];

    assert_ac_html_parity(elisp_form, expect);
}
