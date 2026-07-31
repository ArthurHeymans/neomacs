use std::time::Duration;

use crate::{ASX_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod dom;
mod query;
mod registry;
mod rendering;
mod request;
mod search;

const ASX_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ASX_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defvar helm-google-suggest-default-function nil)

;; ASX 20191024.1100 requires `request' at load time but does not declare it
;; in Package-Requires.  Tests replace this seam explicitly and never perform
;; network I/O.
(unless
    (featurep 'request)
  (defvar request-curl-options nil)
  (defun request
      (&rest _arguments)
    (error
     "Unexpected unstubbed request call"))
  (provide 'request))

(defun asx-test-search-dom
    ()
  '(html nil
    (body nil
     (div
      ((class . "r"))
      (a
       ((href . "https://stackoverflow.com/questions/101/first"))
       (h3 nil "First " (em nil "result"))))
     (div
      ((class . "other"))
      (a
       ((href . "https://example.com/not-a-question"))
       (h3 nil "Ignored")))
     (div
      ((class . "r"))
      (a
       ((href . "https://emacs.stackexchange.com/questions/202/second"))
       (h3 nil "Second result")))
     (div
      ((class . "result "))
      (a
       ((class . "result__a"))
       "Duck "
       (strong nil "one"))
      (span
       ((class . "result__url"))
       " stackoverflow.com/questions/303/duck-one "))
     (div
      ((class . "result "))
      (a
       ((class . "result__a"))
       "Duck two")
      (span
       ((class . "result__url"))
       " example.com/articles/404 ")))))

(defun asx-test-post-dom
    ()
  '(html nil
    (body nil
     (a
      ((class . "question-hyperlink")
       (href . "/questions/101/first"))
      "How to "
      (code nil "mapcar")
      "?")
     (div
      ((id . "question"))
      (div
       ((class . "post-text"))
       (p nil "Question " (strong nil "body") ".")
       (pre
        ((class . "lang-emacs-lisp"))
        "(+ 1 2)"))
      (span
       ((class . "js-vote-count"))
       "12"))
     (div
      ((class . "post-taglist"))
      (a ((class . "post-tag")) "emacs")
      (a ((class . "post-tag")) "elisp"))
     (div
      ((id . "answer-1"))
      (div
       ((class . "answercell"))
       (div
        ((class . "post-text"))
        (p nil "First answer."))
       (span
        ((class . "js-vote-count"))
        "7")))
     (div
      ((id . "answer-2"))
      (div
       ((class . "answercell"))
       (div
        ((class . "post-text"))
        (p nil "Second " (a ((href . "https://example.com")) "answer") "."))
       (span
        ((class . "js-vote-count"))
        "-1"))))))

(defun asx-test-post-summary
    (post)
  (list
   :url
   (plist-get post :url)
   :title
   (plist-get post :title)
   :body
   (plist-get post :body)
   :score
   (plist-get post :score)
   :answers
   (plist-get post :answers)
   :tags
   (plist-get post :tags)))

(defun asx-test-rendered-buffer-summary
    ()
  (list
   (buffer-substring-no-properties
    (point-min)
    (point-max))
   major-mode
   visual-line-mode
   buffer-read-only
   (point)
   (buffer-modified-p)))
"##;

fn asx_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASX_MELPA_PIN, source_file)
        .expect("prepare pinned asx source below ./tmp")
        .with_prelude(ASX_TEST_PRELUDE)
        .with_timeout(ASX_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed asx parity test").into()
}

fn assert_asx_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = asx_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("asx parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_asx_parity(elisp_form: &str, expected: Expect) {
    assert_asx_source_parity("asx.el", elisp_form, expected);
}

pub(crate) fn assert_asx_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_asx_source_parity("asx-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_asx_autoload_parity` cases (2a).
pub(crate) fn assert_asx_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        asx_oracle("asx-autoloads.el"),
        &name,
        "asx_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_asx_parity` cases (2a).
pub(crate) fn assert_asx_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(asx_oracle("asx.el"), &name, "asx_parity", cases);
}
