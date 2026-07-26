use std::time::Duration;

use crate::{AC_DCD_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod completion;
mod imports;
mod lifecycle;
mod parsing;
mod surface;

const AC_DCD_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_dcd_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_DCD_MELPA_PIN, "ac-dcd.el")
        .expect("prepare pinned ac-dcd source below ./tmp")
        .with_timeout(AC_DCD_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ac-dcd parity test").into()
}

pub(crate) fn assert_ac_dcd_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_dcd_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-dcd parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_dcd_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_dcd_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-dcd signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
