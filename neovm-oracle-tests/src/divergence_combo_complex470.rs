/// Batch 470: rfc2047, rfc2231, time-date, format-spec, cookies, sha1, hexl.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx470_rfc2047_encode_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'rfc2047)
  (list (fboundp 'rfc2047-encode-string)
        (fboundp 'rfc2047-decode-string)))
"##,
    );
}

#[test]
fn div_cx470_rfc2231_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'rfc2231)
  (list (fboundp 'rfc2231-parse-param-value)
        (fboundp 'rfc2231-get-value)))
"##,
    );
}

#[test]
fn div_cx470_time_date_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'time-date)
  (list (date-to-day "2024-06-16")
        (date-to-time "2024-06-16 14:30:00")
        (time-to-days (encode-time 0 0 0 16 6 2024 nil))
        (time-to-day-in-year (encode-time 0 0 0 16 6 2024 nil))))
"##,
    );
}

#[test]
fn div_cx470_format_spec_modifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'format-spec)
  (let ((spec (format-spec-make ?a "hello" ?b "world" ?n 42)))
    (list (format-spec "%a %b" spec)
          (format-spec "%n" spec)
          (format-spec "%(one%)" spec)
          (format-spec "%a" (format-spec-make ?a (format-spec-make ?b "test"))))))
"##,
    );
}

#[test]
fn div_cx470_cookies_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cookie)
  (list (fboundp 'cookie) (fboundp 'cookie-handle-cookie-line)))
"##,
    );
}

#[test]
fn div_cx470_sha1_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (sha1 "hello")
      (secure-hash 'sha1 "hello")
      (secure-hash 'md5 "hello")
      (secure-hash 'sha256 "hello"))
"##,
    );
}

#[test]
fn div_cx470_hexl_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'hexl)
  (list (fboundp 'hexl-mode) (fboundp 'hexl-find-file)))
"##,
    );
}

#[test]
fn div_cx470_encode_time_with_decoded() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(encode-time (decode-time (encode-time 0 30 14 16 6 2024 nil)))
"##,
    );
}

#[test]
fn div_cx470_decoded_time_second() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((dt (decode-time (encode-time 0 0 0 16 6 2024 nil))))
  (list (decoded-time-year dt) (decoded-time-month dt)
        (decoded-time-day dt) (decoded-time-hour dt)
        (decoded-time-second dt)))
"##,
    );
}

#[test]
fn div_cx470_time_add_subtract_days() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((t1 (encode-time 0 0 0 1 1 2024 nil))
      (day (seconds-to-time 86400)))
  (list (time-less-p (time-add t1 day) t1)
        (time-less-p t1 (time-add t1 day))
        (time-equal-p t1 t1)))
"##,
    );
}

#[test]
fn div_cx470_time_zones() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (encode-time 0 0 12 16 6 2024 "UTC")
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx470_seconds_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (seconds-to-string 0)
      (seconds-to-string 60)
      (seconds-to-string 3600)
      (seconds-to-string 86400)
      (seconds-to-string 3661))
"##,
    );
}

#[test]
fn div_cx470_days_in_month() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(require 'calendar)
  (list (calendar-last-day-of-month 1 2024)
        (calendar-last-day-of-month 2 2024)
        (calendar-last-day-of-month 2 2023))
"##,
    );
}

#[test]
fn div_cx470_smtpmail_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'smtpmail)
  (list (boundp 'smtpmail-default-smtp-server)
        (boundp 'smtpmail-smtp-service)))
"##,
    );
}

#[test]
fn div_cx470_sasl_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'sasl)
  (list (fboundp 'sasl-find-mechanism)
        (boundp 'sasl-mechanisms)))
"##,
    );
}
