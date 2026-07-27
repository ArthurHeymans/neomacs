use std::time::Duration;

use crate::{AES_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod buffers;
mod ciphers;
mod passwords;
mod primitives;
mod surface;

const AES_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aes_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AES_MELPA_PIN, "aes.el")
        .expect("prepare pinned aes source below ./tmp")
        .with_timeout(AES_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed aes parity test").into()
}

pub(crate) fn assert_aes_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aes_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("aes parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aes_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aes_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("aes signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
