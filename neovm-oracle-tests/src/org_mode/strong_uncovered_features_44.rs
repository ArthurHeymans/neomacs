//! Strong uncovered-features-44 oracle tests — org-timestamp, org-planning, org-schedule.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-timestamp-from-string
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_ts_from() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((ts (org-timestamp-from-string "<2026-01-15 Wed 10:30>")))
  (list (org-element-property :year-start ts)
        (org-element-property :month-start ts)
        (org-element-property :day-start ts)
        (org-element-property :hour-start ts)
        (org-element-property :minute-start ts)
        (org-element-property :dayofweek ts)))"##,
        expect_test::expect![[r#""OK (2026 1 15 10 30 nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-timestamp-format
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_ts_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((ts (org-timestamp-from-string "<2026-01-15 Wed 10:30>")))
  (org-timestamp-format ts "%Y-%m-%d %H:%M"))"##,
        expect_test::expect![[r#""OK \"2026-01-15 10:30\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-timestamp-to-time
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_ts_to_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((t (org-timestamp-to-time (org-timestamp-from-string "<2026-01-15 Wed>"))))
  (list (nth 0 t) (nth 1 t)))"##,
        expect_test::expect![[r#""ERR (setting-constant t)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-timestamp-up/down-day
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_ts_ud() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nSCHEDULED: <2026-01-15 Wed>")
  (goto-char (point-min))
  (search-forward "<2026")
  (backward-char 2)
  (let ((d1 (org-element-property :day-start (org-element-context))))
    (org-timestamp-up-day)
    (let ((d2 (org-element-property :day-start (org-element-context))))
      (org-timestamp-down-day)
      (let ((d3 (org-element-property :day-start (org-element-context))))
        (list d1 d2 d3)))))"##,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-schedule
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_schedule() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (org-schedule nil "<2026-01-15>")
  (buffer-string))"##,
        expect_test::expect![[r#""OK \"* T\nSCHEDULED: <2026-01-15 Thu>\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-deadline
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_deadline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (org-deadline nil "<2026-01-20>")
  (buffer-string))"##,
        expect_test::expect![[r#""OK \"* T\nDEADLINE: <2026-01-20 Tue>\"""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-time-stamp
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_ts_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-max))
  (org-time-stamp nil)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-time-stamp-inactive
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_ts_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-max))
  (org-time-stamp-inactive nil)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-get-repeat
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO W\nSCHEDULED: <2026-01-15 +1w>\n* TODO M\nDEADLINE: <2026-01-20 +1m>\n* TODO N")
  (goto-char (point-min))
  (let ((r1 (org-get-repeat)))
    (forward-line 2)
    (let ((r2 (org-get-repeat)))
      (forward-line 2)
      (list r1 r2 (org-get-repeat)))))"##,
        expect_test::expect![[r#""OK (\"+1w\" \"+1m\" nil)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-get-scheduled-time
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_sched_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nSCHEDULED: <2026-01-15 Wed>")
  (goto-char (point-min))
  (let ((t (org-get-scheduled-time nil)))
    (list (nth 0 t) (nth 1 t))))"##,
        expect_test::expect![[r#""ERR (setting-constant t)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-get-deadline-time
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_dead_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nDEADLINE: <2026-01-20 Mon>")
  (goto-char (point-min))
  (let ((t (org-get-deadline-time nil)))
    (list (nth 0 t) (nth 1 t))))"##,
        expect_test::expect![[r#""ERR (setting-constant t)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-parse-time-string
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_parse_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-parse-time-string "<2026-01-15 Wed 10:30>")
        (org-parse-time-string "[2026-01-20 Mon]")
        (org-parse-time-string "<2026-01-25>"))"##,
        expect_test::expect![[
            r#""OK ((0 30 10 15 1 2026 nil -1 nil) (0 0 0 20 1 2026 nil -1 nil) (0 0 0 25 1 2026 nil -1 nil))""#
        ]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-fix-decoded-time
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_fix_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-fix-decoded-time '(0 30 10 15 1 2026))
        (org-fix-decoded-time '(0 0 0 1 1 2026)))"##,
        expect_test::expect![[r#""OK ((0 30 10 15 1 2026) (0 0 0 1 1 2026))""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-time-stamp-to-now
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_ts_to_now() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(org-time-stamp-to-now "<2026-01-15>")"##,
        expect_test::expect![[r#""OK -165""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-days-to-iso-week
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_iso_week() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-days-to-iso-week 0)
        (org-days-to-iso-week 1)
        (org-days-to-iso-week 7))"##,
        expect_test::expect![[r#""OK (1 1 1)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-today
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_today() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(org-today)"##,
        expect_test::expect![[r#""OK 739796""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-current-time
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_current_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((t (org-current-time)))
  (list (nth 0 t) (nth 1 t)))"##,
        expect_test::expect![[r#""ERR (setting-constant t)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-float-year
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_float_year() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-float-year 2026)
        (org-float-year 2000)
        (org-float-year 1900))"##,
        expect_test::expect![[r#""ERR (void-function org-float-year)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-date-to-day
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_date_to_day() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-date-to-day "2026-01-15")
        (org-date-to-day "2026-06-01")
        (org-date-to-day "2026-12-31"))"##,
        expect_test::expect![[r#""ERR (void-function org-date-to-day)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-day-to-date
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_day_to_date() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-day-to-date (org-date-to-day "2026-01-15"))
        (org-day-to-date (org-date-to-day "2026-06-01")))"##,
        expect_test::expect![[r#""ERR (void-function org-day-to-date)""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-time-string-to-seconds
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_ts_to_sec() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-time-string-to-seconds "1:30")
        (org-time-string-to-seconds "0:45")
        (org-time-string-to-seconds "2:15:30"))"##,
        expect_test::expect![[r#""ERR (error \"Not an Org time string: 1:30\")""#]],
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-minutes-to-hh:mm-string
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf44_min_to_hhmm() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (org-minutes-to-hh:mm-string 90)
        (org-minutes-to-hh:mm-string 45)
        (org-minutes-to-hh:mm-string 150))"##,
        expect_test::expect![[r#""ERR (void-function org-minutes-to-hh:mm-string)""#]],
    );
}
