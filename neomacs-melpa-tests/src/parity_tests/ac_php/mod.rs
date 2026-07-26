use std::time::Duration;

use crate::{AC_PHP_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod actions;
mod prefix;
mod presentation;
mod surface;

const AC_PHP_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_php_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_PHP_MELPA_PIN, "ac-php.el")
        .expect("prepare pinned ac-php source below ./tmp")
        .with_timeout(AC_PHP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ac-php parity test").into()
}

pub(crate) fn assert_ac_php_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_php_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-php parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
