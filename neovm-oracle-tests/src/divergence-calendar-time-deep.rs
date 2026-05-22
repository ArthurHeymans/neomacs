//! Divergence tests: calendar, diary, holidays, solar, lunar deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_calendar_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'calendar)
  (fboundp 'calendar-current-date)
  (featurep 'calendar))"#,
    );
}

#[test]
fn divergence_diary_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'diary)
  (fboundp 'diary-view-entries)
  (featurep 'diary-lib))"#,
    );
}

#[test]
fn divergence_holidays() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'list-holidays)
  (fboundp 'calendar-holiday-list)
  (featurep 'holidays))"#,
    );
}

#[test]
fn divergence_solar_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'sunrise-sunset)
  (featurep 'solar))"#,
    );
}

#[test]
fn divergence_lunar_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'lunar-phases)
  (featurep 'lunar))"#,
    );
}

#[test]
fn divergence_time_date_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'format-time-string)
  (fboundp 'decode-time)
  (fboundp 'encode-time)
  (fboundp 'current-time)
  (fboundp 'time-add)
  (fboundp 'time-subtract))"#,
    );
}

#[test]
fn divergence_decode_time_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((time (decode-time)))
  (list (listp time)
        (>= (length time) 9)
        (integerp (nth 0 time))
        (integerp (nth 1 time))
        (integerp (nth 2 time))
        (integerp (nth 3 time))
        (integerp (nth 4 time))
        (integerp (nth 5 time)))) "#,
    );
}

#[test]
fn divergence_encode_decode_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((encoded (encode-time 30 45 12 1 6 2025 nil -1 nil))
        (decoded (decode-time encoded)))
  (list (nth 0 decoded)
        (nth 1 decoded)
        (nth 2 decoded)
        (nth 3 decoded)
        (nth 4 decoded)
        (nth 5 decoded))) "#,
    );
}

#[test]
fn divergence_float_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'float-time)
  (floatp (float-time))
  (>= (float-time) 0)
  (fboundp 'seconds-to-time))"#,
    );
}

#[test]
fn divergence_time_conversion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((now (current-time)))
  (list (listp now)
        (time-add now 100)
        (time-subtract now 50)
        (time-equal-p now now)
        (time-less-p now (time-add now 1)))) "#,
    );
}
