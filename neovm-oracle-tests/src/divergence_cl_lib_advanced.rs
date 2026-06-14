//! cl-lib advanced divergence probes (calibration).
//!
//! Probes the trickier cl-lib macros/functions: cl-letf, cl-symbol-macrolet,
//! cl-typecase, cl-check-type, cl-assert, cl-rotatef, cl-shiftf, cl-coerce,
//! cl-defstruct (conc-name/constructor/predicate), cl-getf, cl-loop clauses,
//! cl-do, cl-position/find/count, cl-remove-duplicates, cl-sort/stable-sort,
//! cl-subseq, cl-merge, cl-labels, cl-reduce.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cl_letf_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((x 5))
  (list (cl-letf ((x 10)) x) x))
"##,
    );
}

#[test]
fn div_cl_letf_place() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((lst (list 1 2 3)))
  (cl-letf (((nth 1) lst) 99))
  lst)
"##,
    );
}

#[test]
fn div_cl_symbol_macrolet() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((lst (list 1 2 3)))
  (cl-symbol-macrolet ((x (car lst)))
    (setq x 99))
  lst)
"##,
    );
}

#[test]
fn div_cl_typecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-typecase 5 (string "s") (number "n") (t "o"))
      (cl-typecase "x" (string "s") (number "n") (t "o"))
      (cl-typecase '(1) (string "s") (cons "c") (t "o")))
"##,
    );
}

#[test]
fn div_cl_check_type_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (cl-check-type 5 string) (error (car err)))
      (condition-case err (cl-check-type "x" string) (error :passed)))
"##,
    );
}

#[test]
fn div_cl_assert_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (cl-assert (= 1 2) t "nope") (error (car err)))
      (condition-case err (cl-assert (= 1 1) t "nope") (error :passed)))
"##,
    );
}

#[test]
fn div_cl_rotatef() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((a 1) (b 2) (c 3))
  (cl-rotatef a b c)
  (list a b c))
"##,
    );
}

#[test]
fn div_cl_shiftf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((a 1) (b 2))
  (list (cl-shiftf a b 3) a b))
"##,
    );
}

#[test]
fn div_cl_coerce() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-coerce '(1 2 3) 'vector)
      (cl-coerce [1 2 3] 'list)
      (cl-coerce "ab" 'list)
      (cl-coerce '(97 98) 'string))
"##,
    );
}

#[test]
fn div_cl_defstruct_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (cl-defstruct neo-pt x y)
  (let ((p (make-neo-pt :x 1 :y 2)))
    (list (neo-pt-x p) (neo-pt-y p) (neo-pt-p p))))
"##,
    );
}

#[test]
fn div_cl_defstruct_conc_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (cl-defstruct (neo-box (:conc-name neo-box-)) size)
  (list (neo-box-size (make-neo-box :size 5))
        (neo-box-p (make-neo-box :size 5))))
"##,
    );
}

#[test]
fn div_cl_defstruct_named_constructor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (cl-defstruct (neo-coord (:constructor neo-coord-create (a b))) (x a) (y b))
  (let ((c (neo-coord-create 7 8)))
    (list (neo-coord-x c) (neo-coord-y c))))
"##,
    );
}

#[test]
fn div_cl_getf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (list :a 1 :b 2)))
  (list (cl-getf p :a) (cl-getf p :c) (cl-getf p :c :default)))
"##,
    );
}

#[test]
fn div_cl_loop_collect_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-loop for x in '(1 2 3) collect (* x 2))
      (cl-loop for x in '(1 2 3) sum x)
      (cl-loop for x from 1 to 5 sum x))
"##,
    );
}

#[test]
fn div_cl_loop_while_for_equals() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-loop for x in '(1 2 3 4) while (< x 3) collect x)
      (cl-loop for x in '(1 2 3) for y = (* x 2) collect (list x y)))
"##,
    );
}

#[test]
fn div_cl_loop_into_maximize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-loop for x in '(3 1 4 1 5 9 2 6) maximize x into m finally (return m))
"##,
    );
}

#[test]
fn div_cl_do() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-do ((x 0 (1+ x)) (acc nil (push x acc))) ((>= x 3) (reverse acc)))
"##,
    );
}

#[test]
fn div_cl_position_find_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-position 2 '(1 2 3))
      (cl-find 3 '(1 2 3))
      (cl-count nil '(1 nil 2 nil))))
"##,
    );
}

#[test]
fn div_cl_remove_duplicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-remove-duplicates '(1 2 2 3 3 3 1))
"##,
    );
}

#[test]
fn div_cl_sort_stable_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-sort (copy-sequence '(3 1 2 1 3)) #'<)
      (cl-stable-sort (copy-sequence '((1 . :a) (1 . :b) (2 . :c)))
                      (lambda (a b) (< (car a) (car b)))))
"##,
    );
}

#[test]
fn div_cl_subseq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-subseq '(1 2 3 4 5) 1 3)
      (cl-subseq "hello" 1 4)
      (cl-subseq [1 2 3 4] 2))
"##,
    );
}

#[test]
fn div_cl_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-merge 'list '(1 3 5) '(2 4 6) #'<)
"##,
    );
}

#[test]
fn div_cl_labels_recursive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-labels ((fact (n) (if (= n 0) 1 (* n (fact (1- n)))))) (fact 5))
"##,
    );
}

#[test]
fn div_cl_reduce() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-reduce #'+ '(1 2 3 4))
      (cl-reduce #'cons '(1 2 3) :from-end t :initial-value 0))
"##,
    );
}
