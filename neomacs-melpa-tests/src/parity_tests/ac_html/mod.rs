use std::time::Duration;

use crate::{AC_HTML_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod candidates;
mod default_data;
mod documentation;
mod html;
mod providers;
mod slim;
mod surface;
mod templates;

const AC_HTML_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_html_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_HTML_MELPA_PIN, "ac-html.el")
        .expect("prepare pinned ac-html source below ./tmp")
        .with_timeout(AC_HTML_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-html parity test")
        .into()
}

pub(crate) fn assert_ac_html_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_html_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-html parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_html_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_html_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-html signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
