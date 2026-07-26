use std::time::Duration;

use crate::{AC_PHP_CORE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod context;
mod filesystem;
mod lifecycle;
mod navigation;
mod parser;
mod search;
mod surface;
mod tags;
mod utilities;

const AC_PHP_CORE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_php_core_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_PHP_CORE_MELPA_PIN, "ac-php-core.el")
        .expect("prepare pinned ac-php-core source below ./tmp")
        .with_timeout(AC_PHP_CORE_TEST_TIMEOUT)
}

fn ac_php_core_autoload_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_PHP_CORE_MELPA_PIN, "ac-php-core-autoloads.el")
        .expect("prepare pinned ac-php-core autoload source below ./tmp")
        .with_timeout(AC_PHP_CORE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-php-core parity test")
        .into()
}

pub(crate) fn assert_ac_php_core_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_php_core_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-php-core parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_php_core_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_php_core_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-php-core signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_php_core_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_php_core_autoload_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ac-php-core autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
