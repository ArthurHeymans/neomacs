use expect_test::expect;

use super::assert_async1_parity;

#[test]
fn async1_plist_get_ports_the_complete_upstream_value_and_default_matrix() {
    let elisp_form = r##"(list
         (async1-plist-get
          '(:foo (function bar))
          :foo)
         (async1-plist-get
          '(:foo nil :bar 1)
          :foo)
         (async1-plist-get
          '(:foo 42 :bar 1)
          :foo)
         (async1-plist-get
          '(:foo 1 :bar)
          :bar)
         (async1-plist-get
          '(:foo)
          :foo)
         (async1-plist-get
          '(:foo 42 :foo 99)
          :foo)
         (async1-plist-get
          '(:foo (bar baz))
          :foo)
         (async1-plist-get
          '(:foo 123)
          :bar
          777)
         (async1-plist-get
          '(:foo 1 :bar nil :zaza nil)
          :zaza)
         (async1-plist-get
          '(:foo 1 :bar nil :zaza)
          :zaza)
         (async1-plist-get
          '(:zaza :foo 1 :bar nil)
          :zaza))"##;
    let expect = expect!["OK (bar nil 42 nil nil 42 (bar baz) 777 nil nil nil)"];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_plist_get_distinguishes_absent_keys_from_present_nil_and_keyword_values() {
    let elisp_form = r##"(list
         (async1-plist-get
          '(:present nil)
          :present
          :fallback)
         (async1-plist-get
          '(:present nil)
          :absent
          :fallback)
         (async1-plist-get
          '(:present :next :next 3)
          :present
          :fallback)
         (async1-plist-get
          '(:present :)
          :present
          :fallback)
         (async1-plist-get
          '(:present ordinary-symbol)
          :present
          :fallback))"##;
    let expect = expect!["OK (nil :fallback nil : ordinary-symbol)"];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_plist_get_unwraps_quoted_symbols_but_preserves_other_callable_values() {
    let elisp_form = r##"(let* ((closure
                  (lambda (value)
                    (+ value 10)))
                 (quoted
                  (async1-plist-get
                   '(:handler
                     (function named-handler))
                   :handler))
                 (live
                  (async1-plist-get
                   (list :handler closure)
                   :handler)))
         (list
          quoted
          (eq quoted
              'named-handler)
          (functionp live)
          (eq live closure)
          (funcall live 5)))"##;
    let expect = expect!["OK (named-handler t t t 15)"];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_plist_get_accepts_non_keyword_keys_and_returns_the_first_duplicate() {
    let elisp_form = r##"(list
         (async1-plist-get
          '(name "first" name "second" count 3)
          'name)
         (async1-plist-get
          '(name "first" name "second" count 3)
          'count)
         (async1-plist-get
          '(name "first")
          'missing
          "fallback")
         (async1-plist-get
          '(nil "nil-key" :x 1)
          nil))"##;
    let expect = expect![[r#"OK ("first" 3 "fallback" "nil-key")"#]];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_plist_remove_handles_beginning_middle_end_and_absent_keys() {
    let elisp_form = r##"(list
         (async1-plist-remove
          '(:target 1 :a 2 :b 3)
          :target)
         (async1-plist-remove
          '(:a 1 :target 2 :b 3)
          :target)
         (async1-plist-remove
          '(:a 1 :b 2 :target 3)
          :target)
         (async1-plist-remove
          '(:a 1 :b 2)
          :target)
         (async1-plist-remove nil
                              :target))"##;
    let expect = expect!["OK ((:a 2 :b 3) (:a 1 :b 3) (:a 1 :b 2) (:a 1 :b 2) nil)"];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_plist_remove_duplicate_keys_and_equal_values_follow_delq_semantics() {
    let elisp_form = r##"(list
         (async1-plist-remove
          '(:a 1
            :target 2
            :b 2
            :target 3
            :c 4)
          :target)
         (async1-plist-remove
          '(:target "same"
            :a "same"
            :target "other"
            :b "same")
          :target)
         (async1-plist-remove
          '(:target x
            :a x
            :b y)
          :target))"##;
    let expect = expect![[r#"OK ((:a 1 :b 3 :c 4) (:a "same" "other" :b "same") (:a :b y))"#]];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_plist_remove_nil_value_removes_every_nil_cell_from_the_copy() {
    let elisp_form = r##"(let ((input
                '(:a nil
                  :target nil
                  :b nil
                  :c 3)))
         (list
          (async1-plist-remove input
                               :target)
          input))"##;
    let expect = expect!["OK ((:a :b :c 3) (:a nil :target nil :b nil :c 3))"];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_plist_remove_preserves_input_and_reuses_identity_only_when_key_is_absent() {
    let elisp_form = r##"(let* ((input
                  (list
                   :a
                   (list "nested")
                   :target
                   2
                   :b
                   3))
                 (present
                  (async1-plist-remove
                   input
                   :target))
                 (absent
                  (async1-plist-remove
                   input
                   :missing)))
         (list
          input
          present
          absent
          (eq input present)
          (eq input absent)
          (eq
           (plist-get input :a)
           (plist-get present :a))))"##;
    let expect =
        expect![[r#"OK (#2=(:a #1=("nested") :target 2 :b 3) (:a #1# :b 3) #2# nil t t)"#]];

    assert_async1_parity(elisp_form, expect);
}

#[test]
fn async1_plist_helpers_report_malformed_non_list_inputs_without_mutation() {
    let elisp_form = r##"(list
         (async1-test-error
          (lambda ()
            (async1-plist-get
             '(:a 1 . tail)
             :missing)))
         (async1-test-error
          (lambda ()
            (async1-plist-remove
             '(:a 1 . tail)
             :target)))
         (async1-test-error
          (lambda ()
            (async1-plist-get
             [:a 1]
             :a)))
         (async1-test-error
          (lambda ()
            (async1-plist-remove
             [:target 1]
             :target))))"##;
    let expect = expect![
        "OK ((:error wrong-type-argument (listp (:a 1 . tail))) (:error wrong-type-argument (listp (:a 1 . tail))) (:error wrong-type-argument (listp [:a 1])) (:error wrong-type-argument (listp [:target 1])))"
    ];

    assert_async1_parity(elisp_form, expect);
}
