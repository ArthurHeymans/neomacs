use std::time::Duration;

use crate::{AC_C_HEADERS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod files;
mod integration;
mod symbols;

const AC_C_HEADERS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_c_headers_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_C_HEADERS_MELPA_PIN, "ac-c-headers.el")
        .expect("prepare pinned ac-c-headers source below ./tmp")
        .with_timeout(AC_C_HEADERS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-c-headers parity test")
        .into()
}

pub(crate) fn assert_ac_c_headers_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_c_headers_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-c-headers parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
