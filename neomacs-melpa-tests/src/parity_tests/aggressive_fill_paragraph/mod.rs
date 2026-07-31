use std::time::Duration;

use crate::{AGGRESSIVE_FILL_PARAGRAPH_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod editing;
mod lifecycle;
mod suppression;
mod workflows;

const AGGRESSIVE_FILL_PARAGRAPH_TEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Helpers shared by the typing workflows.
///
/// This package's entire product is buffer text, so every workflow types
/// through the real command loop and reads back what landed.  Two details are
/// load bearing and are centralised here rather than repeated:
///
/// * `execute-kbd-macro` only reaches the buffer of the *selected window*, so
///   `afp-test-open` shows the buffer instead of merely making it current.
/// * a bare position is a claim nothing can check, so `afp-test-where` reports
///   point beside the text of the line it names.
const AGGRESSIVE_FILL_PARAGRAPH_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun afp-test-open (mode column)
  "Show a fresh buffer in the selected window, in MODE, filled to COLUMN."
  (let ((buffer (generate-new-buffer "*afp-workflow*")))
    (set-window-buffer (selected-window) buffer)
    (set-buffer buffer)
    (funcall mode)
    (buffer-enable-undo)
    (setq fill-column column)
    buffer))

(defun afp-test-close (buffer)
  "Discard BUFFER without a save prompt."
  (when (buffer-live-p buffer)
    (with-current-buffer buffer
      (set-buffer-modified-p nil))
    (kill-buffer buffer)))

(defun afp-test-type (text)
  "Type TEXT one character at a time through the real command loop."
  (execute-kbd-macro (string-to-vector text)))

(defun afp-test-press (keys)
  "Press KEYS, written in `kbd' notation."
  (execute-kbd-macro (kbd keys)))

(defun afp-test-text ()
  "The whole buffer, which is what this package exists to produce."
  (copy-sequence
   (buffer-substring-no-properties (point-min) (point-max))))

(defun afp-test-where ()
  "Point, named by the text of the line it sits on.
A line number alone cannot notice that it is pointed at the wrong line."
  (list (point)
        (line-number-at-pos)
        (current-column)
        (copy-sequence
         (buffer-substring-no-properties
          (line-beginning-position)
          (line-end-position)))))
"##;

fn aggressive_fill_paragraph_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(
        AGGRESSIVE_FILL_PARAGRAPH_MELPA_PIN,
        "aggressive-fill-paragraph.el",
    )
    .expect("prepare pinned aggressive-fill-paragraph source below ./tmp")
    .with_prelude(AGGRESSIVE_FILL_PARAGRAPH_TEST_PRELUDE)
    .with_timeout(AGGRESSIVE_FILL_PARAGRAPH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aggressive-fill-paragraph parity test")
        .into()
}

pub(crate) fn assert_aggressive_fill_paragraph_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aggressive_fill_paragraph_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("aggressive-fill-paragraph parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_aggressive_fill_paragraph_parity` cases (2a).
pub(crate) fn assert_aggressive_fill_paragraph_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        aggressive_fill_paragraph_oracle(),
        &name,
        "aggressive_fill_paragraph_parity",
        cases,
    );
}
