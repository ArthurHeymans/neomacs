//! format-time-string deep (ISO week/year, 12h/%p, day names, zero-pad
//! defaults, %F/%T/%z combos, %- dash flag, %^ upcase, %s epoch) and subr-x
//! (thread-first/last, if/when-let, named-let, and-let*, string-*, hash keys/
//! values) parity.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn fts_12h_pm() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%I:%M:%S %p" '(26150 29968) t) (format-time-string "%l:%M%P" '(26150 29968) t) (format-time-string "%-I" '(26150 29968) t))"##,
    );
}

#[test]
fn fts_caret_upper_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%^A %^B" '(26150 29968) t) (format-time-string "%^p" '(26150 29968) t))"##,
    );
}

#[test]
fn fts_combined_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%FT%T%z" '(26150 29968) t) (format-time-string "%a, %d %b %Y %H:%M:%S" '(26150 29968) t))"##,
    );
}

#[test]
fn fts_dash_flag_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%-m/%-d/%Y" '(26150 29968) t) (format-time-string "%-H:%-M" '(26150 29968) t) (format-time-string "%-j" '(26150 29968) t))"##,
    );
}

#[test]
fn fts_day_names_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%a/%A/%b/%B" '(26150 29968) t) (format-time-string "%a/%A" '(25700 30000) t))"##,
    );
}

#[test]
fn fts_iso_week_year_combos() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%G-W%V-%u" '(26150 29968) t) (format-time-string "%Y%j" '(26150 29968) t) (format-time-string "%g" '(26150 29968) t))"##,
    );
}

#[test]
fn fts_seconds_since_epoch() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%s" '(26150 29968) t) (string-to-number (format-time-string "%s" '(26150 29968) t)))"##,
    );
}

#[test]
fn fts_zero_pad_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format-time-string "%H%M%S" '(26150 29968) t) (format-time-string "%m/%d" '(26150 29968) t) (format-time-string "%y" '(26150 29968) t) (format-time-string "%C" '(26150 29968) t))"##,
    );
}

#[test]
fn and_or_let_star() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (and-let* ((x 5) ((> x 3)) (y (* x 2))) y)
        (and-let* ((x 5) ((< x 3))) 'never))"##,
    );
}

#[test]
fn hash_table_keys_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'subr-x)
(let ((h (make-hash-table :test 'equal)))
  (puthash "a" 1 h) (puthash "b" 2 h)
  (list (sort (hash-table-keys h) #'string<) (sort (hash-table-values h) #'<)))"##,
    );
}

#[test]
fn if_when_let() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'subr-x)
(list (if-let ((x 5) (y 10)) (+ x y) 'no)
      (if-let ((x nil)) 'yes 'no)
      (when-let ((a 1) (b 2)) (+ a b))
      (when-let ((a nil)) 'never))"##,
    );
}

#[test]
fn named_let() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(named-let loop ((n 5) (acc 1)) (if (= n 0) acc (loop (1- n) (* acc n))))"##,
    );
}

#[test]
fn string_subr_x() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'subr-x)
(list (string-trim "  hi  ") (string-empty-p "") (string-empty-p "x")
      (string-blank-p "  ") (string-join '("a" "b") ",") (string-pad "x" 3))"##,
    );
}

#[test]
fn thread_macros() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'subr-x)
(list (thread-first 5 (+ 3) (* 2)) (thread-last 5 (+ 3) (* 2))
      (thread-first '(1 2 3) (car) (number-to-string)))"##,
    );
}
