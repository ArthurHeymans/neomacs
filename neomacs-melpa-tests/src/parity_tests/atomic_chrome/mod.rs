use std::time::Duration;

use crate::{ATOMIC_CHROME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod buffers;
mod httpd;
mod messaging;
mod protocol;
mod registry;
mod servers;
mod tables;
mod workflows;

const ATOMIC_CHROME_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ATOMIC_CHROME_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun atomic-chrome-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun atomic-chrome-test-buffer-table-snapshot ()
  (let (rows)
    (maphash
     (lambda (buffer value)
       (let ((socket
              (nth 0 value))
             (frame
              (nth 1 value)))
       (push
        (list
         (if
             (bufferp buffer)
             (buffer-name buffer)
           buffer)
         (if
             (websocket-p socket)
             (list
              (websocket-client-data socket)
              (websocket-server-conn socket))
           socket)
         frame)
        rows)))
     atomic-chrome-buffer-table)
    (sort
     rows
     (lambda (left right)
       (string<
        (format "%s" (car left))
        (format "%s" (car right)))))))

(defun atomic-chrome-test-socket (name server)
  (websocket-inner-create
   :url
   (format "ws://%s.test" name)
   :conn name
   :server-conn server
   :client-data name))

(defun atomic-chrome-test-socket-name (socket)
  (if
      (websocket-p socket)
      (websocket-client-data socket)
    socket))

(defun atomic-chrome-test-frame (payload)
  (make-websocket-frame
   :opcode 'text
   :payload payload
   :length (string-bytes payload)
   :completep t))

(defun atomic-chrome-test-kill-buffer (buffer)
  (when
      (buffer-live-p buffer)
    (with-current-buffer buffer
      (let ((kill-buffer-hook nil)
            (kill-buffer-query-functions nil))
        (kill-buffer buffer)))))

(defun atomic-chrome-test-buffer-state (buffer)
  (with-current-buffer buffer
    (list
     (buffer-name)
     (buffer-string)
     major-mode
     atomic-chrome-edit-mode
     (and
      (memq
       'atomic-chrome-close-connection
       kill-buffer-hook)
      t)
     (and
      (memq
       'atomic-chrome-send-buffer-text
       post-command-hook)
      t)
     (buffer-modified-p))))
"##;

fn atomic_chrome_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ATOMIC_CHROME_MELPA_PIN, source_file)
        .expect("prepare pinned atomic-chrome source below ./tmp")
        .with_prelude(ATOMIC_CHROME_TEST_PRELUDE)
        .with_timeout(ATOMIC_CHROME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed atomic-chrome parity test")
        .into()
}

fn assert_atomic_chrome_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = atomic_chrome_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("atomic-chrome parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_atomic_chrome_parity(elisp_form: &str, expected: Expect) {
    assert_atomic_chrome_source_parity("atomic-chrome.el", elisp_form, expected);
}

pub(crate) fn assert_atomic_chrome_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_atomic_chrome_source_parity("atomic-chrome-autoloads.el", elisp_form, expected);
}



/// Multi-probe batch for `assert_atomic_chrome_autoload_parity` cases (2a).
pub(crate) fn assert_atomic_chrome_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        atomic_chrome_oracle("atomic-chrome-autoloads.el"),
        &name,
        "atomic_chrome_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_atomic_chrome_parity` cases (2a).
pub(crate) fn assert_atomic_chrome_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        atomic_chrome_oracle("atomic-chrome.el"),
        &name,
        "atomic_chrome_parity",
        cases,
    );
}
