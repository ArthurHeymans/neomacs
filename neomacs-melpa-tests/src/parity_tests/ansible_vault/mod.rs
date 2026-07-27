use std::time::Duration;

use crate::{ANSIBLE_VAULT_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod execute;
mod headers;
mod mode;
mod passwords;
mod surface;

const ANSIBLE_VAULT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ansible_vault_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANSIBLE_VAULT_MELPA_PIN, source_file)
        .expect("prepare pinned ansible-vault source below ./tmp")
        .with_prelude(
            r##"(progn
                   (setq exec-path nil)
                   (setenv "PATH" ""))"##,
        )
        .with_timeout(ANSIBLE_VAULT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ansible-vault parity test")
        .into()
}

fn assert_ansible_vault_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ansible_vault_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ansible-vault parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ansible_vault_parity(elisp_form: &str, expected: Expect) {
    assert_ansible_vault_source_parity("ansible-vault.el", elisp_form, expected);
}

pub(crate) fn assert_ansible_vault_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ansible_vault_source_parity("ansible-vault-autoloads.el", elisp_form, expected);
}
