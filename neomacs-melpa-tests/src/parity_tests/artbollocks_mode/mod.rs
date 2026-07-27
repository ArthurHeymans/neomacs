use std::time::Duration;

use crate::{ARTBOLLOCKS_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod font_lock;
mod metrics;
mod mode;
mod regexes;
mod registry;
mod searching;

const ARTBOLLOCKS_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ARTBOLLOCKS_MODE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun artbollocks-test-face-runs ()
  (let ((position
         (point-min))
        result)
    (while
        (< position
           (point-max))
      (let* ((next
              (or
               (next-single-property-change
                position
                'face
                nil
                (point-max))
               (point-max)))
             (face
              (get-text-property
               position
               'face)))
        (when
            (or
             (memq
              face
              '(artbollocks-lexical-illusions-face
                artbollocks-passive-voice-face
                artbollocks-weasel-words-face
                artbollocks-face))
             (and
              (listp face)
              (seq-some
               (lambda (candidate)
                 (memq
                  candidate
                  '(artbollocks-lexical-illusions-face
                    artbollocks-passive-voice-face
                    artbollocks-weasel-words-face
                    artbollocks-face)))
               face)))
          (push
           (list
            (buffer-substring-no-properties
             position
             next)
            face)
           result))
        (setq position next)))
    (nreverse result)))

(defun artbollocks-test-match
    (search-function)
  (let (matches)
    (goto-char
     (point-min))
    (while
        (funcall
         search-function
         (point-max))
      (push
       (list
        (match-string-no-properties
         0)
        (mapcar
         (lambda (index)
           (match-string-no-properties
            index))
         (number-sequence
          1
          (1-
           (/ (length
               (match-data))
              2))))
        (match-beginning 0)
        (match-end 0)
        (point))
       matches))
    (nreverse matches)))
"##;

fn artbollocks_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARTBOLLOCKS_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned artbollocks-mode source below ./tmp")
        .with_prelude(ARTBOLLOCKS_MODE_TEST_PRELUDE)
        .with_timeout(ARTBOLLOCKS_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed artbollocks-mode parity test")
        .into()
}

fn assert_artbollocks_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = artbollocks_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("artbollocks-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_artbollocks_mode_parity(elisp_form: &str, expected: Expect) {
    assert_artbollocks_mode_source_parity("artbollocks-mode.el", elisp_form, expected);
}

pub(crate) fn assert_artbollocks_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_artbollocks_mode_source_parity("artbollocks-mode-autoloads.el", elisp_form, expected);
}
