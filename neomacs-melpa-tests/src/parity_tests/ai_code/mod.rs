use std::time::Duration;

use crate::{AI_CODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod backends;
mod behaviors;
mod core;
mod links;
mod mcp;
mod prompts;
mod sessions;
mod viewport;
mod workflows;

// The editor-helper workflow starts several real pty processes, so the
// 30s this used to allow was marginal: the case passed when run alone and
// timed out under package load, which reads as flakiness in the package
// rather than as a harness cap.  The other long-running suites allow
// 120-240s.
const AI_CODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const AI_CODE_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'map)
(require 'seq)
"##;

fn ai_code_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AI_CODE_MELPA_PIN, source_file)
        .expect("prepare pinned ai-code source below ./tmp")
        .with_prelude(AI_CODE_PRELUDE)
        .with_timeout(AI_CODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ai-code parity test")
        .into()
}

fn assert_ai_code_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ai_code_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ai-code parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ai_code_parity(elisp_form: &str, expected: Expect) {
    assert_ai_code_source_parity("ai-code.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_ai_code_parity` cases (2a).
pub(crate) fn assert_ai_code_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        ai_code_oracle("ai-code.el"),
        &name,
        "ai_code_parity",
        cases,
    );
}
