use std::time::Duration;

use crate::{ALL_THE_ICONS_COMPLETION_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod icons;
mod metadata;
mod mode;
mod registry;

const ALL_THE_ICONS_COMPLETION_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const DETERMINISTIC_PRELUDE: &str = r##"
(require 'cl-lib)
"##;

fn all_the_icons_completion_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALL_THE_ICONS_COMPLETION_MELPA_PIN, source_file)
        .expect("prepare pinned all-the-icons-completion source below ./tmp")
        .with_prelude(DETERMINISTIC_PRELUDE)
        .with_timeout(ALL_THE_ICONS_COMPLETION_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed all-the-icons-completion parity test")
        .into()
}

fn assert_all_the_icons_completion_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = all_the_icons_completion_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("all-the-icons-completion parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_all_the_icons_completion_parity(elisp_form: &str, expected: Expect) {
    assert_all_the_icons_completion_source_parity(
        "all-the-icons-completion.el",
        elisp_form,
        expected,
    );
}

pub(crate) fn assert_all_the_icons_completion_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_all_the_icons_completion_source_parity(
        "all-the-icons-completion-autoloads.el",
        elisp_form,
        expected,
    );
}
