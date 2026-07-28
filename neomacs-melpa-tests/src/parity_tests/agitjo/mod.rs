use std::time::Duration;

use crate::{AGITJO_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod composition;
mod publish;
mod smoke;

const AGITJO_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn agitjo_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGITJO_MELPA_PIN, "agitjo.el")
        .expect("prepare pinned agitjo source below ./tmp")
        .with_timeout(AGITJO_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed agitjo parity test").into()
}

pub(crate) fn assert_agitjo_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = agitjo_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("agitjo parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
