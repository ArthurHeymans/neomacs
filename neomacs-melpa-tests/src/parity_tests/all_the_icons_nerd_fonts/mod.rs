use std::time::Duration;

use crate::{ALL_THE_ICONS_NERD_FONTS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod families;
mod overrides;
mod prefer;
mod registry;

const ALL_THE_ICONS_NERD_FONTS_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn all_the_icons_nerd_fonts_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALL_THE_ICONS_NERD_FONTS_MELPA_PIN, source_file)
        .expect("prepare pinned all-the-icons-nerd-fonts source below ./tmp")
        .with_timeout(ALL_THE_ICONS_NERD_FONTS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed all-the-icons-nerd-fonts parity test")
        .into()
}

fn assert_all_the_icons_nerd_fonts_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = all_the_icons_nerd_fonts_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("all-the-icons-nerd-fonts parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_all_the_icons_nerd_fonts_parity(elisp_form: &str, expected: Expect) {
    assert_all_the_icons_nerd_fonts_source_parity(
        "all-the-icons-nerd-fonts.el",
        elisp_form,
        expected,
    );
}

pub(crate) fn assert_all_the_icons_nerd_fonts_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_all_the_icons_nerd_fonts_source_parity(
        "all-the-icons-nerd-fonts-autoloads.el",
        elisp_form,
        expected,
    );
}





/// Multi-probe batch for `assert_all_the_icons_nerd_fonts_autoload_parity` cases (2a).
pub(crate) fn assert_all_the_icons_nerd_fonts_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        all_the_icons_nerd_fonts_oracle("all-the-icons-nerd-fonts-autoloads.el"),
        &name,
        "all_the_icons_nerd_fonts_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_all_the_icons_nerd_fonts_parity` cases (2a).
pub(crate) fn assert_all_the_icons_nerd_fonts_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        all_the_icons_nerd_fonts_oracle("all-the-icons-nerd-fonts.el"),
        &name,
        "all_the_icons_nerd_fonts_parity",
        cases,
    );
}
