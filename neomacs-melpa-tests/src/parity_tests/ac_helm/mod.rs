use std::time::Duration;

use crate::{AC_HELM_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod action;
mod candidates;
mod command;
mod initialization;
mod surface;

const AC_HELM_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_helm_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_HELM_MELPA_PIN, "ac-helm.el")
        .expect("prepare pinned ac-helm source below ./tmp")
        .with_timeout(AC_HELM_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-helm parity test")
        .into()
}

pub(crate) fn assert_ac_helm_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_helm_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-helm parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_helm_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_helm_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-helm signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
