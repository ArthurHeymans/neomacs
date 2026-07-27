use std::time::Duration;

use crate::{ALL_THE_ICONS_IVY_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod buffers;
mod files;
mod registry;
mod setup;

const ALL_THE_ICONS_IVY_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn all_the_icons_ivy_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALL_THE_ICONS_IVY_MELPA_PIN, source_file)
        .expect("prepare pinned all-the-icons-ivy source below ./tmp")
        .with_timeout(ALL_THE_ICONS_IVY_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed all-the-icons-ivy parity test")
        .into()
}

fn assert_all_the_icons_ivy_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = all_the_icons_ivy_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("all-the-icons-ivy parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_all_the_icons_ivy_parity(elisp_form: &str, expected: Expect) {
    assert_all_the_icons_ivy_source_parity("all-the-icons-ivy.el", elisp_form, expected);
}

pub(crate) fn assert_all_the_icons_ivy_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_all_the_icons_ivy_source_parity("all-the-icons-ivy-autoloads.el", elisp_form, expected);
}
