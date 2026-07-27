use std::time::Duration;

use crate::{ASDF_VM_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod config;
mod core;
mod installer;
mod mode;
mod plugin;
mod plugin_menu;
mod process;
mod registry;
mod tool_versions;
mod util;

const ASDF_VM_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ASDF_VM_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'subr-x)

(defun asdf-vm-test-path (filename)
  (expand-file-name
   filename
   (getenv
    "NEOMACS_TEST_SANDBOX_ROOT")))

(defun asdf-vm-test-write-file (path content)
  (make-directory
   (file-name-directory path)
   t)
  (with-temp-file path
    (insert content))
  path)

(defun asdf-vm-test-read-file (path)
  (with-temp-buffer
    (insert-file-contents-literally path)
    (buffer-string)))

(defun asdf-vm-test-tabulated-list-goto-id
    (id)
  (goto-char
   (point-min))
  (let (found)
    (while
        (and
         (not found)
         (not
          (eobp)))
      (when
          (equal
           (tabulated-list-get-id)
           id)
        (setq found t))
      (unless found
        (forward-line 1)))
    found))

(defun asdf-vm-test-make-executable
    (name body)
  (let ((path
         (asdf-vm-test-path
          (concat
           "bin/"
           name))))
    (asdf-vm-test-write-file
     path
     (concat
      "#!/bin/sh\n"
      "set -eu\n"
      body
      "\n"))
    (set-file-modes path #o755)
    path))

(defun asdf-vm-test-error-data
    (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))
"##;

fn asdf_vm_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASDF_VM_MELPA_PIN, source_file)
        .expect("prepare pinned asdf-vm source below ./tmp")
        .with_prelude(ASDF_VM_TEST_PRELUDE)
        .with_timeout(ASDF_VM_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed asdf-vm parity test")
        .into()
}

fn assert_asdf_vm_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = asdf_vm_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("asdf-vm parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_asdf_vm_parity(elisp_form: &str, expected: Expect) {
    assert_asdf_vm_source_parity("asdf-vm.el", elisp_form, expected);
}

pub(crate) fn assert_asdf_vm_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_asdf_vm_source_parity("asdf-vm-autoloads.el", elisp_form, expected);
}
