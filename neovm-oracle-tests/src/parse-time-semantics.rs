//! Oracle parity tests for GNU `calendar/parse-time.el` parsing semantics.
//!
//! GNU `parse-time-string` first tries ISO 8601 parsing and then falls back to
//! a liberal token/rule parser.  These tests pin returned decoded-time fields,
//! tokenization, two-digit year rules, timezone parsing, and malformed input.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_parse_time_tokenize_and_rfc_dates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'parse-time)
  (list
   (parse-time-tokenize "Wed, 15 Jan 2020 16:12:21 -0800")
   (parse-time-string "Wed, 15 Jan 2020 16:12:21 -0800")
   (parse-time-string "Thu, 01 Jan 1970 00:00:00 GMT")
   (parse-time-string "Fri Nov 21 09:55:06 1997")
   (parse-time-string "21 Nov 97 09:55 EST")))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_parse_time_iso8601_variants_and_encoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'parse-time)
  (list
   (parse-time-string "2020-01-15T16:12:21-08:00")
   (parse-time-string "2020-01-15T16:12:21Z")
   (parse-time-string "20200115T161221Z")
   (format-time-string "%Y-%m-%d %H:%M:%S %z"
                       (parse-iso8601-time-string "2020-01-15T16:12:21Z") t)
   (format-time-string "%Y-%m-%d %H:%M:%S %z"
                       (parse-iso8601-time-string "2020-01-15T16:12:21-08:00") t)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_parse_time_two_digit_years_times_and_zones() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'parse-time)
  (list
   (parse-time-string "Jan 2 49 1:02")
   (parse-time-string "Jan 2 50 1:02")
   (parse-time-string "Jan 2 99 1:02:03")
   (parse-time-string "Jan 2 00 1:02:03")
   (parse-time-string "Jan 2 2020 1:02")
   (parse-time-string "Jan 2 2020 01:02:03 PDT")
   (parse-time-string "Jan 2 2020 01:02:03 +0530")
   (parse-time-string "Jan 2 2020 01:02:03 -0330")))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_parse_time_malformed_and_partial_inputs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'parse-time)
  (list
   (parse-time-string "")
   (parse-time-string "not a date")
   (parse-time-string "25:99")
   (parse-time-string "March 2020")
   (parse-time-string "2020-13-99")
   (condition-case err
       (parse-time-tokenize 42)
     (error (list (car err) (cadr err))))
   (condition-case err
       (parse-time-string 42)
     (error (list (car err) (cadr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
