use std::time::Duration;

use crate::{ARVIEW_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod cleanup;
mod commands;
mod copying;
mod detection;
mod process;
mod registry;
mod viewing;

const ARVIEW_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ARVIEW_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun arview-test-path (filename)
  (expand-file-name
   filename
   (getenv
    "NEOMACS_TEST_SANDBOX_ROOT")))

(defun arview-test-write-file (path content)
  (make-directory
   (file-name-directory path)
   t)
  (with-temp-file path
    (insert content))
  path)

(defun arview-test-read-file (path)
  (with-temp-buffer
    (insert-file-contents-literally path)
    (buffer-string)))

(defun arview-test-create-tar
    (&optional archive-name)
  (let* ((source
          (arview-test-path
           "fixture-source"))
         (archive
          (arview-test-path
           (or archive-name
               "fixture.tar")))
         (log
          (get-buffer-create
           " *arview-test-tar-log*")))
    (make-directory
     (expand-file-name
      "nested"
      source)
     t)
    (arview-test-write-file
     (expand-file-name
      "alpha.txt"
      source)
     "alpha\nline two\n")
    (arview-test-write-file
     (expand-file-name
      "nested/bravo λ.txt"
      source)
     "bravo λ\n")
    (arview-test-write-file
     (expand-file-name
      "space name.txt"
      source)
     "space payload\n")
    (with-current-buffer log
      (erase-buffer))
    (let ((exit
           (process-file
            "tar"
            nil
            log
            nil
            "-cf"
            archive
            "-C"
            source
            ".")))
      (unless (zerop exit)
        (error
         "Fixture tar failed: %s"
         (with-current-buffer log
           (buffer-string)))))
    archive))

(defun arview-test-tree (directory)
  (mapcar
   (lambda (path)
     (list
      (file-relative-name
       path
       directory)
      (arview-test-read-file
       path)))
   (sort
    (directory-files-recursively
     directory
     ".*"
     nil
     nil)
    #'string<)))

(defun arview-test-kill-sandbox-buffers ()
  (let ((root
         (getenv
          "NEOMACS_TEST_SANDBOX_ROOT")))
    (dolist (buffer
             (buffer-list))
      (let ((directory
             (buffer-local-value
              'default-directory
              buffer)))
        (when
            (and
             (stringp directory)
             (string-prefix-p
              root
              directory))
          (with-current-buffer buffer
            (set-buffer-modified-p nil))
          (kill-buffer buffer))))))
"##;

fn arview_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARVIEW_MELPA_PIN, source_file)
        .expect("prepare pinned arview source below ./tmp")
        .with_prelude(ARVIEW_TEST_PRELUDE)
        .with_timeout(ARVIEW_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed arview parity test").into()
}

fn assert_arview_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arview_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arview parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arview_parity(elisp_form: &str, expected: Expect) {
    assert_arview_source_parity("arview.el", elisp_form, expected);
}

pub(crate) fn assert_arview_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_arview_source_parity("arview-autoloads.el", elisp_form, expected);
}
