use std::time::Duration;

use crate::{CachedMelpaOracle, MULTI_TERM_MELPA_PIN};

use super::batch_support::{ParityBatchCase, assert_oracle_batch_cases};

mod workflows;

const MULTI_TERM_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const MULTI_TERM_TEST_PRELUDE: &str = r####"
(require 'cl-lib)
(require 'subr-x)
(require 'term)
(require 'multi-term)

(defun neomacs-multi-term-test-make-term (name program &rest _)
  "Create a fake terminal buffer named NAME without a live process."
  (let ((buffer (get-buffer-create (format "*%s*" name))))
    (with-current-buffer buffer
      (erase-buffer)
      (insert (format "fake-term:%s:%s\n" name program))
      (setq major-mode 'term-mode)
      (setq-local multi-term-fake t))
    buffer))

(defun neomacs-multi-term-test-with-fakes (function)
  "Run FUNCTION with make-term and multi-term-internal replaced by fakes."
  (let ((multi-term-buffer-list nil)
        (multi-term-dedicated-buffer nil)
        (multi-term-dedicated-window nil)
        (multi-term-try-create nil)
        (multi-term-program "/bin/sh")
        (created nil))
    (cl-letf (((symbol-function 'make-term)
               (lambda (name program &rest args)
                 (let ((buffer (apply #'neomacs-multi-term-test-make-term
                                      name program args)))
                   (push (buffer-name buffer) created)
                   buffer)))
              ((symbol-function 'multi-term-internal)
               (lambda ()
                 (setq major-mode 'term-mode)
                 (setq-local multi-term-internal-ran t)))
              ((symbol-function 'term-mode) #'ignore)
              ((symbol-function 'term-char-mode) #'ignore)
              ((symbol-function 'term-check-proc) (lambda (&rest _) nil))
              ((symbol-function 'term-quit-subjob) #'ignore))
      (unwind-protect
          (funcall function)
        (dolist (name created)
          (when (get-buffer name) (kill-buffer name)))
        (setq multi-term-buffer-list nil
              multi-term-dedicated-buffer nil
              multi-term-dedicated-window nil)))))

(defun neomacs-multi-term-test-names ()
  "Return multi-term buffer names in list order."
  (mapcar #'buffer-name multi-term-buffer-list))
"####;

fn multi_term_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(MULTI_TERM_MELPA_PIN, "multi-term.el")
        .expect("prepare exact shallow multi-term source below ./tmp")
        .with_prelude(MULTI_TERM_TEST_PRELUDE)
        .with_timeout(MULTI_TERM_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    std::thread::current()
        .name()
        .unwrap_or("unnamed multi-term parity test")
        .into()
}

fn assert_multi_term_batch(cases: &[ParityBatchCase]) {
    assert_oracle_batch_cases(
        multi_term_oracle(),
        &current_test_name(),
        "multi_term_parity",
        cases,
    );
}

#[test]
fn multi_term_package_batch() {
    assert_multi_term_batch(&workflows::workflow_batch_cases());
}
