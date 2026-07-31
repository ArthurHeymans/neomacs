use std::time::Duration;

use crate::{ACE_LINK_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const ACE_LINK_TEST_TIMEOUT: Duration = Duration::from_secs(120);

/// ace-link labels every link visible in the selected window and follows the
/// one whose key the user presses, so every workflow needs a real buffer in the
/// selected window and real keys: avy reads its label key with `read-key',
/// which during `execute-kbd-macro' consumes the macro's remaining keys.
///
/// Nothing in ace-link or avy is stubbed.  The labels are observed the way a
/// user sees them -- the overlays carrying an avy lead face, read from
/// `avy-translate-char-function', avy's public hook for non-QWERTY layouts,
/// which runs while those overlays are still on screen.  The only test doubles
/// are the two browser functions, a true external boundary; both are declared
/// with a `browse-url-browser-kind' so `browse-url' routes to them instead of
/// launching a real browser.
const ACE_LINK_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defvar ace-link-test-keys nil
  "Each key avy read, with the labels visible at that moment.")

(defvar ace-link-test-browsed nil
  "Each URL handed to a browser, newest first.")

(defun ace-link-test-path (name)
  "Return the absolute sandbox path of NAME."
  (expand-file-name name (getenv "NEOMACS_TEST_SANDBOX_ROOT")))

(defun ace-link-test-write (name text)
  "Write TEXT to sandbox file NAME and return its path."
  (let ((path (ace-link-test-path name)))
    (make-directory (file-name-directory path) t)
    (with-temp-buffer
      (insert text)
      (write-region (point-min) (point-max) path nil 'silent))
    path))

(defconst ace-link-test-info-manual
  (concat "This is sandbox.info, produced by hand.\n"
          "\n\037\nFile: sandbox.info,  Node: Top,  Next: Basics,  Up: (dir)\n\n"
          "Sandbox Manual\n**************\n\n* Menu:\n\n"
          "* Basics::      How to begin.\n"
          "* Advanced::    Deeper water.\n"
          "\n\037\nFile: sandbox.info,  Node: Basics,  Next: Advanced,  Prev: Top,  Up: Top\n\n"
          "1 Basics\n========\n\nSee *note Advanced:: for the rest, or go *note Top::.\n"
          "\n\037\nFile: sandbox.info,  Node: Advanced,  Prev: Basics,  Up: Top\n\n"
          "2 Advanced\n==========\n\nBack to *note Basics::.\n")
  "A small hand-written manual with a menu and cross references.")

(defun ace-link-test-open-manual ()
  "Visit the sandbox Info manual in the selected window."
  (ace-link-test-write "manual/sandbox.info" ace-link-test-info-manual)
  (info (ace-link-test-path "manual/sandbox.info"))
  (set-window-buffer (selected-window) (current-buffer))
  (current-buffer))

(defun ace-link-test-labels ()
  "Return the avy labels a user can see right now.
Each entry is (OFFSET LABEL TEXT-AT-OFFSET)."
  (let (labels)
    (dolist (overlay (overlays-in (point-min) (point-max)))
      (let ((text (or (overlay-get overlay 'display)
                      (overlay-get overlay 'before-string)
                      (overlay-get overlay 'after-string))))
        (when (and (stringp text)
                   (> (length text) 0)
                   (memq (get-text-property 0 'face text)
                         '(avy-lead-face avy-lead-face-0)))
          (push (list (- (overlay-start overlay) (point-min))
                      (substring-no-properties text)
                      (save-excursion
                        (goto-char (overlay-start overlay))
                        (buffer-substring-no-properties (point)
                                                        (line-end-position))))
                labels))))
    (sort labels (lambda (a b) (< (car a) (car b))))))

(defun ace-link-test-record-key (char)
  "Record CHAR and the labels on screen, then return CHAR unchanged."
  (push (list (key-description (vector char)) (ace-link-test-labels))
        ace-link-test-keys)
  char)

(defun ace-link-test-pressed ()
  "Return the recorded keys in the order the user pressed them."
  (reverse ace-link-test-keys))

(defun ace-link-test-browser (url &rest _)
  "Record URL as opened in the primary browser."
  (push (list 'browse url) ace-link-test-browsed)
  'browsed)
(function-put 'ace-link-test-browser 'browse-url-browser-kind 'internal)

(defun ace-link-test-external-browser (url &rest _)
  "Record URL as opened in the secondary, external browser."
  (push (list 'browse-external url) ace-link-test-browsed)
  'browsed-externally)
(function-put 'ace-link-test-external-browser 'browse-url-browser-kind 'external)

(defun ace-link-test-capture-browsers ()
  "Route every browser call into `ace-link-test-browsed'."
  (require 'browse-url)
  (setq browse-url-browser-function #'ace-link-test-browser
        browse-url-secondary-browser-function #'ace-link-test-external-browser
        browse-url-handlers nil))

(defun ace-link-test-browsed ()
  "Return the recorded browser calls in call order."
  (reverse ace-link-test-browsed))

(defun ace-link-test-where ()
  "Report where the user ended up."
  (list :buffer (buffer-name)
        :window-buffer (buffer-name (window-buffer (selected-window)))
        :mode major-mode
        :point (- (point) (point-min))
        :line (line-number-at-pos)
        :column (current-column)
        :line-text (buffer-substring-no-properties
                    (line-beginning-position) (line-end-position))))

(defmacro ace-link-test-session (&rest body)
  "Run BODY with key and browser recording, then kill the buffers it made."
  `(let ((existing (buffer-list)))
     (setq ace-link-test-keys nil
           ace-link-test-browsed nil
           avy-translate-char-function #'ace-link-test-record-key)
     (unwind-protect
         (progn ,@body)
       (dolist (buffer (buffer-list))
         (unless (memq buffer existing)
           (with-current-buffer buffer
             (set-buffer-modified-p nil))
           (kill-buffer buffer))))))
"##;

fn ace_link_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_LINK_MELPA_PIN, "ace-link.el")
        .expect("prepare pinned ace-link source below ./tmp")
        .with_prelude(ACE_LINK_TEST_PRELUDE)
        .with_timeout(ACE_LINK_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-link parity test")
        .into()
}

pub(crate) fn assert_ace_link_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_link_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-link parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_ace_link_parity` cases (2a).
pub(crate) fn assert_ace_link_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(ace_link_oracle(), &name, "ace_link_parity", cases);
}
