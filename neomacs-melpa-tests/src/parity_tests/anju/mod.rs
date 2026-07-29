use std::time::Duration;

use crate::{ANJU_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod context_menu;
mod initialization;
mod mode_line;
mod registry;
mod style_text;
mod utils;
mod workflows;

const ANJU_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ANJU_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun anju-test-menu-entries (menu)
  (let (result)
    (map-keymap
     (lambda (event item)
       (when (eq (car-safe item) 'menu-item)
         (let ((definition (nth 2 item))
               (properties (nthcdr 3 item)))
           (push
            (list
             event
             (nth 1 item)
             (if (keymapp definition) '<submenu> definition)
             :enable (plist-get properties :enable)
             :visible (plist-get properties :visible)
             :style (plist-get properties :style)
             :selected (plist-get properties :selected)
             :help (plist-get properties :help))
            result))))
     menu)
    (nreverse result)))

(defun anju-test-menu-labels (menu)
  (mapcar #'cadr (anju-test-menu-entries menu)))

(defun anju-test-buffer (name mode directory)
  (let ((buffer (get-buffer-create name)))
    (with-current-buffer buffer
      (setq default-directory directory)
      (funcall mode))
    buffer))

(defun anju-test-kill-buffers (buffers)
  (mapc
   (lambda (buffer)
     (when (buffer-live-p buffer)
       (kill-buffer buffer)))
   buffers))
"##;

fn anju_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANJU_MELPA_PIN, source_file)
        .expect("prepare pinned anju source and dependencies below ./tmp")
        .with_prelude(ANJU_TEST_PRELUDE)
        .with_timeout(ANJU_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed anju parity test").into()
}

fn assert_anju_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anju_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anju parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anju_parity(elisp_form: &str, expected: Expect) {
    assert_anju_source_parity("anju.el", elisp_form, expected);
}

pub(crate) fn assert_anju_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anju_source_parity("anju-autoloads.el", elisp_form, expected);
}
