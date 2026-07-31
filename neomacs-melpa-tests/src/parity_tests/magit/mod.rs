use std::time::Duration;

use crate::{CachedMelpaOracle, MAGIT_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod blame;
mod clone;
mod formatting;
mod git;
mod prompts;
mod status;

const MAGIT_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn magit_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(MAGIT_MELPA_PIN, "magit.el")
        .expect("prepare pinned Magit source and dependencies below ./tmp")
        .with_prelude(
            r##"(setq magit-git-global-arguments
                       (append
                        '("-c" "init.defaultBranch=master"
                          "-c" "user.name=A U Thor"
                          "-c" "user.email=a.u.thor@example.com")
                        (and (boundp 'magit-git-global-arguments)
                             magit-git-global-arguments)))"##,
        )
        .with_timeout(MAGIT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed Magit parity test").into()
}

pub(crate) fn assert_magit_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = magit_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("Magit parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_magit_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = magit_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("Magit signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_magit_parity` cases (2a).
pub(crate) fn assert_magit_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        magit_oracle(),
        &name,
        "magit_parity",
        cases,
    );
}
