use std::time::Duration;

use crate::{AC_HTML_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod workflows;

const AC_HTML_TEST_TIMEOUT: Duration = Duration::from_secs(180);

/// ac-html is an auto-complete source backed by the data files the package
/// ships under `completion-data/', so every workflow sets the buffer up the way
/// the package documents -- enable a data provider, call `ac-html-setup', put
/// its sources on `ac-sources' -- and then completes through `ac-start' /
/// `ac-update' / `ac-complete' in a window-displayed buffer.  Nothing is
/// stubbed; the candidate lists and the documentation both come off disk.
const AC_HTML_TEST_PRELUDE: &str = r####"
(require 'cl-lib)
(require 'auto-complete)
(require 'ac-html-default-data-provider)

(defmacro aht-test-in-buffer (&rest body)
  "Run BODY in a window-displayed `html-mode' buffer set up the way the
package's README documents: enable a data provider, call the language's
setup function, then put its sources on `ac-sources'."
  `(let ((buffer (generate-new-buffer "*ac-html-workflow*")))
     (unwind-protect
         (progn
           (set-window-buffer (selected-window) buffer)
           (set-buffer buffer)
           (html-mode)
           (ac-html-enable-data-provider 'ac-html-default-data-provider)
           (ac-html-setup)
           (setq ac-sources
                 '(ac-source-html-tag ac-source-html-attr ac-source-html-attrv))
           (auto-complete-mode 1)
           (setq aht-test-documented nil)
           ,@body)
       (kill-buffer buffer))))

(defun aht-test-candidates ()
  (ac-start :force-init t)
  (ac-update t)
  (mapcar #'substring-no-properties ac-candidates))

(defun aht-test-offer (text)
  "Retype the buffer as TEXT, record what auto-complete offers, then abort."
  (erase-buffer)
  (insert text)
  (let* ((candidates (aht-test-candidates))
         (prefix ac-prefix)
         (symbols (delete-dups
                   (mapcar (lambda (item) (get-text-property 0 'symbol item))
                           ac-candidates))))
    (ac-abort)
    (list :typed text :prefix prefix :count (length candidates)
          :symbols symbols :candidates candidates)))

(defvar aht-test-documented nil)

(defun aht-test-documentation (item)
  (let ((doc (popup-item-documentation item)))
    (and doc (substring-no-properties doc))))

(defun aht-test-offer-with-docs (text)
  "Like `aht-test-offer', but also read each candidate's documentation."
  (erase-buffer)
  (insert text)
  (let* ((candidates (aht-test-candidates))
         (prefix ac-prefix)
         (docs (mapcar (lambda (item)
                         (list (substring-no-properties item)
                               (get-text-property 0 'symbol item)
                               (aht-test-documentation item)))
                       ac-candidates)))
    (ac-abort)
    (list :typed text :prefix prefix :candidates candidates :documentation docs)))
"####;

fn ac_html_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_HTML_MELPA_PIN, "ac-html.el")
        .expect("prepare pinned ac-html source below ./tmp")
        .with_prelude(AC_HTML_TEST_PRELUDE)
        .with_timeout(AC_HTML_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-html parity test")
        .into()
}

pub(crate) fn assert_ac_html_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_html_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ac-html parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
