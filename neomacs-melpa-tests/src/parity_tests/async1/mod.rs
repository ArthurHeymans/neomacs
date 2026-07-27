use std::time::Duration;

use crate::{ASYNC1_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod construction;
mod pipelines;
mod plist;
mod registry;
mod timers;

const ASYNC1_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ASYNC1_ARCHIVE_URL: &str = "https://melpa.org/packages/async1-20260421.2116.tar";
const ASYNC1_ARCHIVE_SHA256: &str =
    "fef17778d235913a76e9f5341a0cfb7c9dc96c7ccd22103ba5952e6fd936c2f2";
const ASYNC1_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

;; A deterministic virtual timer queue exercises the package's real callback
;; topology without wall-clock races.  Dedicated timer tests below still use
;; GNU Emacs/Neomacs `run-at-time' and their actual event loops.
(defvar async1-test-now 0)
(defvar async1-test-next-id 0)
(defvar async1-test-timer-queue nil)

(defun async1-test-reset-scheduler
    ()
  (setq async1-test-now 0
        async1-test-next-id 0
        async1-test-timer-queue nil))

(defun async1-test-schedule
    (delay repeat function &rest arguments)
  (let* ((id
          (setq async1-test-next-id
                (1+ async1-test-next-id)))
         (due
          (+ async1-test-now delay))
         (event
          (list due id repeat function arguments)))
    (push event async1-test-timer-queue)
    (list :async1-test-timer id)))

(defun async1-test-event-before-p
    (left right)
  (or
   (< (nth 0 left)
      (nth 0 right))
   (and
    (= (nth 0 left)
       (nth 0 right))
    (< (nth 1 left)
       (nth 1 right)))))

(defun async1-test-drain
    ()
  (let (trace)
    (while async1-test-timer-queue
      (setq async1-test-timer-queue
            (sort async1-test-timer-queue
                  #'async1-test-event-before-p))
      (let* ((event
              (pop async1-test-timer-queue))
             (due
              (nth 0 event))
             (id
              (nth 1 event))
             (repeat
              (nth 2 event))
             (function
              (nth 3 event))
             (arguments
              (nth 4 event)))
        (setq async1-test-now due)
        (push
         (list
          :at due
          :id id
          :repeat repeat
          :function
          (if
              (symbolp function)
              function
            :closure)
          :arguments arguments)
         trace)
        (apply function arguments)))
    (nreverse trace)))

(defun async1-test-await
    (predicate timeout)
  (let ((deadline
         (+ (float-time)
            timeout)))
    (while
        (and
         (not
          (funcall predicate))
         (< (float-time)
            deadline))
      (sit-for 0.005))
    (funcall predicate)))

(defun async1-test-error
    (thunk)
  (condition-case error
      (list :ok
            (funcall thunk))
    (error
     (list
      :error
      (car error)
      (cdr error)))))
"##;

fn async1_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new_from_frozen_melpa_archive(
        ASYNC1_MELPA_PIN,
        source_file,
        ASYNC1_ARCHIVE_URL,
        ASYNC1_ARCHIVE_SHA256,
    )
    .expect("prepare digest-verified async1 source below ./tmp")
    .with_prelude(ASYNC1_TEST_PRELUDE)
    .with_timeout(ASYNC1_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed async1 parity test").into()
}

fn assert_async1_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = async1_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("async1 parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_async1_parity(elisp_form: &str, expected: Expect) {
    assert_async1_source_parity("async1.el", elisp_form, expected);
}

pub(crate) fn assert_async1_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_async1_source_parity("async1-autoloads.el", elisp_form, expected);
}
