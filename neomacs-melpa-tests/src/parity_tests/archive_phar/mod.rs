use std::time::Duration;

use crate::{ARCHIVE_PHAR_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ARCHIVE_PHAR_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn archive_phar_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARCHIVE_PHAR_MELPA_PIN, "archive-phar.el")
        .expect("prepare pinned archive-phar source below ./tmp")
        .with_timeout(ARCHIVE_PHAR_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed archive-phar parity test")
        .into()
}

pub(crate) fn assert_archive_phar_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = archive_phar_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("archive-phar parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_archive_phar_parity` cases (2a).
pub(crate) fn assert_archive_phar_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        archive_phar_oracle(),
        &name,
        "archive_phar_parity",
        cases,
    );
}
