use std::time::Duration;

use crate::{AAS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod expansion;
mod formatting;
mod keymaps;
mod modes;
mod surface;

const AAS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aas_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AAS_MELPA_PIN, "aas.el")
        .expect("prepare pinned aas source below ./tmp")
        .with_timeout(AAS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed aas parity test").into()
}

pub(crate) fn assert_aas_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aas_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("aas parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aas_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aas_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("aas signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
