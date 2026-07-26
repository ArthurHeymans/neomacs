use expect_test::expect;

use super::{assert_ac_html_parity, assert_ac_html_signal_parity};

#[test]
fn ac_html_define_data_provider_registers_exact_callbacks_and_is_idempotent() {
    let elisp_form = r##"(let ((ac-html-data-providers
                    '(existing-provider)))
               (ac-html-define-data-provider
                   'fixture-provider
                 :tag-func 'fixture-tags
                 :attr-func 'fixture-attrs
                 :attrv-func 'fixture-attrvs
                 :id-func 'fixture-ids
                 :class-func 'fixture-classes
                 :tag-doc-func 'fixture-tag-doc
                 :attr-doc-func 'fixture-attr-doc
                 :attrv-doc-func 'fixture-attrv-doc
                 :id-doc-func 'fixture-id-doc
                 :class-doc-func 'fixture-class-doc)
               (ac-html-define-data-provider
                   'fixture-provider
                 :tag-func 'replacement-tags)
               (list
                ac-html-data-providers
                (mapcar
                 (lambda (key)
                   (cons
                    key
                    (ac-html-query-data-provider
                     'fixture-provider key)))
                 '(:tag-func
                   :attr-func
                   :attrv-func
                   :id-func
                   :class-func
                   :tag-doc-func
                   :attr-doc-func
                   :attrv-doc-func
                   :id-doc-func
                   :class-doc-func))
                (ac-html-query-data-provider
                 'fixture-provider
                 :missing)))"##;
    let expect = expect![
        "OK ((fixture-provider existing-provider) ((:tag-func . replacement-tags) (:attr-func) (:attrv-func) (:id-func) (:class-func) (:tag-doc-func) (:attr-doc-func) (:attrv-doc-func) (:id-doc-func) (:class-doc-func)) nil)"
    ];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_define_data_provider_ignores_unknown_labels_but_consumes_their_values() {
    let elisp_form = r##"(let ((ac-html-data-providers nil))
               (ac-html-define-data-provider
                   'fixture-provider
                 :unknown 'discarded
                 :tag-func 'fixture-tags
                 :another-unknown 'also-discarded)
               (list
                ac-html-data-providers
                (get 'fixture-provider
                     :tag-func)
                (get 'fixture-provider
                     :unknown)
                (get 'fixture-provider
                     :another-unknown)))"##;
    let expect = expect!["OK ((fixture-provider) fixture-tags nil nil)"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_define_data_provider_treats_a_missing_property_value_as_nil() {
    let elisp_form = r##"(macroexpand
               '(ac-html-define-data-provider
                    'fixture-provider
                  :tag-func))"##;
    let expect = expect![[
        "OK (progn (add-to-list 'ac-html-data-providers #1='fixture-provider) (put #1# :tag-func nil) (put #1# :attr-func nil) (put #1# :attrv-func nil) (put #1# :id-func nil) (put #1# :class-func nil) (put #1# :tag-doc-func nil) (put #1# :attr-doc-func nil) (put #1# :attrv-doc-func nil) (put #1# :id-doc-func nil) (put #1# :class-doc-func nil))"
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_enable_data_provider_is_buffer_local_ordered_and_idempotent() {
    let elisp_form = r##"(let ((default-before
                    (default-value
                     'ac-html-enabled-data-providers)))
               (with-temp-buffer
                 (setq-local
                  ac-html-enabled-data-providers
                  '(base))
                 (list
                  (ac-html-enable-data-provider
                   'first)
                  (ac-html-enable-data-provider
                   'second)
                  (ac-html-enable-data-provider
                   'first)
                  ac-html-enabled-data-providers
                  (local-variable-p
                   'ac-html-enabled-data-providers)
                  (equal
                   default-before
                   (default-value
                    'ac-html-enabled-data-providers)))))"##;
    let expect = expect!["OK (#1=(first base) #2=(second . #1#) #2# #2# t t)"];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_define_ac_source_creates_only_requested_sources_and_exact_setup() {
    let elisp_form = r##"(progn
               (makunbound
                'ac-source-fixture-tag)
               (makunbound
                'ac-source-fixture-attr)
               (makunbound
                'ac-source-fixture-attrv)
               (makunbound
                'ac-source-fixture-id)
               (makunbound
                'ac-source-fixture-class)
               (ac-html-define-ac-source
                   "fixture"
                 :tag-prefix fixture-tag-prefix
                 :attrv-prefix "fixture-value-regexp"
                 :class-prefix fixture-class-prefix
                 :current-tag-func fixture-current-tag
                 :current-attr-func fixture-current-attr)
               (with-temp-buffer
                 (let ((setup-result
                        (ac-fixture-setup)))
                   (list
                    setup-result
                    ac-html-current-tag-function
                    ac-html-current-attr-function
                    (mapcar
                     (lambda (symbol)
                       (list
                        symbol
                        (boundp symbol)
                        (and
                         (boundp symbol)
                         (symbol-value symbol))))
                     '(ac-source-fixture-tag
                       ac-source-fixture-attr
                       ac-source-fixture-attrv
                       ac-source-fixture-id
                       ac-source-fixture-class))))))"##;
    let expect = expect![[
        r#"OK (fixture-current-attr fixture-current-tag fixture-current-attr ((ac-source-fixture-tag t ((candidates . ac-html-all-tag-candidates) (prefix . fixture-tag-prefix) (document . ac-html-tag-documentation) (symbol . "t"))) (ac-source-fixture-attr nil nil) (ac-source-fixture-attrv t ((candidates . ac-html-all-attrv-candidates) (prefix . "fixture-value-regexp") (document . ac-html-attrv-documentation) (symbol . "v"))) (ac-source-fixture-id nil nil) (ac-source-fixture-class t ((candidates . ac-html-all-class-candidates) (prefix . fixture-class-prefix) (document . ac-html-class-documentation) (symbol . "c")))))"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_provider_callback_failure_propagates_without_running_later_providers() {
    let elisp_form = r##"(let ((ac-html-enabled-data-providers
                    '(first-provider
                      second-provider))
                   calls)
               (put 'first-provider
                    :tag-func
                    (lambda ()
                      (push 'first calls)
                      (error "fixture provider failure")))
               (put 'second-provider
                    :tag-func
                    (lambda ()
                      (push 'second calls)
                      '("unreachable")))
               (unwind-protect
                   (ac-html-all-tag-candidates)
                 (put 'first-provider
                      :tag-func nil)
                 (put 'second-provider
                      :tag-func nil)))"##;
    let expect = expect![[r#"ERR (error "fixture provider failure")"#]];

    assert_ac_html_signal_parity(elisp_form, expect);
}
