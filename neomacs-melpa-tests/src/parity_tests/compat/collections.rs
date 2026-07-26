use expect_test::expect;

use super::{assert_compat_parity, assert_compat_signal_parity};

#[test]
fn compat_ensure_list_and_proper_list_cover_atoms_dotted_and_circular_inputs() {
    let elisp_form = r##"(let ((circle (list 1 2 3)))
               (setcdr (last circle) circle)
               (list
                (ensure-list nil)
                (ensure-list 1)
                (ensure-list '(1 . 2))
                (ensure-proper-list nil)
                (ensure-proper-list 1)
                (ensure-proper-list '(1 . 2))
                (mapcar #'proper-list-p
                        (list nil
                              '(1)
                              '(1 2 3)
                              '(1 . 2)
                              '(1 2 . 3)
                              circle
                              1
                              "abc"
                              [1 2 3]))))"##;
    let expect =
        expect![[r#"OK (nil (1) (1 . 2) nil (1) ((1 . 2)) (0 1 3 nil nil nil nil nil nil))"#]];

    assert_compat_parity(elisp_form, expect);
}

#[test]
fn compat_take_drop_and_ntake_preserve_exact_copy_and_mutation_semantics() {
    let elisp_form = r##"(let* ((source (list 1 2 3 4))
                    (taken (take 2 source))
                    (dropped (drop 2 source))
                    (mutable (list 'a 'b 'c 'd))
                    (tail (cddr mutable))
                    (ntaken (ntake 2 mutable)))
               (list
                (copy-tree taken)
                (copy-tree dropped)
                (copy-tree source)
                (eq taken source)
                (eq dropped (cddr source))
                (copy-tree ntaken)
                (copy-tree mutable)
                (copy-tree tail)
                (eq ntaken mutable)))"##;
    let expect = expect![[r#"OK ((1 2) (3 4) (1 2 3 4) nil t (a b) (a b) (c d) t)"#]];

    assert_compat_parity(elisp_form, expect);
}

#[test]
fn compat_sequence_predicates_cover_closures_function_values_and_boundaries() {
    let elisp_form = r##"(let ((numbers '(3 2 1 0 -1 -2 -3))
                    (threshold 1))
               (list
                (copy-tree
                 (drop-while #'plusp numbers))
                (copy-tree
                 (drop-while
                  (lambda (number)
                    (> number threshold))
                  numbers))
                (copy-tree
                 (take-while #'plusp numbers))
                (copy-tree
                 (take-while
                  (lambda (number)
                    (> number threshold))
                  numbers))
                (all #'numberp numbers)
                (all #'plusp numbers)
                (copy-tree
                 (member-if #'zerop numbers))
                (copy-tree
                 (funcall
                  (identity #'member-if)
                  #'minusp numbers))))"##;
    let expect = expect![[
        r#"OK ((0 -1 -2 -3) (1 0 -1 -2 -3) (3 2 1) (3 2) t nil (0 -1 -2 -3) (-1 -2 -3))"#
    ]];

    assert_compat_parity(elisp_form, expect);
}

#[test]
fn compat_length_comparators_cover_lists_vectors_and_equal_boundaries() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (limit)
                  (length< '(a b c) limit))
                '(0 2 3 4))
               (mapcar
                (lambda (limit)
                  (length> [a b c] limit))
                '(0 2 3 4))
               (length= nil 0)
               (length= "abc" 3)
               (length= '(a b c) 2))"##;
    let expect = expect![[r#"OK ((nil nil nil t) (t t nil nil) t t nil)"#]];

    assert_compat_parity(elisp_form, expect);
}

#[test]
fn compat_length_comparator_rejects_non_sequence() {
    let elisp_form = "(length< 3 1)";
    let expect = expect![[r#"ERR (wrong-type-argument sequencep 3)"#]];

    assert_compat_signal_parity(elisp_form, expect);
}

#[test]
fn compat_hash_table_contains_distinguishes_missing_from_nil_value() {
    let elisp_form = r##"(let ((table (make-hash-table :test #'equal)))
               (puthash "present-nil" nil table)
               (puthash "present-value" 7 table)
               (list
                (hash-table-contains-p "present-nil" table)
                (gethash "present-nil" table 'fallback)
                (hash-table-contains-p "present-value" table)
                (gethash "present-value" table)
                (hash-table-contains-p "missing" table)
                (gethash "missing" table 'fallback)))"##;
    let expect = expect![[r#"OK (t nil t 7 nil fallback)"#]];

    assert_compat_parity(elisp_form, expect);
}
