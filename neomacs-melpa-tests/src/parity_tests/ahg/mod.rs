use std::time::Duration;

use crate::{AHG_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands_grep;
mod diff_annotate;
mod helpers;
mod logs;
mod mq;
mod record_histedit;
mod repository;
mod surface;

const AHG_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ahg_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AHG_MELPA_PIN, "ahg.el")
        .expect("prepare pinned ahg source below ./tmp")
        .with_timeout(AHG_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ahg parity test").into()
}

pub(crate) fn assert_ahg_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ahg_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ahg parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
