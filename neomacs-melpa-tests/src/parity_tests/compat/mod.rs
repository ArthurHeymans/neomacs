use std::time::Duration;

use crate::{COMPAT_GNU_ELPA_PIN, CachedPackageOracle};
use expect_test::Expect;

mod buffers;
mod collections;
mod core;

const COMPAT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn compat_oracle() -> CachedPackageOracle {
    CachedPackageOracle::new_from_gnu_elpa(COMPAT_GNU_ELPA_PIN, "compat.el")
        .expect("prepare pinned Compat source and dependencies below ./tmp")
        .with_timeout(COMPAT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed Compat parity test").into()
}

pub(crate) fn assert_compat_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = compat_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("Compat parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_compat_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = compat_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("Compat signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
