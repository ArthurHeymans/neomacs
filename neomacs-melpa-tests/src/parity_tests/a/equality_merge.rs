use expect_test::expect;

use super::assert_a_parity;

#[test]
fn a_equal_compares_alists_and_hashes_by_recursive_key_value_content() {
    let elisp_form = r##"(let ((table
                    (a-hash-table
                     :outer
                     (a-list
                      :vector [1 2]
                      :nil nil)
                     :value 3)))
               (list
                (a-equal
                 '((:a . 1)
                   (:b . 2))
                 '((:b . 2)
                   (:a . 1)))
                (a-equal
                 table
                 '((:value . 3)
                   (:outer
                    (:nil)
                    (:vector 1 2))))
                (a-equal
                 table
                 '((:value . 3)
                   (:outer
                    (:nil)
                    (:vector 1 9))))))"##;
    let expect = expect!["OK (t t nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_equal_sequence_recursion_crosses_list_vector_and_string_types() {
    let elisp_form = r##"(list
              (a-equal
               '(1 2 3)
               [1 2 3])
              (a-equal
               '(97 98)
               "ab")
              (a-equal
               "λx"
               [?λ ?x])
              (a-equal nil [])
              (a-equal
               [1 [2 (3)]]
               '(1 (2 [3]))))"##;
    let expect = expect!["OK (t t t t t)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_equal_rejects_length_key_value_nested_and_atomic_mismatches() {
    let elisp_form = r##"(list
              (a-equal '(1 2)
                       [1 2 3])
              (a-equal
               '((:a . 1))
               '((:b . 1)))
              (a-equal
               '((:a . 1))
               '((:a . 2)))
              (a-equal
               '((:a . (1 2)))
               '((:a . [1 3])))
              (a-equal 'symbol
                       'other)
              (a-equal 1 1.0))"##;
    let expect = expect!["OK (nil nil nil nil nil nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_equal_duplicate_alists_compare_only_the_first_value_for_each_repeated_key() {
    let elisp_form = r##"(list
              (a-equal
               '((:a . 1)
                 (:a . 9))
               '((:a . 1)
                 (:a . 8)))
              (a-equal
               '((:a . 1)
                 (:a . 9))
               '((:a . 2)
                 (:a . 9)))
              (a-equal
               '((:a . 1)
                 (:a . 9))
               '((:a . 1))))"##;
    let expect = expect!["OK (t nil nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_equal_alias_returns_the_same_recursive_results_as_the_primary_function() {
    let elisp_form = r##"(let ((left
                    (a-list
                     :nested [1 2]))
                   (right
                    (a-hash-table
                     :nested '(1 2))))
               (list
                (a-equal left right)
                (a-equal? left right)
                (eq
                 (indirect-function
                  'a-equal?)
                 (indirect-function
                  'a-equal))))"##;
    let expect = expect!["OK (t t t)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_merge_is_right_biased_preserves_first_type_and_has_zero_and_one_input_identities() {
    let elisp_form = r##"(let* ((source
                      '((:a . 1)
                        (:b . 2)))
                     (result
                      (a-merge
                       source
                       '((:b . 20)
                         (:c . 3))
                       '((:a . 9)))))
               (list
                result
                source
                (a-merge)
                (eq source
                    (a-merge
                     source))))"##;
    let expect = expect![[r#"OK (((:c . 3) (:a . 9) (:b . 20)) ((:a . 1) (:b . 2)) nil t)"#]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_merge_with_passes_new_then_old_values_and_recombines_each_later_collection() {
    let elisp_form = r##"(let ((result
                    (a-merge-with
                     (lambda (new old)
                       (list new old))
                     '((:a . 10)
                       (:b . 2))
                     '((:a . 3)
                       (:c . 4))
                     '((:a . 8)))))
               (list
                result
                (a-get result :a)
                (a-get result :b)
                (a-get result :c)))"##;
    let expect = expect![[r#"OK (((:c . 4) (:a . #1=(8 (3 10))) (:b . 2)) #1# 2 4)"#]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_merge_and_merge_with_clone_hash_first_inputs_without_mutation() {
    let elisp_form = r##"(let* ((source
                      (a-hash-table
                       :a 1 :b 2))
                     (merged
                      (a-merge
                       source
                       '((:a . 5)
                         (:c . 3))))
                     (combined
                      (a-merge-with
                       #'+ source
                       '((:a . 5)
                         (:c . 3)))))
               (list
                (hash-table-p merged)
                (hash-table-p combined)
                (eq source merged)
                (a-get merged :a)
                (a-get combined :a)
                (a-get combined :c)
                (a-get source :a)
                (a-has-key source :c)))"##;
    let expect = expect!["OK (t t nil 5 6 3 1 nil)"];

    assert_a_parity(elisp_form, expect);
}
