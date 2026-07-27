use std::time::Duration;

use crate::{APDL_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod font_lock;
mod indentation;
mod mode;
mod navigation;
mod registry;
mod syntax;
mod variables;
mod workflows;

const APDL_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(240);

fn apdl_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APDL_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned apdl-mode source below ./tmp")
        .with_prelude(
            r##"(progn
                   (require 'cl-lib)
                   ;; Mode activation normally discovers a local Ansys
                   ;; installation. Parity cases exercise the editor and
                   ;; language behavior without relying on either host.
                   (setq apdl-initialised-flag t))"##,
        )
        .with_timeout(APDL_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apdl-mode parity test")
        .into()
}

fn assert_apdl_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apdl_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apdl-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apdl_mode_parity(elisp_form: &str, expected: Expect) {
    assert_apdl_mode_source_parity("apdl-mode.el", elisp_form, expected);
}

pub(crate) fn assert_apdl_mode_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apdl_mode_oracle("apdl-mode.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apdl-mode signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apdl_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_apdl_mode_source_parity("apdl-mode-autoloads.el", elisp_form, expected);
}
