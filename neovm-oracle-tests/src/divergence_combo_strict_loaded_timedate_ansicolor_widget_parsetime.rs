//! Strict combo oracle probes, batch 35: heavier loaded-library coverage via
//! assert_oracle_parity_with_load — time-date.el (date/time conversions),
//! ansi-color.el (ANSI escape application), wid-edit.el (widget create/get),
//! and parse-time.el (parse-time-string).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_h2_time_date_conversions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(let ((t0 (encode-time 0 0 0 1 1 2020 0)))
  (list (time-to-days t0)
        (time-to-seconds t0)
        (days-to-time (time-to-days t0))
        (date-to-time "2020-01-01 00:00:00")
        (float-time t0)
        (time-to-days (encode-time 0 0 0 1 1 1970 0))))
"##,
        &["calendar/time-date.el"],
    );
}

#[test]
fn div_h2_ansi_color_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(let ((s "\033[31mred\033[0m and \033[1;32mbold green\033[0m"))
  (list (ansi-color-apply s)
        (length (ansi-color-apply s))
        (ansi-color-filter-apply s)))
"##,
        &["ansi-color.el"],
    );
}

#[test]
fn div_h2_widget_create_and_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (let ((w (widget-create 'editable-field :format "%v" "default text"))
        (w2 (widget-create 'checkbox)))
    (list (widgetp w)
          (widget-type w)
          (widget-get w :value)
          (widgetp w2)
          (widget-type w2)
          (widget-apply w :value-get))))
"##,
        &["wid-edit.el"],
    );
}

#[test]
fn div_h2_parse_time_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (parse-time-string "2020-06-15 12:30:45")
      (parse-time-string "Mon, 15 Jun 2020 12:30:45 +0000")
      (parse-time-string "invalid junk"))
"##,
        &["calendar/parse-time.el"],
    );
}

#[test]
fn div_h2_time_date_arithmetic_fixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(let ((t0 (encode-time 30 0 0 1 1 2020 0)))
  (list (time-add t0 (days-to-time 30))
        (time-subtract t0 3600)
        (time-less-p t0 (time-add t0 1))
        (time-equal-p t0 t0)
        (float-time (days-to-time 1))))
"##,
        &["calendar/time-date.el"],
    );
}
