//! Divergence tests: gcl, calendar, calc, org stubs, and mode-line.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_cal_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'calendar)
  (fboundp 'holiday-list)
  (featurep 'calendar))"#,
    );
}

#[test]
fn divergence_calc_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'calc)
  (featurep 'calc)
  (fboundp 'calc-eval))"#,
    );
}

#[test]
fn divergence_org_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'org-mode)
  (fboundp 'org-agenda)
  (featurep 'org))"#,
    );
}

#[test]
fn divergence_mode_line_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (stringp mode-name)
  (stringp mode-line-format)
  (consp mode-line-format)
  (boundp 'mode-line-process)
  (boundp 'minor-mode-alist))"#,
    );
}

#[test]
fn divergence_mode_line_modification() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (setq mode-name "TestMode")
  (list mode-name
        (buffer-name)
        (buffer-file-name)
        (buffer-modified-p)))"#,
    );
}

#[test]
fn divergence_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'header-line-format)
  (boundp 'mode-line-in-non-selected-windows)
  (booleanp mode-line-in-non-selected-windows))"#,
    );
}

#[test]
fn divergence_echo_area() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'message)
  (fboundp 'format-message)
  (fboundp 'minibuffer-message)
  (stringp (message "test %d" 42)))"#,
    );
}

#[test]
fn divergence_cursor_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'cursor-type)
  (boundp 'blink-cursor-mode)
  (fboundp 'blink-cursor-mode))"#,
    );
}

#[test]
fn divergence_buffer_display_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'truncate-lines)
  (booleanp truncate-lines)
  (boundp 'word-wrap)
  (booleanp word-wrap)
  (boundp 'tab-width)
  (integerp tab-width))"#,
    );
}

#[test]
fn divergence_buffer_display_tables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'standard-display-table)
  (boundp 'buffer-display-table)
  (char-table-p (standard-display-table))
  (or (null buffer-display-table)
      (char-table-p buffer-display-table)))"#,
    );
}
