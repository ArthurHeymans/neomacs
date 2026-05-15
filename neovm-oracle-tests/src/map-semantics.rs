//! Oracle parity tests for GNU mapping primitive edge semantics.
//!
//! GNU implements `mapconcat`, `mapcar`, `mapc`, and `mapcan` in `src/fns.c`
//! through `mapcar1`.  `mapcar1` computes sequence length up front, but for
//! lists it stops early if the list is shortened as a side effect.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_mapcar_mapc_sequence_types_and_char_table_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((bv (make-bool-vector 3 nil)))
  (aset bv 1 t)
  (list
   (mapcar #'identity '(a b c))
   (mapcar #'identity [a b c])
   (mapcar #'identity "aé")
   (mapcar #'identity bv)
   (let ((v [a b c]))
     (list (eq v (mapc #'ignore v)) v))
   (condition-case err
       (mapcar #'identity (make-char-table 'test nil))
     (error (list (car err) (cdr err))))
   (condition-case err
       (mapc #'ignore 42)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_mapcar_stops_when_list_shortened_by_callback() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((seq (list 1 2 3 4))
      (seen nil))
  (list
   (mapcar (lambda (x)
             (push x seen)
             (when (= x 1)
               (setcdr seq nil))
             (* x 10))
           seq)
   (nreverse seen)
   seq))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_mapconcat_separator_and_return_validation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (mapconcat #'identity '("a" nil "c") nil)
 (mapconcat #'identity '("a" "b" "c") [?|])
 (mapconcat (lambda (x) (vector (+ ?0 x))) '(1 2 3) '(?-))
 (mapconcat #'identity [] ",")
 (condition-case err
     (mapconcat #'identity '("a" bad "c") ",")
   (error (list (car err) (cdr err))))
 (condition-case err
     (mapconcat (lambda (_x) 42) '(a) ",")
   (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_mapcan_destructive_nconc_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((first (list 'a))
       (second (list 'b))
       (third (list 'c))
       (input (list first nil second third))
       (result (mapcan #'identity input)))
  (list
   result
   (eq result first)
   (cdr first)
   (eq (cdr first) second)
   (eq (cdr second) third)
   input
   (condition-case err
       (mapcan (lambda (_x) 42) '(a b))
     (error (list (car err) (cdr err)))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_mapping_dotted_and_circular_input_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((cycle (list 'a 'b))
       (_ (setcdr (last cycle) cycle)))
  (list
   (condition-case err
       (mapcar #'identity '(a b . c))
     (error (list (car err) (cdr err))))
   (condition-case err
       (mapconcat #'identity '("a" "b" . c) ",")
     (error (list (car err) (cdr err))))
   (condition-case err
       (mapcan #'list '(a b . c))
     (error (list (car err) (cdr err))))
   (condition-case err
       (mapcar #'identity cycle)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
