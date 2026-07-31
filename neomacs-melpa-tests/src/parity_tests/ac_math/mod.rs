use std::time::Duration;

use crate::{AC_MATH_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

/// ac-math contributes three auto-complete sources for LaTeX buffers, so the
/// workflows complete through `ac-start` / `ac-update` / `ac-complete` in a
/// window-displayed buffer.  The package decides which source applies by
/// looking at the `face` text property at point, which is what font-latex puts
/// there, so the fixtures set that property directly rather than pulling in
/// AUCTeX.
const AC_MATH_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'auto-complete)

(defmacro ac-math-test-in-buffer (&rest body)
  "Run BODY in a window-displayed buffer with auto-complete armed."
  `(let ((buffer (generate-new-buffer "*ac-math-workflow*")))
     (unwind-protect
         (progn
           (set-window-buffer (selected-window) buffer)
           (set-buffer buffer)
           (setq ac-sources nil)
           (auto-complete-mode 1)
           ,@body)
       (kill-buffer buffer))))

(defun ac-math-test-math-region (start end)
  "Mark START..END as font-latex math, the way font-latex would."
  (put-text-property start end 'face 'font-latex-math-face))

(defun ac-math-test-candidates ()
  "Start completion at point and return the plain candidate strings."
  (ac-start :force-init t)
  (ac-update t)
  (mapcar #'substring-no-properties ac-candidates))

(defun ac-math-test-text ()
  (substring-no-properties (buffer-string)))
"##;

const AC_MATH_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_math_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_MATH_MELPA_PIN, "ac-math.el")
        .expect("prepare pinned ac-math source below ./tmp")
        .with_prelude(AC_MATH_TEST_PRELUDE)
        .with_timeout(AC_MATH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-math parity test")
        .into()
}

pub(crate) fn assert_ac_math_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_math_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-math parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_ac_math_parity` cases (2a).
pub(crate) fn assert_ac_math_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        ac_math_oracle(),
        &name,
        "ac_math_parity",
        cases,
    );
}
