use std::time::Duration;

use crate::{ACHIEVEMENTS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const ACHIEVEMENTS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

/// achievements watches which commands are actually run - through `keyfreq',
/// its recommended companion - and unlocks records that it persists to a file.
/// The workflows therefore type real keys with `execute-kbd-macro' into a
/// buffer displayed in the selected window, let the real `keyfreq-mode' record
/// them, and then use the package's own commands.  `keyfreq' counts the
/// *previous* command from `pre-command-hook', so each key sequence ends with
/// one extra command to flush the one before it.  The achievements file is
/// redirected into the per-case sandbox; nothing else is stubbed.
const ACHIEVEMENTS_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun ach-test-path (name)
  (expand-file-name name (getenv "NEOMACS_TEST_SANDBOX_ROOT")))

(defmacro ach-test-with-live-buffer (&rest body)
  "Run BODY in a real, window-displayed buffer so typed keys reach it."
  `(let ((buffer (generate-new-buffer "*achievements-workflow*")))
     (unwind-protect
         (progn
           (set-window-buffer (selected-window) buffer)
           (set-buffer buffer)
           ,@body)
       (kill-buffer buffer))))

(defun ach-test-earned ()
  "Names of every earned achievement, sorted."
  (sort (delq nil
              (mapcar (lambda (achievement)
                        (and (achievements-earned-p achievement)
                             (emacs-achievement-name achievement)))
                      achievements-list))
        #'string<))

(defun ach-test-record (name)
  "Return NAME's stored record, with its predicate reduced to a state."
  (let ((achievement (achievements-get-achievements-by-name name)))
    (and achievement
         (list (emacs-achievement-name achievement)
               (emacs-achievement-description achievement)
               (let ((predicate (emacs-achievement-predicate achievement)))
                 (cond ((eq predicate t) t)
                       ((null predicate) nil)
                       (t :pending)))
               (emacs-achievement-points achievement)
               (emacs-achievement-transient achievement)
               (achievements-earned-p achievement)))))

(defun ach-test-log ()
  "Text of the achievements log buffer, or a marker when there is none."
  (let ((buffer (get-buffer "*achievements-log*")))
    (if buffer
        (with-current-buffer buffer
          (buffer-substring-no-properties (point-min) (point-max)))
      'no-log-buffer)))

(defun ach-test-unlock-messages ()
  "Every ACHIEVEMENT UNLOCKED line the session produced, in order."
  (with-current-buffer (get-buffer-create "*Messages*")
    (let ((lines nil))
      (dolist (line (split-string (buffer-string) "\n" t) (nreverse lines))
        (when (string-prefix-p "ACHIEVEMENT UNLOCKED" line)
          (push line lines))))))

(defun ach-test-rows (&rest names)
  "Return (NAME . ROW) for each NAME in the *Achievements* buffer.
ROW is nil when the achievement is not listed at all."
  (with-current-buffer "*Achievements*"
    (mapcar
     (lambda (name)
       (cons name
             (save-excursion
               (goto-char (point-min))
               (and (re-search-forward (concat "^.*" (regexp-quote name)) nil t)
                    (buffer-substring-no-properties
                     (line-beginning-position) (line-end-position))))))
     names)))
"##;

fn achievements_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACHIEVEMENTS_MELPA_PIN, "achievements.el")
        .expect("prepare pinned achievements source below ./tmp")
        .with_prelude(ACHIEVEMENTS_TEST_PRELUDE)
        .with_timeout(ACHIEVEMENTS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed achievements parity test")
        .into()
}

pub(crate) fn assert_achievements_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = achievements_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("achievements parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_achievements_parity` cases (2a).
pub(crate) fn assert_achievements_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        achievements_oracle(),
        &name,
        "achievements_parity",
        cases,
    );
}
