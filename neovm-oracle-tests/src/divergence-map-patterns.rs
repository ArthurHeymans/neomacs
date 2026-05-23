//! Divergence tests: map, filter, reduce patterns across types.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_mapcar_mapc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (mapcar '1+ '(1 2 3))
  (mapc 'identity '(a b c))
  (mapconcat 'symbol-name '(a b c) "-")) "#,
    );
}

#[test]
fn divergence_mapcar_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (mapcar '1+ [1 2 3])
  (mapc 'identity [a b c])
  (length (mapcar '1+ [1 2 3]))) "#,
    );
}

#[test]
fn divergence_map_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (mapcar 'char-to-string "abc")
  (mapconcat 'char-to-string "xyz" "|")
  (length (mapcar 'identity "hello"))) "#,
    );
}

#[test]
fn divergence_dolist_dotimes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((result nil))
  (dolist (x '(1 2 3) result)
    (push x result))) "#,
    );
}

#[test]
fn divergence_dotimes_accumulate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((result nil))
  (dotimes (i 5 result)
    (push i result))) "#,
    );
}

#[test]
fn divergence_mapcan_mapconcat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (mapcan 'list '(1 2 3) '(a b c))
  (mapcan (lambda (x) (list x x)) '(1 2))
  (mapconcat 'number-to-string '(1 2 3) ", ")) "#,
    );
}

#[test]
fn divergence_map_into() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'map-into)
  (fboundp 'map-do)
  (fboundp 'map-apply)) "#,
    );
}

#[test]
fn divergence_copy_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((orig '(1 2 3))
        (v-orig [1 2 3])
        (s-orig "abc"))
  (list (equal orig (copy-sequence orig))
        (equal v-orig (copy-sequence v-orig))
        (equal s-orig (copy-sequence s-orig))
        (not (eq orig (copy-sequence orig))))) "#,
    );
}

#[test]
fn divergence_sequence_funcs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (length '(1 2 3))
  (length [1 2 3])
  (length "abc")
  (elt '(a b c) 1)
  (elt [a b c] 1)
  (elt "abc" 1)) "#,
    );
}

#[test]
fn divergence_nested_mapcar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (mapcar (lambda (row) (mapcar '1+ row))
          '((1 2) (3 4) (5 6)))
  (apply 'append (mapcar (lambda (x) (list x x)) '(1 2 3)))) "#,
    );
}
