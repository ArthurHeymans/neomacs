use std::time::Duration;

use crate::{AC_ISPELL_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod cache;
mod candidates;
mod protocol;
mod setup;
mod surface;

const AC_ISPELL_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_ispell_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_ISPELL_MELPA_PIN, "ac-ispell.el")
        .expect("prepare pinned ac-ispell source below ./tmp")
        .with_timeout(AC_ISPELL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-ispell parity test")
        .into()
}

pub(crate) fn assert_ac_ispell_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_ispell_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-ispell parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
