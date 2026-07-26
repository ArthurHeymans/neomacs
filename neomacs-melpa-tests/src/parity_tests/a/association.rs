use expect_test::expect;

use super::{assert_a_parity, assert_a_signal_parity};

#[test]
fn a_assoc_updates_all_equal_alist_duplicates_prepends_new_keys_and_is_immutable() {
    let elisp_form = r##"(let* ((source
                      '((:a . 1)
                        (:duplicate . 2)
                        (:duplicate . 3)))
                     (result
                      (a-assoc
                       source
                       :duplicate 9
                       :b 4)))
               (list
                result
                source
                (eq result source)))"##;
    let expect = expect![[
        r#"OK (((:b . 4) #1=(:a . 1) (:duplicate . 9) (:duplicate . 9)) (#1# (:duplicate . 2) (:duplicate . 3)) nil)"#
    ]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_assoc_copies_vectors_extends_gaps_and_returns_nil_for_invalid_indices() {
    let elisp_form = r##"(let* ((source [zero one two])
                     (result
                      (a-assoc
                       source
                       1 'changed
                       5 'five
                       3 'three)))
               (list
                result
                source
                (eq result source)
                (a-assoc-1
                 source -1 'bad)
                (a-assoc-1
                 source :index 'bad)))"##;
    let expect = expect!["OK ([zero changed two three nil five] [zero one two] nil nil nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_assoc_clones_hash_tables_preserves_the_test_and_leaves_the_source_unmodified() {
    let elisp_form = r##"(let* ((source
                      (a-hash-table
                       "key" 1))
                     (result
                      (a-assoc
                       source
                       (copy-sequence
                        "key")
                       2
                       :new 3)))
               (list
                (eq source result)
                (hash-table-test result)
                (a-count result)
                (a-get result "key")
                (a-get result :new)
                (a-get source "key")
                (a-has-key source :new)))"##;
    let expect = expect!["OK (nil equal 2 2 3 1 nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_assoc_rejects_an_odd_number_of_key_value_arguments() {
    let elisp_form = r##"(a-assoc nil :key)"##;
    let expect = expect![[r#"ERR (user-error "a-assoc requires an even number of arguments!")"#]];

    assert_a_signal_parity(elisp_form, expect);
}

#[test]
fn a_keys_and_values_preserve_alist_order_normalize_hashes_and_ignore_vectors() {
    let elisp_form = r##"(let ((table
                    (a-hash-table
                     :b 2
                     :a 1)))
               (list
                (a-keys
                 '((:b . 2)
                   (:a . 1)
                   (:b . 3)))
                (a-vals
                 '((:b . 2)
                   (:a . 1)
                   (:b . 3)))
                (sort
                 (a-keys table)
                 (lambda (left right)
                   (string<
                    (symbol-name left)
                    (symbol-name right))))
                (sort
                 (a-vals table)
                 #'<)
                (a-keys [one two])
                (a-vals [one two])))"##;
    let expect = expect!["OK ((:b :a :b) (2 1 3) (:a :b) (1 2) nil nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_count_covers_every_supported_sequence_hash_and_unsupported_atom() {
    let elisp_form = r##"(list
              (a-count nil)
              (a-count
               '((:a . 1)
                 (:b . 2)))
              (a-count [a b c])
              (a-count "λx")
              (a-count
               (a-hash-table
                :a 1 :b 2))
              (a-count 'atom))"##;
    let expect = expect!["OK (0 2 3 2 2 nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_reduce_kv_visits_alists_in_key_order_and_hashes_once_per_entry() {
    let elisp_form = r##"(let ((table
                    (a-hash-table
                     :b 2
                     :a 1)))
               (list
                (a-reduce-kv
                 (lambda (acc key value)
                   (cons
                    (list key value)
                    acc))
                 nil
                 '((:a . 1)
                   (:b . 2)))
                (sort
                 (a-reduce-kv
                  (lambda (acc key value)
                    (cons
                     (cons key value)
                     acc))
                  nil table)
                 (lambda (left right)
                   (string<
                    (symbol-name
                     (car left))
                    (symbol-name
                     (car right)))))))"##;
    let expect = expect!["OK (((:b 2) (:a 1)) ((:a . 1) (:b . 2)))"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_alist_and_hash_constructors_cover_aliases_duplicates_odd_tails_and_tests() {
    let elisp_form = r##"(let* ((alist
                      (a-alist
                       :a 1 :b 2
                       :dangling))
                     (alias
                      (a-list
                       :a 1 :b 2
                       :dangling))
                     (table
                      (a-hash-table
                       "key" 1
                       (copy-sequence
                        "key")
                       2
                       :dangling)))
               (list
                alist
                alias
                (equal alist alias)
                (hash-table-test table)
                (a-count table)
                (a-get table "key")
                (a-get table
                       :dangling
                       'missing)))"##;
    let expect = expect![[
        r#"OK (((:a . 1) (:b . 2) (:dangling)) ((:a . 1) (:b . 2) (:dangling)) t equal 2 2 nil)"#
    ]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_dissoc_on_alists_reverses_retained_entries_and_re_reads_duplicate_values() {
    let elisp_form = r##"(let ((source
                    '((:a . 1)
                      (:b . 2)
                      (:c . 3)
                      (:b . 4))))
               (list
                (a-dissoc source :b)
                (a-dissoc source
                          :missing)
                source))"##;
    let expect = expect![[
        r#"OK (((:c . 3) (:a . 1)) ((:b . 2) (:c . 3) (:b . 2) (:a . 1)) ((:a . 1) (:b . 2) (:c . 3) (:b . 4)))"#
    ]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_dissoc_on_hashes_preserves_test_clones_and_removes_all_requested_keys() {
    let elisp_form = r##"(let* ((source
                      (a-hash-table
                       :a 1 :b nil
                       :c 3))
                     (result
                      (a-dissoc
                       source :b
                       :missing)))
               (list
                (eq source result)
                (hash-table-test result)
                (sort
                 (a-keys result)
                 (lambda (left right)
                   (string<
                    (symbol-name left)
                    (symbol-name right))))
                (a-has-key result :b)
                (a-has-key source :b)
                (a-count source)))"##;
    let expect = expect!["OK (nil equal (:a :c) nil t 3)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_dissoc_returns_nil_for_unsupported_collections() {
    let elisp_form = r##"(list
              (a-dissoc [a b] 0)
              (a-dissoc 'atom :key)
              (a-dissoc 42 :key))"##;
    let expect = expect!["OK (nil nil nil)"];

    assert_a_parity(elisp_form, expect);
}
