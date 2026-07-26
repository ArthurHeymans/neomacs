use std::time::Duration;

use crate::{CachedMelpaOracle, DASH_MELPA_PIN};
use expect_test::Expect;

mod control_flow;
mod reductions;
mod sequences;
mod sets_and_trees;
mod traversal;

const DASH_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn dash_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(DASH_MELPA_PIN, "dash.el")
        .expect("prepare pinned Dash source below ./tmp")
        .with_prelude(r##"(require 'cl-lib)"##)
        .with_timeout(DASH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed Dash parity test").into()
}

pub(crate) fn assert_dash_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = dash_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("Dash parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_dash_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = dash_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("Dash signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
