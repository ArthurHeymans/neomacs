use std::time::Duration;

use crate::{ASYNC_HTTP_QUEUE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod registry;
mod responses;
mod scheduling;
mod state;
mod workflows;

const ASYNC_HTTP_QUEUE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ASYNC_HTTP_QUEUE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'json)
(require 'seq)
(require 'url)

(defun async-http-queue-test-error-data (thunk)
  (condition-case error-data
      (list :ok (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun async-http-queue-test-state
    (urls &optional max-concurrent timeout parser
          completion-callback error-callback)
  (async-http-queue--state-create
   :queue
   (mapcar
    (lambda (url)
      `((url . ,url)
        (status . pending)
        (data . nil)))
    urls)
   :active-workers 0
   :max-concurrent (or max-concurrent 5)
   :timeout (or timeout 10)
   :parser
   (if (eq parser :default)
       #'json-parse-buffer
     parser)
   :completion-callback completion-callback
   :error-callback error-callback))

(defun async-http-queue-test-queue-snapshot (state)
  (mapcar
   (lambda (item)
     (list
      (alist-get 'url item)
      (alist-get 'status item)
      (alist-get 'data item)))
   (async-http-queue--state-queue state)))

(defun async-http-queue-test-state-snapshot (state)
  (list
   :queue
   (async-http-queue-test-queue-snapshot state)
   :active
   (async-http-queue--state-active-workers state)
   :limit
   (async-http-queue--state-max-concurrent state)
   :timeout
   (async-http-queue--state-timeout state)
   :parser
   (cond
    ((eq (async-http-queue--state-parser state)
         #'json-parse-buffer)
     'json-parse-buffer)
    ((null
      (async-http-queue--state-parser state))
     nil)
    (t :custom))
   :completion
   (and
    (async-http-queue--state-completion-callback state)
    t)
   :error
   (and
    (async-http-queue--state-error-callback state)
    t)))

(defun async-http-queue-test-http-response
    (status-code body &optional line-ending reason)
  (let ((newline (or line-ending "\r\n")))
    (concat
     (format
      "HTTP/1.1 %d %s"
      status-code
      (or reason "Test"))
     newline
     "Content-Type: application/json"
     newline
     "X-Test: deterministic"
     newline
     newline
     body)))

(defun async-http-queue-test-response-buffer
    (name response)
  (let ((buffer
         (generate-new-buffer
          (concat " *async-http-queue-test-" name "*"))))
    (with-current-buffer buffer
      (insert response))
    buffer))

(defun async-http-queue-test-run-timer-event (event)
  (unless (aref event 6)
    (apply
     (aref event 4)
     (aref event 5))))

(defun async-http-queue-test-timer-summary (events)
  (mapcar
   (lambda (event)
     (list
      (aref event 1)
      (aref event 2)
      (aref event 3)
      (aref event 6)))
   events))

(defun async-http-queue-test-kill-buffer (buffer)
  (when (buffer-live-p buffer)
    (kill-buffer buffer)))
"##;

fn async_http_queue_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASYNC_HTTP_QUEUE_MELPA_PIN, source_file)
        .expect("prepare pinned async-http-queue source below ./tmp")
        .with_prelude(ASYNC_HTTP_QUEUE_TEST_PRELUDE)
        .with_timeout(ASYNC_HTTP_QUEUE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed async-http-queue parity test")
        .into()
}

fn assert_async_http_queue_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = async_http_queue_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("async-http-queue parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_async_http_queue_parity(elisp_form: &str, expected: Expect) {
    assert_async_http_queue_source_parity("async-http-queue.el", elisp_form, expected);
}

pub(crate) fn assert_async_http_queue_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_async_http_queue_source_parity("async-http-queue-autoloads.el", elisp_form, expected);
}



/// Multi-probe batch for `assert_async_http_queue_autoload_parity` cases (2a).
pub(crate) fn assert_async_http_queue_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_http_queue_oracle("async-http-queue-autoloads.el"),
        &name,
        "async_http_queue_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_async_http_queue_parity` cases (2a).
pub(crate) fn assert_async_http_queue_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_http_queue_oracle("async-http-queue.el"),
        &name,
        "async_http_queue_parity",
        cases,
    );
}
