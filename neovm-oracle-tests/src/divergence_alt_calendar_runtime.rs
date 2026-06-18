//! Alternative-calendar conversion parity (pure-Elisp algorithms over the core
//! engine): Islamic, Hebrew, Chinese, French-Revolutionary, Persian, Coptic/
//! Ethiopic, Bahai, Mayan long-count, astronomical Julian day, and ISO
//! from/to absolute date.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn astro_julian_day() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-julian)
  (list (calendar-astro-from-absolute 739052)
        (floor (calendar-astro-from-absolute 739052)))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn bahai() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-bahai)
  (list (calendar-bahai-from-absolute 739052))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn chinese() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-china)
  (list (calendar-chinese-from-absolute 739052))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn coptic_ethiopic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-coptic)
  (list (calendar-coptic-from-absolute 739052)
        (calendar-ethiopic-from-absolute 739052))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn french() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-french)
  (list (calendar-french-from-absolute 739052)
        (calendar-french-to-absolute '(1 1 233)))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn hebrew() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-hebrew)
  (list (calendar-hebrew-from-absolute 739052)
        (calendar-hebrew-to-absolute '(7 1 5785)))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn islamic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-islam)
  (list (calendar-islamic-from-absolute 739052)
        (calendar-islamic-to-absolute '(1 1 1446)))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn iso_calendar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-iso)
  (list (calendar-iso-from-absolute 739052)
        (calendar-iso-to-absolute '(25 6 2024)))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn mayan() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-mayan)
  (list (calendar-mayan-long-count-from-absolute 739052))) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn persian() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (progn (require 'cal-persia)
  (list (calendar-persian-from-absolute 739052)
        (calendar-persian-to-absolute '(1 1 1403)))) (error (cons (quote ERR) (car e))))"##,
    );
}
