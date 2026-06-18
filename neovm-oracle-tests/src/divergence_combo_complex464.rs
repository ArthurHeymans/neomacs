/// Batch 464: regexp-opt extreme, format extreme, subr-x deep, cl-lib seq.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx464_regexp_opt_extreme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (regexp-opt '("a" "b" "c"))
      (regexp-opt '("hello" "help" "helicopter"))
      (regexp-opt '("a") 'paren))"##,
    );
}

#[test]
fn div_cx464_regexp_opt_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (regexp-opt-depth (regexp-opt '("foo" "bar" "baz") 'paren))
      (regexp-opt '("hel" "hell" "hello") 'shy))"##,
    );
}

#[test]
fn div_cx464_format_extreme_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%100s" "right")
      (format "%-100s" "left")
      (format "%0100d" 42))"##,
    );
}

#[test]
fn div_cx464_format_scientific() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%e" 3.14159)
      (format "%.3e" 3.14159)
      (format "%.10f" (/ 1.0 3.0))
      (format "%g" 1e10))"##,
    );
}

#[test]
fn div_cx464_subr_x_string_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (string-truncate-left 5 "hello world")
        (string-truncate-left 20 "short")
        (string-clean-whitespace " a   b   c ")))"##,
    );
}

#[test]
fn div_cx464_cl_lib_pairwise() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (cl-pairlis '(a b c) '(1 2 3))
      (cl-pairlis '(x y) '(10 20 t t)))"##,
    );
}

#[test]
fn div_cx464_cl_lib_set_diff() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (cl-set-difference '(1 2 3 4) '(3 4 5 6))
      (cl-union '(1 2 3) '(3 4 5))
      (cl-intersection '(1 2 3) '(2 3 4)))"##,
    );
}

#[test]
fn div_cx464_cl_lib_sort_by_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(cl-sort '((a 3) (b 1) (c 2)) #'< :key #'cadr)"##);
}

#[test]
fn div_cx464_seq_map_index() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'seq)
  (list (seq-map-indexed (lambda (e i) (cons i e)) '(a b c))
        (seq-filter #'numberp '(1 a 2 b 3 c))
        (seq-remove #'numberp '(1 a 2 b 3 c))))"##,
    );
}

#[test]
fn div_cx464_seq_reduce_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'seq)
  (list (seq-reduce #'+ '(1 2 3 4) 0)
        (seq-find #'numberp '(a b 3 c))
        (seq-count #'numberp '(1 a 2 b 3))))"##,
    );
}

#[test]
fn div_cx464_seq_zip_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'seq)
  (list (seq-into (seq-zip '(a b c) '(1 2 3)) 'list)
        (seq-sort #'string< '("c" "a" "b"))))"##,
    );
}

#[test]
fn div_cx464_subr_x_hash_table_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (let ((ht (make-hash-table :test 'equal)))
    (puthash "a" 1 ht)
    (puthash "b" 2 ht)
    (list (hash-table-keys ht)
          (hash-table-values ht))))"##,
    );
}

#[test]
fn div_cx464_subr_x_threading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (thread-first '(1 2 3 4) (reverse) (cl-rest))
        (thread-last '(1 2 3 4) (reverse) (cl-rest))))"##,
    );
}

#[test]
fn div_cx464_format_mixed_number_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%d %d %d" most-positive-fixnum 0 -1)
      (format "%x %x" 255 256)
      (format "%o %o" 63 64))"##,
    );
}

#[test]
fn div_cx464_format_heavy_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%s" "hello\nworld\ttab\rreturn")
      (format "%S" (intern "test-symbol"))
      (format "%% %.0f" 3.14))"##,
    );
}
