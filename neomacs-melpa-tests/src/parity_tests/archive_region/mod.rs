use std::time::Duration;

use crate::{ARCHIVE_REGION_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod archiving;
mod dispatch;
mod headers;
mod opening;
mod paths;
mod registry;

const ARCHIVE_REGION_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ARCHIVE_REGION_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun archive-region-test-path
    (filename)
  (expand-file-name
   filename
   (getenv
    "NEOMACS_TEST_SANDBOX_ROOT")))

(defun archive-region-test-read-file
    (path)
  (with-temp-buffer
    (insert-file-contents-literally
     path)
    (buffer-string)))

(defun archive-region-test-kill-file-buffers ()
  (let ((root
         (getenv
          "NEOMACS_TEST_SANDBOX_ROOT")))
    (dolist (buffer (buffer-list))
      (when-let ((file
                  (buffer-local-value
                   'buffer-file-name
                   buffer)))
        (when (string-prefix-p
               root
               file)
          (with-current-buffer buffer
            (set-buffer-modified-p nil))
          (kill-buffer buffer))))))
"##;

fn archive_region_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARCHIVE_REGION_MELPA_PIN, source_file)
        .expect("prepare pinned archive-region source below ./tmp")
        .with_prelude(ARCHIVE_REGION_TEST_PRELUDE)
        .with_timeout(ARCHIVE_REGION_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed archive-region parity test")
        .into()
}

fn assert_archive_region_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = archive_region_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("archive-region parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_archive_region_parity(elisp_form: &str, expected: Expect) {
    assert_archive_region_source_parity("archive-region.el", elisp_form, expected);
}

pub(crate) fn assert_archive_region_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_archive_region_source_parity("archive-region-autoloads.el", elisp_form, expected);
}
