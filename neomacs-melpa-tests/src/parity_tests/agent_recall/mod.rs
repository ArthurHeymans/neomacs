use std::time::Duration;

use crate::{AGENT_RECALL_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod backfill;
mod browse;
mod consult;
mod index;
mod search;
mod sessions;
mod surface;
mod tracking;
mod transcripts;

const AGENT_RECALL_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn agent_recall_oracle(source_file: &str) -> CachedMelpaOracle {
    let prelude = if source_file == "agent-recall-consult.el" {
        r##"(unless (require 'agent-shell nil t)
               (provide 'agent-shell))
             (unless (featurep 'agent-recall)
               (load
                (expand-file-name
                 "agent-recall.el"
                 (file-name-directory
                  (getenv "NEOMACS_PACKAGE_SOURCE")))
                nil t t))"##
    } else {
        "(unless (require 'agent-shell nil t) (provide 'agent-shell))"
    };
    CachedMelpaOracle::new(AGENT_RECALL_MELPA_PIN, source_file)
        .expect("prepare pinned agent-recall source below ./tmp")
        .with_prelude(prelude)
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

pub(crate) fn assert_agent_recall_consult_parity(elisp_form: &str, expected: Expect) {
    assert_agent_recall_source_parity("agent-recall-consult.el", elisp_form, expected);
}
