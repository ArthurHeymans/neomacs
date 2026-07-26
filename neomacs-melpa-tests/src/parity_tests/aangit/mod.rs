use std::time::Duration;

use crate::{AANGIT_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod layouts;
mod readers;

const AANGIT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aangit_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AANGIT_MELPA_PIN, "aangit.el")
        .expect("prepare pinned aangit source below ./tmp")
        .with_timeout(AANGIT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed aangit parity test").into()
}

pub(crate) fn assert_aangit_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aangit_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("aangit parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
