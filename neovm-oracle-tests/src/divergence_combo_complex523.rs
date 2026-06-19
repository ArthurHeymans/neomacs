/// Batch 523: sequence operations - copy-sequence on all types, length implicit.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx523_copy_sequence_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((v [1 2 3]) (c (copy-sequence [1 2 3])))
  (aset c 0 99)
  (list (aref v 0) (aref c 0)))
"##,
    );
}

#[test]
fn div_cx523_copy_sequence_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((l '(1 2 3)) (c (copy-sequence '(1 2 3))))
  (setcar c 99)
  (list (car l) (car c)))
"##,
    );
}

#[test]
fn div_cx523_copy_sequence_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "hello") (c (copy-sequence "hello")))
  (list s c (equal s c)))
"##,
    );
}

#[test]
fn div_cx523_length_implicit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (length [1 2 3]) (length '(a b c d)) (length "hello") (length (make-bool-vector 10 t)))
"##,
    );
}

#[test]
fn div_cx523_elt_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (elt '(10 20 30) 0) (elt '(10 20 30) 2) (elt '(10 20 30) -1))
"##,
    );
}

#[test]
fn div_cx523_elt_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (elt [10 20 30] 0) (elt [10 20 30] 2) (elt [10 20 30] -1))
"##,
    );
}

#[test]
fn div_cx523_elt_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (elt "abc" 0) (elt "abc" 2) (elt "abc" -1))
"##,
    );
}

#[test]
fn div_cx523_reverse_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (reverse "hello") (reverse "a") (reverse ""))
"##,
    );
}

#[test]
fn div_cx523_reverse_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (reverse '(1 2 3)) (reverse '(a)) (reverse '()))
"##,
    );
}

#[test]
fn div_cx523_reverse_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (reverse [1 2 3]) (reverse [1]) (reverse []))
"##,
    );
}

#[test]
fn div_cx523_nreverse_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((l (list 1 2 3))) (nreverse l) l)
"##,
    );
}

#[test]
fn div_cx523_sort_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(sort '(3 1 4 1 5 9) #'<)
"##,
    );
}

#[test]
fn div_cx523_member_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (member 3 '(1 2 3 4)) (member 5 '(1 2 3)))
"##,
    );
}

#[test]
fn div_cx523_delete_dup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (delete-dups '(1 2 1 3 2 4)) (delete-dups '(a b a)))
"##,
    );
}

#[test]
fn div_cx523_assoc_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (assq 'b '((a . 1) (b . 2) (c . 3))) (assq 'd '((a . 1))))
"##,
    );
}
