use std::time::Duration;

use crate::{AUTO_PACKAGE_UPDATE_MELPA_PIN, CachedMelpaOracle, DASH_MELPA_PIN};
use expect_test::Expect;

mod buffers;
mod install;
mod registry;
mod schedule;
mod selection;
mod updates;

const AUTO_PACKAGE_UPDATE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTO_PACKAGE_UPDATE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'package)
(require 'seq)

(defun auto-package-update-test-root (name)
  (let ((root
         (file-name-as-directory
          (expand-file-name
           name
           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))))
    (make-directory root t)
    root))

(defun auto-package-update-test-path (root name)
  (expand-file-name name root))

(defun auto-package-update-test-write (file contents)
  (make-directory (file-name-directory file) t)
  (with-temp-file file
    (insert contents))
  file)

(defun auto-package-update-test-read (file)
  (with-temp-buffer
    (insert-file-contents-literally file)
    (buffer-string)))

(defun auto-package-update-test-error (thunk)
  (condition-case error-data
      (list :value (funcall thunk))
    (error
     (list
      :signal
      (car error-data)
      (cdr error-data)))))

(defun auto-package-update-test-desc
    (name version &optional requirements directory archive)
  (package-desc-create
   :name name
   :version version
   :summary (format "Fixture %s" name)
   :reqs requirements
   :kind 'tar
   :archive (or archive "fixture")
   :dir directory))

(defun auto-package-update-test-kill-buffers (&rest names)
  (dolist (name names)
    (let ((buffer (get-buffer name)))
      (when buffer
        (with-current-buffer buffer
          (set-buffer-modified-p nil))
        (kill-buffer buffer)))))
"##;

fn auto_package_update_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_PACKAGE_UPDATE_MELPA_PIN, source_file)
        .expect("prepare pinned auto-package-update source below ./tmp")
        .with_melpa_dependency(DASH_MELPA_PIN)
        .expect("prepare pinned dash dependency below ./tmp")
        .with_prelude(AUTO_PACKAGE_UPDATE_TEST_PRELUDE)
        .with_timeout(AUTO_PACKAGE_UPDATE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-package-update parity test")
        .into()
}

fn assert_auto_package_update_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_package_update_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("auto-package-update parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_package_update_parity(elisp_form: &str, expected: Expect) {
    assert_auto_package_update_source_parity("auto-package-update.el", elisp_form, expected);
}

pub(crate) fn assert_auto_package_update_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_package_update_source_parity(
        "auto-package-update-autoloads.el",
        elisp_form,
        expected,
    );
}
