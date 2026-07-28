use std::time::Duration;

use crate::{ATTRAP_MELPA_PIN, CachedMelpaOracle, DASH_MELPA_PIN, F_MELPA_PIN, S_MELPA_PIN};
use expect_test::Expect;

mod dispatch;
mod elisp;
mod ghc;
mod hlint_latex;
mod options;
mod registry;

const ATTRAP_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ATTRAP_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun attrap-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun attrap-test-option-shape (options)
  (mapcar
   (lambda (option)
     (list
      (copy-tree
       (car option))
      (functionp
       (cdr option))))
   options))

(defun attrap-test-place-markers (contents)
  (insert contents)
  (goto-char
   (point-min))
  (unless
      (search-forward "«POINT»" nil t)
    (error
     "Fixture has no point marker: %S"
     contents))
  (replace-match "" t t)
  (let ((beg
         (point))
        end)
    (goto-char
     (point-min))
    (if
        (search-forward "«END»" nil t)
        (progn
          (replace-match "" t t)
          (setq end
                (point)))
      (setq end beg))
    (goto-char beg)
    (list beg end)))

(defun attrap-test-run-fixer-option
    (fixer message contents option-index)
  (with-temp-buffer
    (pcase-let
        ((`(,beg ,end)
          (attrap-test-place-markers
           contents)))
      (let* ((options
              (funcall fixer message beg end))
             (shape
              (and
               (listp options)
               (attrap-test-option-shape
                options)))
             (after-fixer
              (buffer-string))
             (application
              (when
                  (numberp option-index)
                (let ((option
                       (nth option-index options)))
                  (list
                   (attrap-test-error-data
                    (lambda ()
                      (funcall
                       (cdr option))))
                   (buffer-string)
                   (point))))))
        (list
         shape
         after-fixer
         application)))))
"##;

fn attrap_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ATTRAP_MELPA_PIN, source_file)
        .expect("prepare pinned attrap source and dependencies below ./tmp")
        .with_melpa_dependency(DASH_MELPA_PIN)
        .expect("prepare pinned dash dependency below ./tmp")
        .with_melpa_dependency(F_MELPA_PIN)
        .expect("prepare pinned f dependency below ./tmp")
        .with_melpa_dependency(S_MELPA_PIN)
        .expect("prepare pinned s dependency below ./tmp")
        .with_prelude(ATTRAP_TEST_PRELUDE)
        .with_timeout(ATTRAP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed attrap parity test").into()
}

fn assert_attrap_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = attrap_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("attrap parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_attrap_parity(elisp_form: &str, expected: Expect) {
    assert_attrap_source_parity("attrap.el", elisp_form, expected);
}

pub(crate) fn assert_attrap_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_attrap_source_parity("attrap-autoloads.el", elisp_form, expected);
}
