use std::time::Duration;

use crate::{ANYINS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod practical;

const ANYINS_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ANYINS_TEST_PRELUDE: &str = r##"
(defun neomacs-anyins-highlight-count ()
  (let ((cursor (point-min))
        (count 0))
    (while (< cursor (point-max))
      (when
          (eq
           (get-char-property cursor 'face)
           'anyins-recorded-positions)
        (setq count (1+ count)))
      (setq cursor (1+ cursor)))
    count))

(defun neomacs-anyins-marker-state (points)
  (list
   :points points
   :faces
   (mapcar
    (lambda (point)
      (get-char-property point 'face))
    points)
   :highlight-count
   (neomacs-anyins-highlight-count)
   :read-only buffer-read-only))
"##;

fn anyins_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANYINS_MELPA_PIN, "anyins.el")
        .expect("prepare pinned anyins source below ./tmp")
        .with_prelude(ANYINS_TEST_PRELUDE)
        .with_timeout(ANYINS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed anyins parity test").into()
}

pub(crate) fn assert_anyins_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anyins_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anyins parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_anyins_parity` cases (2a).
pub(crate) fn assert_anyins_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(anyins_oracle(), &name, "anyins_parity", cases);
}
