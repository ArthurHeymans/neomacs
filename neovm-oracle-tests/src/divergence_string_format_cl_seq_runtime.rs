//! string/format helpers (pad/trim/replace/join/distance/case/format-spec
//! numeric formats) and cl-lib/seq (loop, destructuring-bind, defstruct,
//! case/typecase, reduce/remove/position, subseq/sort, seq-*) parity.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn format_number_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%d" 3.9) (format "%.2f" 3.14159) (format "%x" 255)
        (format "%o" 8) (format "%e" 12345.678) (format "%g" 0.0001) (format "%5.2f" 3.1))"##,
    );
}

#[test]
fn format_spec_fn() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'format-spec)
(list (format-spec "%a-%b" '((?a . "X") (?b . "Y")))
      (format-spec "%-5a|" '((?a . "hi")))
      (format-spec "%05d" '((?d . 42))))"##,
    );
}

#[test]
fn string_case_fns() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (upcase "héllo") (downcase "HÉLLO") (capitalize "hello world")
        (upcase-initials "hello world") (capitalize "foo-bar"))"##,
    );
}

#[test]
fn string_chop_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-chop-newline "hi\n") (string-chop-newline "hi")
        (split-string "a\nb\nc" "\n") (string-lines "a\nb\nc"))"##,
    );
}

#[test]
fn string_distance_prefix() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-distance "kitten" "sitting")
        (string-prefix-p "he" "hello") (string-suffix-p "lo" "hello")
        (string-prefix-p "HE" "hello" t))"##,
    );
}

#[test]
fn string_pad_trim() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-pad "ab" 5) (string-pad "ab" 5 ?* ) (string-pad "ab" 5 nil t)
        (string-trim "  hi  ") (string-trim-left "xxhi" "x+") (string-trim-right "hixx" "x+"))"##,
    );
}

#[test]
fn string_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-replace "a" "X" "banana") (string-replace "" "X" "ab")
        (string-remove-prefix "pre" "prefix") (string-remove-suffix "ix" "prefix"))"##,
    );
}

#[test]
fn string_split_join() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-join '("a" "b" "c") "-") (string-join '("x" "y"))
        (string-search "lo" "hello") (string-search "z" "hello"))"##,
    );
}

#[test]
fn cl_case_typecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(list (cl-case 2 (1 'one) (2 'two) (t 'other))
      (cl-typecase "s" (integer 'i) (string 'str) (t 'other))
      (cl-etypecase 5 (number 'num)))"##,
    );
}

#[test]
fn cl_defstruct() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(cl-defstruct (pt-xyz (:constructor make-pt-xyz)) x y)
(let ((p (make-pt-xyz :x 3 :y 4)))
  (list (pt-xyz-x p) (pt-xyz-y p) (pt-xyz-p p) (type-of p) (recordp p)))"##,
    );
}

#[test]
fn cl_destructuring() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(cl-destructuring-bind (a b &optional c &rest d) '(1 2 3 4 5)
  (list a b c d))"##,
    );
}

#[test]
fn cl_loop_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(list (cl-loop for i from 1 to 5 collect (* i i))
      (cl-loop for x in '(1 2 3 4) when (cl-evenp x) sum x)
      (cl-loop repeat 3 collect 'a))"##,
    );
}

#[test]
fn cl_reduce_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(list (cl-reduce #'+ '(1 2 3 4) :initial-value 10)
      (cl-remove-if #'cl-oddp '(1 2 3 4 5 6))
      (cl-remove-duplicates '(1 2 2 3 3 3))
      (cl-position 3 '(1 2 3 4)))"##,
    );
}

#[test]
fn cl_subseq_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(list (cl-subseq '(1 2 3 4 5) 1 3) (cl-subseq "hello" 1)
      (cl-sort (list 3 1 2) #'<) (cl-sort (vector 3 1 2) #'>))"##,
    );
}

#[test]
fn cl_values_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(cl-multiple-value-bind (a b) (cl-values 1 2) (list a b))"##,
    );
}

#[test]
fn seq_fns() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'seq)
(list (seq-filter #'cl-evenp '(1 2 3 4)) (seq-map #'1+ [1 2 3])
      (seq-reduce #'+ '(1 2 3) 0) (seq-take '(1 2 3 4) 2)
      (seq-partition '(1 2 3 4 5) 2) (seq-uniq '(1 1 2 3 3)))"##,
    );
}
