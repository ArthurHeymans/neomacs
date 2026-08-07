use std::time::Duration;

use crate::{COMPAT_GNU_ELPA_PIN, CachedMelpaOracle, VERTICO_MELPA_PIN};

use super::batch_support::{ParityBatchCase, assert_oracle_batch_cases};

mod workflows;

const VERTICO_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const VERTICO_TEST_PRELUDE: &str = r####"
(require 'cl-lib)
(require 'subr-x)
(require 'vertico)

(defun neomacs-vertico-test-with-session (function)
  "Run FUNCTION with a minimal Vertico minibuffer session state."
  (let ((vertico--input (cons "" nil))
        (vertico--candidates '("apple" "apricot" "banana" "berry" "cherry"))
        (vertico--base "")
        (vertico--total 5)
        (vertico--index 0)
        (vertico--lock-candidate nil)
        (vertico--lock-groups nil)
        (vertico--groups nil)
        (vertico--all-groups nil)
        (vertico--history-hash nil)
        (vertico--metadata nil)
        (vertico-count 3)
        (vertico-scroll-margin 1)
        (vertico-cycle t))
    (funcall function)))
"####;

fn vertico_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(VERTICO_MELPA_PIN, "vertico.el")
        .expect("prepare exact shallow Vertico source below ./tmp")
        .with_gnu_elpa_dependency(COMPAT_GNU_ELPA_PIN)
        .expect("prepare exact shallow compat dependency below ./tmp")
        .with_prelude(VERTICO_TEST_PRELUDE)
        .with_timeout(VERTICO_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    std::thread::current()
        .name()
        .unwrap_or("unnamed vertico parity test")
        .into()
}

fn assert_vertico_batch(cases: &[ParityBatchCase]) {
    assert_oracle_batch_cases(
        vertico_oracle(),
        &current_test_name(),
        "vertico_parity",
        cases,
    );
}

#[test]
fn vertico_package_batch() {
    assert_vertico_batch(&workflows::workflow_batch_cases());
}
