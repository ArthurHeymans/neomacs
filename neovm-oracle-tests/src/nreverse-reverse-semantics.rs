//! Oracle parity tests for GNU `nreverse`/`reverse` edge semantics.
//!
//! GNU implements these in `src/fns.c`: `nreverse` destructively reverses
//! lists, vectors, and bool-vectors, but returns `reverse` for strings; `reverse`
//! returns fresh sequence storage and uses a different non-sequence error
//! predicate from `nreverse`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_nreverse_mutates_list_spine_and_vector_storage() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((list (list 'a 'b 'c))
       (first-cons list)
       (reversed-list (nreverse list))
       (vec (vector 1 2 3 4))
       (same-vec (nreverse vec)))
  (list reversed-list
        (eq (car (last reversed-list)) first-cons)
        first-cons
        vec
        same-vec
        (eq vec same-vec)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_nreverse_bool_vector_mutates_and_string_does_not() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((bv (make-bool-vector 6 nil))
       (_ (aset bv 0 t))
       (_ (aset bv 2 t))
       (_ (aset bv 5 t))
       (same-bv (nreverse bv))
       (s (copy-sequence "abcd"))
       (rs (nreverse s)))
  (list (eq bv same-bv)
        (mapcar (lambda (i) (aref bv i)) '(0 1 2 3 4 5))
        s
        rs
        (eq s rs)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_reverse_is_shallow_and_does_not_mutate_inputs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((shared (list 'cell))
       (list (list 'a shared 'c))
       (rev-list (reverse list))
       (vec (vector shared 2 3))
       (rev-vec (reverse vec)))
  (aset rev-vec 0 'changed)
  (list list
        rev-list
        (eq (cadr rev-list) shared)
        vec
        rev-vec
        (eq vec rev-vec)
        (eq (aref vec 0) (aref rev-vec 2))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_reverse_string_properties_and_multibyte_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((s (copy-sequence (concat "a" (char-to-string #x4e2d) "b")))
       (_ (put-text-property 0 2 'face 'bold s))
       (r (reverse s)))
  (list s
        r
        (substring-no-properties r)
        (multibyte-string-p r)
        (text-properties-at 0 r)
        (text-properties-at 1 r)
        (text-properties-at 2 r)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_reverse_and_nreverse_error_payloads() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (condition-case err
     (reverse 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (nreverse 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (reverse '(a b . c))
   (error (list (car err) (cdr err))))
 (condition-case err
     (nreverse '(a b . c))
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
