use std::time::Duration;

use crate::{ALL_THE_ICONS_IVY_RICH_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod annotations;
mod files;
mod icons;
mod processes;
mod surface;

const ALL_THE_ICONS_IVY_RICH_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn all_the_icons_ivy_rich_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(
        ALL_THE_ICONS_IVY_RICH_MELPA_PIN,
        "all-the-icons-ivy-rich.el",
    )
    .expect("prepare pinned all-the-icons-ivy-rich and its dependencies below ./tmp")
    .with_timeout(ALL_THE_ICONS_IVY_RICH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed all-the-icons-ivy-rich parity test")
        .into()
}

pub(crate) fn assert_all_the_icons_ivy_rich_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = all_the_icons_ivy_rich_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("all-the-icons-ivy-rich parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_all_the_icons_ivy_rich_parity` cases (2a).
pub(crate) fn assert_all_the_icons_ivy_rich_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        all_the_icons_ivy_rich_oracle(),
        &name,
        "all_the_icons_ivy_rich_parity",
        cases,
    );
}
