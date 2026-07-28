use std::time::Duration;

use crate::{APIWRAP_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod practical;

const APIWRAP_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn apiwrap_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(APIWRAP_MELPA_PIN, "apiwrap.el")
        .expect("prepare pinned apiwrap source below ./tmp")
        .with_timeout(APIWRAP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apiwrap parity test")
        .into()
}

pub(crate) fn assert_apiwrap_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apiwrap_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apiwrap parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
