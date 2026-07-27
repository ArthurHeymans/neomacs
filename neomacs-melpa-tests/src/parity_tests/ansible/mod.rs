use std::time::Duration;

use crate::{ANSIBLE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod assets;
mod discovery;
mod mode;
mod registry;
mod vault;

const ANSIBLE_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ANSIBLE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun ansible-test-write-file
    (root relative content)
  (let ((path
         (expand-file-name relative root)))
    (make-directory
     (file-name-directory path)
     t)
    (with-temp-file path
      (insert content))
    path))

(defun ansible-test-make-project ()
  (let ((root
         (make-temp-file
          "ansible-parity-project-"
          t)))
    (make-directory
     (expand-file-name "roles" root)
     t)
    (ansible-test-write-file
     root
     "site.yml"
     "---\n- hosts: all\n")
    (ansible-test-write-file
     root
     "playbooks/deploy.yml"
     "---\n- hosts: production\n")
    (ansible-test-write-file
     root
     "playbooks/rollback.yml.backup"
     "---\n- hosts: production\n")
    (ansible-test-write-file
     root
     "playbooks/inventory.yaml"
     "all:\n  hosts:\n")
    (ansible-test-write-file
     root
     "notes/notayml.txt"
     "not a playbook\n")
    root))

(defun ansible-test-face-at
    (needle)
  (save-excursion
    (goto-char (point-min))
    (search-forward needle)
    (get-text-property
     (- (point) (length needle))
     'face)))

(defun ansible-test-read-file
    (path)
  (with-temp-buffer
    (insert-file-contents-literally path)
    (buffer-string)))
"##;

fn ansible_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANSIBLE_MELPA_PIN, source_file)
        .expect("prepare pinned ansible source below ./tmp")
        .with_prelude(ANSIBLE_TEST_PRELUDE)
        .with_timeout(ANSIBLE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ansible parity test")
        .into()
}

fn assert_ansible_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ansible_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ansible parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ansible_parity(elisp_form: &str, expected: Expect) {
    assert_ansible_source_parity("ansible.el", elisp_form, expected);
}

pub(crate) fn assert_ansible_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ansible_source_parity("ansible-autoloads.el", elisp_form, expected);
}
