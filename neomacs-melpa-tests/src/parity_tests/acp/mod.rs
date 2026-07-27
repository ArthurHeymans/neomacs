use std::time::Duration;

use crate::{ACP_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod client;
mod constructors;
mod fakes;
mod logging;
mod registry;
mod routing;
mod traffic;
mod transport;

const ACP_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn acp_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACP_MELPA_PIN, source_file)
        .expect("prepare pinned acp source below ./tmp")
        .with_timeout(ACP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed acp parity test").into()
}

fn assert_acp_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = acp_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("acp parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_acp_parity(elisp_form: &str, expected: Expect) {
    assert_acp_source_parity("acp.el", elisp_form, expected);
}

pub(crate) fn assert_acp_traffic_parity(elisp_form: &str, expected: Expect) {
    assert_acp_source_parity("acp-traffic.el", elisp_form, expected);
}

pub(crate) fn assert_acp_fakes_parity(elisp_form: &str, expected: Expect) {
    assert_acp_source_parity("acp-fakes.el", elisp_form, expected);
}

pub(crate) fn assert_acp_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_acp_source_parity("acp-autoloads.el", elisp_form, expected);
}
