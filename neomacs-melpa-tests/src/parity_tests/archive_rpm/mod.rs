use std::time::Duration;

use crate::{ARCHIVE_RPM_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod compression;
mod cpio_modes;
mod cpio_workflows;
mod detection;
mod registry;
mod rpm_headers;
mod rpm_workflows;

const ARCHIVE_RPM_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn archive_rpm_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARCHIVE_RPM_MELPA_PIN, source_file)
        .expect("prepare pinned archive-rpm source below ./tmp")
        .with_timeout(ARCHIVE_RPM_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed archive-rpm parity test")
        .into()
}

fn assert_archive_rpm_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = archive_rpm_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("archive-rpm parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_archive_rpm_parity(elisp_form: &str, expected: Expect) {
    assert_archive_rpm_source_parity("archive-rpm.el", elisp_form, expected);
}

pub(crate) fn assert_archive_rpm_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = archive_rpm_oracle("archive-rpm.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("archive-rpm signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_archive_cpio_parity(elisp_form: &str, expected: Expect) {
    assert_archive_rpm_source_parity("archive-cpio.el", elisp_form, expected);
}

pub(crate) fn assert_archive_cpio_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = archive_rpm_oracle("archive-cpio.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("archive-cpio signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_archive_rpm_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_archive_rpm_source_parity("archive-rpm-autoloads.el", elisp_form, expected);
}
