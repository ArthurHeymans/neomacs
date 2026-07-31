use std::time::Duration;

use crate::{AUDIO_NOTES_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod filesystem;
mod lifecycle;
mod playback;
mod process;
mod registry;

const AUDIO_NOTES_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUDIO_NOTES_MODE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun audio-notes-test-error
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

(defun audio-notes-test-warning
    (thunk)
  (let (warnings)
    (cl-letf
        (((symbol-function 'display-warning)
          (lambda
            (type message &optional level buffer-name)
            (push
             (list type message level buffer-name)
             warnings))))
      (list
       (funcall thunk)
       (nreverse warnings)))))

(defun audio-notes-test-directory
    (name)
  (let ((directory
         (expand-file-name
          (concat name "/")
          default-directory)))
    (make-directory directory t)
    directory))

(defun audio-notes-test-write
    (directory name contents)
  (let ((path
         (expand-file-name
          name
          directory)))
    (with-temp-file path
      (insert contents))
    path))

(defun audio-notes-test-face-property
    (string)
  (list
   (substring-no-properties string)
   (get-text-property 0 'face string)))
"##;

fn audio_notes_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUDIO_NOTES_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned audio-notes-mode source below ./tmp")
        .with_prelude(AUDIO_NOTES_MODE_TEST_PRELUDE)
        .with_timeout(AUDIO_NOTES_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed audio-notes-mode parity test")
        .into()
}

fn assert_audio_notes_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = audio_notes_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("audio-notes-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_audio_notes_mode_parity(elisp_form: &str, expected: Expect) {
    assert_audio_notes_mode_source_parity("audio-notes-mode.el", elisp_form, expected);
}

pub(crate) fn assert_audio_notes_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_audio_notes_mode_source_parity("audio-notes-mode-autoloads.el", elisp_form, expected);
}





/// Multi-probe batch for `assert_audio_notes_mode_autoload_parity` cases (2a).
pub(crate) fn assert_audio_notes_mode_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        audio_notes_mode_oracle("audio-notes-mode-autoloads.el"),
        &name,
        "audio_notes_mode_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_audio_notes_mode_parity` cases (2a).
pub(crate) fn assert_audio_notes_mode_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        audio_notes_mode_oracle("audio-notes-mode.el"),
        &name,
        "audio_notes_mode_parity",
        cases,
    );
}
