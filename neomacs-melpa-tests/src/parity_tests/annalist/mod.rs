use std::time::Duration;

use crate::{ANNALIST_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod helpers;
mod keybindings;
mod recording;
mod registry;
mod views;

const ANNALIST_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ANNALIST_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun annalist-test-define-deployments
    (&optional table-start-index primary-key extra-settings)
  (annalist-define-tome
      'deployments
    (append
     (list
      :primary-key (or primary-key '(environment service))
      :table-start-index (or table-start-index 0))
     extra-settings
     '(environment service version status owner)))
  (annalist-define-view
      'deployments
      'default
    '((environment :title "Environment")
      (service :title "Service")
      (version :title "Version")
      (status :title "Status")
      (owner :title "Owner")))
  'deployments)

(defun annalist-test-record-deployments ()
  (dolist
      (record
       '(("production" "api" "2.4.0" "healthy" "platform")
         ("staging" "worker" "2.5.0-rc1" "deploying" "runtime")
         ("production" "frontend" "8.1.2" "healthy" "web")
         ("development" "api" "2.6.0-dev" "degraded" "alice")))
    (annalist-record 'operations 'deployments record))
  'recorded)

(defun annalist-test-description
    (annalist type &optional view)
  (annalist-describe annalist type view)
  (let ((buffer
         (get-buffer
          (format "*%s %s*" annalist type))))
    (when buffer
      (with-current-buffer buffer
        (list
         major-mode
         buffer-read-only
         (point-min)
         (point-max)
         (buffer-substring-no-properties
          (point-min)
          (point-max)))))))

(defun annalist-test-keybinding-records
    (annalist)
  (let ((store
         (gethash
          annalist
          (annalist--tome 'keybindings)))
        records)
    (dolist
        (keymap
         (and store
              (annalist--hash-table-keys store)))
      (let ((state-store
             (gethash keymap store)))
        (dolist
            (state
             (annalist--hash-table-keys state-store))
          (setq records
                (append
                 records
                 (annalist--hash-table-values
                  (gethash state state-store)))))))
    records))

(defun annalist-test-reset ()
  (setq annalist--tomes nil
        annalist--local-tomes nil
        annalist--tomes-settings nil
        annalist--tomes-views
        (make-hash-table :test #'equal))
  'reset)
"##;

fn annalist_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANNALIST_MELPA_PIN, source_file)
        .expect("prepare pinned annalist source below ./tmp")
        .with_prelude(ANNALIST_TEST_PRELUDE)
        .with_timeout(ANNALIST_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed annalist parity test")
        .into()
}

fn assert_annalist_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = annalist_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("annalist parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_annalist_parity(elisp_form: &str, expected: Expect) {
    assert_annalist_source_parity("annalist.el", elisp_form, expected);
}

pub(crate) fn assert_annalist_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_annalist_source_parity("annalist-autoloads.el", elisp_form, expected);
}
