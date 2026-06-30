//! Strict combo oracle probes, batch 221: time-string parsing. iso8601-parse
//! over date/datetime/duration, parse-time-string, and decode-time of a parsed
//! time under UTC.
//! Uses assert_oracle_parity_expect format.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_iso8601_parse_date_datetime() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'iso8601)
(list (iso8601-parse "2025-03-15T12:30:45Z")
      (iso8601-parse-date "2025-03-15")
      (iso8601-parse "2025-01-01T00:00:00Z")
      (iso8601-parse "1970-01-01T00:00:00Z")
      (iso8601-parse-duration "PT1H30M")
      (iso8601-parse "2025-12-31T23:59:59Z"))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_parse_time_string_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'parse-time)
(list (parse-time-string "2025-03-15 12:30:45")
      (parse-time-string "Mar 15 2025")
      (parse-time-string "2025-03-15")
      (parse-time-string "15:30")
      (condition-case err (parse-time-string "not a date") (error 'caught))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_decode_time_of_parsed_under_utc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'iso8601)
(require 'parse-time)
(let ((tz (getenv "TZ")))
  (unwind-protect
      (progn
        (set-time-zone-rule "UTC0")
        (list (decode-time (iso8601-parse "2025-03-15T12:30:45Z"))
              (decode-time (iso8601-parse "1970-01-01T00:00:00Z"))
              (format-time-string "%Y-%m-%d %H:%M:%S"
                                   (encode-time (iso8601-parse "2025-06-15T08:00:00Z")))))
    (set-time-zone-rule tz)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
