use std::time::Duration;

use crate::{AC_HTML_BOOTSTRAP_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod activation;
mod data;
mod surface;

const AC_HTML_BOOTSTRAP_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_html_bootstrap_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_HTML_BOOTSTRAP_MELPA_PIN, "ac-html-bootstrap.el")
        .expect("prepare pinned ac-html-bootstrap source below ./tmp")
        .with_timeout(AC_HTML_BOOTSTRAP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-html-bootstrap parity test")
        .into()
}

pub(crate) fn assert_ac_html_bootstrap_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_html_bootstrap_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-html-bootstrap parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
