use std::time::Duration;

use crate::{AC_GEISER_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod candidates;
mod documentation;
mod setup;
mod surface;

const AC_GEISER_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_geiser_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_GEISER_MELPA_PIN, "ac-geiser.el")
        .expect("prepare pinned ac-geiser source below ./tmp")
        .with_timeout(AC_GEISER_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-geiser parity test")
        .into()
}

pub(crate) fn assert_ac_geiser_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_geiser_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-geiser parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_geiser_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_geiser_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-geiser signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
