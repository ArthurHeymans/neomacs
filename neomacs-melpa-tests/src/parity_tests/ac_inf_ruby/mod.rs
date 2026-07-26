use std::time::Duration;

use crate::{AC_INF_RUBY_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod activation;
mod completion;
mod surface;

const AC_INF_RUBY_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_inf_ruby_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_INF_RUBY_MELPA_PIN, "ac-inf-ruby.el")
        .expect("prepare pinned ac-inf-ruby source below ./tmp")
        .with_timeout(AC_INF_RUBY_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-inf-ruby parity test")
        .into()
}

pub(crate) fn assert_ac_inf_ruby_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_inf_ruby_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-inf-ruby parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_inf_ruby_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_inf_ruby_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-inf-ruby signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
