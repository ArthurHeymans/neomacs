//! Oracle parity tests for `take-while', `drop-while', `all', and `any'.

use super::common::assert_oracle_parity;

#[test]
fn oracle_take_drop_while_preserve_gnu_tail_semantics() {
    let form = r#"
(let* ((xs (list 1 2 3 4 5))
       (taken (take-while (lambda (x) (< x 4)) xs))
       (dropped (drop-while (lambda (x) (< x 4)) xs)))
  (list taken
        (eq taken xs)
        (eq dropped (nthcdr 3 xs))
        dropped
        xs))"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_take_drop_while_short_circuit_malformed_tails() {
    let form = r#"
(let ((calls nil)
      (xs '(1 2 3 . bad-tail)))
  (list
   ;; Stop before the malformed tail: no tail validation is forced.
   (take-while (lambda (x) (push x calls) (< x 3)) xs)
   (nreverse calls)
   (let ((calls nil))
     (list (eq (drop-while (lambda (x) (push x calls) (< x 3)) xs)
               (nthcdr 2 xs))
           (nreverse calls)))
   ;; Predicate keeps accepting cons cells, so the next `car' sees bad-tail.
   (condition-case err
       (take-while (lambda (x) (push x calls) t) xs)
     (error (car err)))
   (condition-case err
       (drop-while (lambda (x) (push x calls) t) xs)
     (error (car err)))))"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_all_any_are_defined_by_drop_while() {
    let form = r#"
(let ((xs (list 1 2 3 4))
      (ys '(1 2 3 . bad-tail)))
  (list
   (all #'numberp xs)
   (all (lambda (x) (< x 3)) xs)
   (all #'numberp nil)
   (any (lambda (x) (= x 3)) xs)
   (eq (any (lambda (x) (= x 3)) xs) (nthcdr 2 xs))
   (any (lambda (x) (> x 9)) xs)
   ;; `all' and `any' inherit `drop-while' short-circuiting.
   (all (lambda (x) (< x 3)) ys)
   (condition-case err
       (all #'numberp ys)
     (error (car err)))
   (any (lambda (x) (= x 2)) ys)
   (condition-case err
       (any (lambda (x) nil) ys)
     (error (car err)))))"#;
    assert_oracle_parity(form);
}
