use expect_test::expect;

use super::{assert_a_parity, assert_a_signal_parity};

#[test]
fn a_public_surface_aliases_documentation_and_feature_match_the_pin() {
    let elisp_form = r##"(list
              (featurep 'a)
              (seq-every-p
               #'fboundp
               '(a-associative-p
                 a-associative?
                 a-get
                 a--alist-get
                 a-get-in
                 a-get*
                 a-has-key
                 a-has-key?
                 a-assoc-1
                 a-assoc
                 a-keys
                 a-vals
                 a-reduce-kv
                 a-count
                 a-equal
                 a-equal?
                 a-merge
                 a-merge-with
                 a-alist
                 a-list
                 a-hash-table
                 a-assoc-in
                 a-dissoc--list
                 a-dissoc--hash-table
                 a-dissoc
                 a-update
                 a-update-in))
              (mapcar
               (lambda (aliases)
                 (eq
                  (indirect-function
                   (car aliases))
                  (indirect-function
                   (cadr aliases))))
               '((a-associative?
                  a-associative-p)
                 (a-has-key?
                  a-has-key)
                 (a-equal?
                  a-equal)
                 (a-list
                  a-alist)))
              (fboundp 'a-has-key-p)
              (mapcar
               (lambda (function)
                 (car
                  (split-string
                   (documentation function)
                   "\n")))
               '(a-get
                 a-equal
                 a-update-in)))"##;
    let expect = expect![[
        r#"OK (t t (t t t t) nil ("Return the value MAP mapped to KEY, NOT-FOUND or nil if key not present." "Compare collections A, B for value equality." "In collection COLL, at location KEYS, apply FN with extra args ARGS."))"#
    ]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_associative_predicate_covers_empty_malformed_hash_and_sequence_shapes() {
    let elisp_form = r##"(let ((table
                    (a-hash-table
                     :key 1)))
               (mapcar
                #'a-associative-p
                (list
                 nil
                 '()
                 '((:key . 1))
                 table
                 []
                 [1]
                 '(:key 1)
                 '(nil)
                 '((malformed)))))"##;
    let expect = expect!["OK (t t t t nil nil nil nil t)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_alist_lookup_uses_equal_returns_the_first_duplicate_and_distinguishes_nil() {
    let elisp_form = r##"(let* ((stored
                      (copy-sequence "same"))
                     (lookup
                      (copy-sequence "same"))
                     (map
                      (list
                       (cons stored 'first)
                       (cons :duplicate 1)
                       (cons :duplicate 2)
                       (cons :nil-value nil))))
               (list
                (a-get map lookup)
                (a-get map :duplicate)
                (a-get map :nil-value
                       'fallback)
                (a-get map :missing
                       'fallback)
                (a-get nil :missing
                       'fallback)
                (a--alist-get
                 map lookup
                 'fallback)))"##;
    let expect = expect!["OK (first 1 nil fallback fallback first)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_vector_lookup_and_key_membership_cover_nil_bounds_and_key_types() {
    let elisp_form = r##"(let ((vector
                    [nil one two]))
               (list
                (a-get vector 0
                       'fallback)
                (a-get vector 2)
                (a-get vector 3
                       'fallback)
                (a-get [] 0
                       'fallback)
                (mapcar
                 (lambda (key)
                   (a-has-key vector
                              key))
                 '(0 2 3 -1 1.0
                     :one))))"##;
    let expect = expect!["OK (nil two fallback fallback (t t nil nil nil nil))"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_hash_lookup_uses_equal_and_exposes_the_reserved_missing_sentinel_quirk() {
    let elisp_form = r##"(let* ((stored
                      (copy-sequence "key"))
                     (lookup
                      (copy-sequence "key"))
                     (table
                      (a-hash-table
                       stored 10
                       :nil-value nil
                       :sentinel
                       :not-found)))
               (list
                (a-get table lookup)
                (a-get table :nil-value
                       'fallback)
                (a-get table :missing
                       'fallback)
                (a-has-key table
                           :nil-value)
                (a-has-key table
                           :sentinel)
                (hash-table-test table)
                (a-count table)))"##;
    let expect = expect!["OK (10 nil fallback t nil equal 3)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_get_rejects_a_non_associative_value_with_the_exact_user_error() {
    let elisp_form = r##"(a-get 5 :missing)"##;
    let expect = expect![[r#"ERR (user-error "Not associative: 5")"#]];

    assert_a_signal_parity(elisp_form, expect);
}

#[test]
fn a_has_key_rejects_a_non_associative_value_with_the_exact_user_error() {
    let elisp_form = r##"(a-has-key 'symbol :missing)"##;
    let expect = expect![[r#"ERR (user-error "Not associative: symbol")"#]];

    assert_a_signal_parity(elisp_form, expect);
}

#[test]
fn a_get_in_traverses_mixed_structures_and_preserves_empty_key_identity() {
    let elisp_form = r##"(let* ((leaf
                      (a-hash-table
                       "leaf" 42))
                     (tree
                      (vector
                       'zero
                       (a-list
                        :branch leaf))))
               (list
                (a-get-in
                 tree
                 [1 :branch "leaf"]
                 'missing)
                (a-get-in
                 tree
                 [1 :missing "leaf"]
                 'missing)
                (a-get-in [] []
                          'missing)
                (eq tree
                    (a-get-in tree []))))"##;
    let expect = expect!["OK (42 missing [] t)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_get_in_rejects_a_non_associative_intermediate_value() {
    let elisp_form = r##"(a-get-in
             '((:branch . 5))
             '(:branch :leaf)
             'missing)"##;
    let expect = expect![[r#"ERR (user-error "Not associative: 5")"#]];

    assert_a_signal_parity(elisp_form, expect);
}

#[test]
fn a_get_star_expands_nested_lookups_and_evaluates_each_form_once_in_order() {
    let elisp_form = r##"(let ((events nil)
                  (tree
                   (a-list
                    :a
                    (a-list
                     :b
                     (a-list
                      :c 9)))))
               (list
                (a-get*
                 (prog1 tree
                   (push 'map events))
                 (prog1 :a
                   (push 'a events))
                 (prog1 :b
                   (push 'b events))
                 (prog1 :c
                   (push 'c events)))
                (nreverse events)
                (macroexpand-1
                 '(a-get*
                   tree :a :b :c))))"##;
    let expect = expect![[r#"OK (9 (map a b c) (a-get (a-get (a-get tree :a) :b) :c))"#]];

    assert_a_parity(elisp_form, expect);
}
