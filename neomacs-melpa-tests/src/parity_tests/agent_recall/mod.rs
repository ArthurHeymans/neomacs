use std::time::Duration;

use crate::{AGENT_RECALL_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod interaction;
mod matching;
mod search;
mod smoke;
mod workflows;

const AGENT_RECALL_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn agent_recall_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGENT_RECALL_MELPA_PIN, source_file)
        .expect("prepare pinned agent-recall source and dependency transaction below ./tmp")
        .with_timeout(AGENT_RECALL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed agent-recall parity test")
        .into()
}

fn assert_agent_recall_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = agent_recall_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("agent-recall parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_agent_recall_parity(elisp_form: &str, expected: Expect) {
    assert_agent_recall_source_parity("agent-recall.el", elisp_form, expected);
}

pub(crate) fn assert_agent_recall_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_agent_recall_source_parity("agent-recall-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_agent_recall_autoload_parity` cases (2a).
pub(crate) fn assert_agent_recall_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        agent_recall_oracle("agent-recall-autoloads.el"),
        &name,
        "agent_recall_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_agent_recall_parity` cases (2a).
pub(crate) fn assert_agent_recall_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        agent_recall_oracle("agent-recall.el"),
        &name,
        "agent_recall_parity",
        cases,
    );
}
