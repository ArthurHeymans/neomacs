use std::time::Duration;

use crate::{AA_EDIT_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod core;
mod mode;

const AA_EDIT_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aa_edit_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AA_EDIT_MODE_MELPA_PIN, "aa-edit-mode.el")
        .expect("prepare pinned aa-edit-mode source below ./tmp")
        .with_timeout(AA_EDIT_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aa-edit-mode parity test")
        .into()
}

pub(crate) fn assert_aa_edit_mode_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aa_edit_mode_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("aa-edit-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
