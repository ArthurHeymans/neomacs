//! Optional-argument defaulting parity: sort default predicate + :reverse,
//! cl-subseq negative bounds, cl-find/position/count :start/:from-end,
//! cl-remove/substitute :count, read-from-string start/end, number-sequence
//! default step, assoc-default test/default, alist-get remove via setf,
//! cl-getf/plist-get default, string-trim/pad custom args.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn alist_get_remove_setf() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(let ((al (list (cons 'a 1) (cons 'b 2))))
  (setf (alist-get 'a al nil 'remove) nil)
  (list al (alist-get 'b al)))"##,
    );
}

#[test]
fn assoc_default_opt() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (assoc-default "b" '(("a" . 1) ("b" . 2)))
        (assoc-default 2 '((1 . a) (2 . b)) #'=)
        (assoc-default "x" '(("a" . 1)) nil 'fallback))"##,
    );
}

#[test]
fn cl_find_start_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(list (cl-find 3 '(1 2 3 2 3) :start 3) (cl-position 2 '(1 2 3 2) :from-end t)
      (cl-count 2 '(1 2 2 3 2) :start 2) (cl-find-if #'cl-evenp '(1 3 5 4) :from-end t))"##,
    );
}

#[test]
fn cl_getf_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(list (cl-getf '(:a 1 :b 2) :b) (cl-getf '(:a 1) :missing 'def)
      (plist-get '(:a 1 :b 2) :c))"##,
    );
}

#[test]
fn cl_remove_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(list (cl-remove 2 '(2 2 2 2) :count 2) (cl-remove-if #'cl-evenp '(2 4 6 8) :count 1)
      (cl-substitute 9 2 '(2 2 2) :count 2))"##,
    );
}

#[test]
fn cl_subseq_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(list (cl-subseq "hello" -3) (cl-subseq "hello" 1 -1) (cl-subseq '(1 2 3 4 5) -2)
      (cl-subseq [1 2 3 4] 0 -1))"##,
    );
}

#[test]
fn number_sequence_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (number-sequence 1 5) (number-sequence 5) (number-sequence 1 10 3)
        (number-sequence 0 0))"##,
    );
}

#[test]
fn read_from_string_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (read-from-string "abc def" 4) (read-from-string "(1 2 3)" 0 5)
        (car (read-from-string "  42  ")))"##,
    );
}

#[test]
fn sort_default_pred() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (sort (list 3 1 2)) (sort (vector 5 2 8))
        (sort (list "c" "a" "b")) (sort (list 3 1 2) :reverse t))"##,
    );
}

#[test]
fn string_trim_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-trim "xxabcxx" "x+" "x+") (string-trim "  hi  ")
        (string-trim-left "...test" "\\.+") (string-pad "ab" 5 ?- t))"##,
    );
}
