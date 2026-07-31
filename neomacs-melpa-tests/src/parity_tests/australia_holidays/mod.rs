use std::time::Duration;

use crate::{AUSTRALIA_HOLIDAYS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod integration;
mod national;
mod registry;
mod states;

const AUSTRALIA_HOLIDAYS_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUSTRALIA_HOLIDAYS_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'calendar)
(require 'holidays)

(defun australia-holidays-test-error
    (thunk)
  (condition-case error
      (list
       :ok
       (funcall thunk))
    (error
     (list
      :signal
      (car error)
      (cdr error)))))

(defun australia-holidays-test-between
    (holidays start end)
  (let ((calendar-holidays holidays))
    (sort
     (holiday-in-range
      (calendar-absolute-from-gregorian start)
      (calendar-absolute-from-gregorian end))
     #'calendar-date-compare)))

(defun australia-holidays-test-year
    (holidays year)
  (australia-holidays-test-between
   holidays
   (list 1 1 year)
   (list 12 31 year)))

(defun australia-holidays-test-year-by-symbol
    (symbol year)
  (australia-holidays-test-year
   (symbol-value symbol)
   year))

(defun australia-holidays-test-on-date
    (holidays date)
  (let ((calendar-holidays holidays))
    (calendar-check-holidays date)))
"##;

fn australia_holidays_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUSTRALIA_HOLIDAYS_MELPA_PIN, source_file)
        .expect("prepare pinned australia-holidays source below ./tmp")
        .with_prelude(AUSTRALIA_HOLIDAYS_TEST_PRELUDE)
        .with_timeout(AUSTRALIA_HOLIDAYS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed australia-holidays parity test")
        .into()
}

fn assert_australia_holidays_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = australia_holidays_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("australia-holidays parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_australia_holidays_parity(elisp_form: &str, expected: Expect) {
    assert_australia_holidays_source_parity("australia-holidays.el", elisp_form, expected);
}

pub(crate) fn assert_australia_holidays_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_australia_holidays_source_parity(
        "australia-holidays-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_australia_holidays_autoload_parity` cases (2a).
pub(crate) fn assert_australia_holidays_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        australia_holidays_oracle("australia-holidays-autoloads.el"),
        &name,
        "australia_holidays_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_australia_holidays_parity` cases (2a).
pub(crate) fn assert_australia_holidays_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        australia_holidays_oracle("australia-holidays.el"),
        &name,
        "australia_holidays_parity",
        cases,
    );
}
