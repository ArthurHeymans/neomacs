use std::time::Duration;

use crate::{ARCH_PACKER_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod entries;
mod interactions;
mod parsing;
mod shell;
mod surface;

const ARCH_PACKER_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn arch_packer_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARCH_PACKER_MELPA_PIN, source_file)
        .expect("prepare pinned arch-packer source below ./tmp")
        .with_timeout(ARCH_PACKER_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed arch-packer parity test")
        .into()
}

fn assert_arch_packer_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arch_packer_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arch-packer parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arch_packer_parity(elisp_form: &str, expected: Expect) {
    assert_arch_packer_source_parity("arch-packer.el", elisp_form, expected);
}

pub(crate) fn assert_arch_packer_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_arch_packer_source_parity("arch-packer-autoloads.el", elisp_form, expected);
}

pub(crate) fn assert_arch_packer_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arch_packer_oracle("arch-packer.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arch-packer signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
