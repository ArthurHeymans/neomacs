use std::time::Duration;

use crate::{ANX_API_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod conversions;
mod http;
mod registry;
mod workflows;

const ANX_API_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anx_api_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANX_API_MELPA_PIN, source_file)
        .expect("prepare pinned anx-api source below ./tmp")
        .with_timeout(ANX_API_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anx-api parity test")
        .into()
}

fn assert_anx_api_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anx_api_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anx-api parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anx_api_parity(elisp_form: &str, expected: Expect) {
    assert_anx_api_source_parity("anx-api.el", elisp_form, expected);
}

pub(crate) fn assert_anx_api_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anx_api_source_parity("anx-api-autoloads.el", elisp_form, expected);
}
