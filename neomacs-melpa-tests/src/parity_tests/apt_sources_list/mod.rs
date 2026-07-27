use std::time::Duration;

use crate::{APT_SOURCES_LIST_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod editing;
mod mode;
mod navigation;
mod parsing;
mod registry;
mod upstream_workflows;

const APT_SOURCES_LIST_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn apt_sources_list_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APT_SOURCES_LIST_MELPA_PIN, source_file)
        .expect("prepare pinned apt-sources-list source below ./tmp")
        .with_timeout(APT_SOURCES_LIST_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apt-sources-list parity test")
        .into()
}

fn assert_apt_sources_list_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apt_sources_list_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apt-sources-list parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apt_sources_list_parity(elisp_form: &str, expected: Expect) {
    assert_apt_sources_list_source_parity("apt-sources-list.el", elisp_form, expected);
}

pub(crate) fn assert_apt_sources_list_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apt_sources_list_oracle("apt-sources-list.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("apt-sources-list signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apt_sources_list_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_apt_sources_list_source_parity("apt-sources-list-autoloads.el", elisp_form, expected);
}
