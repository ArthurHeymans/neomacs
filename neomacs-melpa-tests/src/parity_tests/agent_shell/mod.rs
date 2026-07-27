use std::time::Duration;

use crate::{AGENT_SHELL_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod completion;
mod config_usage;
mod content;
mod core;
mod diff;
mod events;
mod list_edit;
mod markdown;
mod misc;
mod providers;
mod ui;

const AGENT_SHELL_TEST_TIMEOUT: Duration = Duration::from_secs(15);
const AGENT_SHELL_PRELUDE: &str = r##"
(dolist (entry (directory-files package-user-dir t "\\`[^.]"))
  (when (file-directory-p entry)
    (add-to-list 'load-path entry)))
"##;

fn agent_shell_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGENT_SHELL_MELPA_PIN, source_file)
        .expect("prepare pinned agent-shell source below ./tmp")
        .with_prelude(AGENT_SHELL_PRELUDE)
        .with_timeout(AGENT_SHELL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed agent-shell parity test")
        .into()
}

fn assert_agent_shell_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = agent_shell_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("agent-shell parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_agent_shell_parity(elisp_form: &str, expected: Expect) {
    assert_agent_shell_source_parity("agent-shell.el", elisp_form, expected);
}
