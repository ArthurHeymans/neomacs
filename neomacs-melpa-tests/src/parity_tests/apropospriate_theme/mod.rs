use std::time::Duration;

use crate::{APROPOSPRIATE_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const APROPOSPRIATE_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const APROPOSPRIATE_THEME_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun apropospriate-test-face-at
    (token)
  (goto-char
   (point-min))
  (search-forward token)
  (or
   (get-text-property
    (match-beginning 0)
    'face)
   (get-text-property
    (match-beginning 0)
    'font-lock-face)))

(defun apropospriate-test-face-view
    (token)
  (let ((face
         (apropospriate-test-face-at
          token)))
    (list
     face
     (face-attribute
      face
      :foreground
      nil
      'default)
     (face-attribute
      face
      :background
      nil
      'default)
     (face-attribute
      face
      :weight
      nil
      'default))))

(defun apropospriate-test-load-color-theme
    (theme)
  (let ((original-frame-parameter
         (symbol-function
          'frame-parameter)))
    (cl-letf
        (((symbol-function
           'display-color-cells)
          (lambda
              (&optional _frame)
            16777216))
         ((symbol-function
           'frame-parameter)
          (lambda
              (frame parameter)
            (if
                (eq parameter
                    'display-type)
                'color
              (funcall
               original-frame-parameter
               frame
               parameter)))))
      (load-theme
       theme
       t))))

(defun apropospriate-test-disable-themes ()
  (mapc
   #'disable-theme
   (copy-sequence
    custom-enabled-themes)))
"##;

fn apropospriate_theme_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(APROPOSPRIATE_THEME_MELPA_PIN, "apropospriate-theme.el")
        .expect("prepare pinned apropospriate-theme source below ./tmp")
        .with_prelude(APROPOSPRIATE_THEME_TEST_PRELUDE)
        .with_timeout(APROPOSPRIATE_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apropospriate-theme parity test")
        .into()
}

pub(crate) fn assert_apropospriate_theme_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apropospriate_theme_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("apropospriate-theme parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_apropospriate_theme_parity` cases (2a).
pub(crate) fn assert_apropospriate_theme_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        apropospriate_theme_oracle(),
        &name,
        "apropospriate_theme_parity",
        cases,
    );
}
