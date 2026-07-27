use std::time::Duration;

use crate::{AOZORA_VIEW_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod cache;
mod layout;
mod navigation;
mod render;
mod surface;
mod workflow;

const AOZORA_VIEW_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn aozora_view_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AOZORA_VIEW_MELPA_PIN, source_file)
        .expect("prepare pinned aozora-view source below ./tmp")
        .with_prelude(
            r##"(progn
                   (defvar byte-compile-current-file nil))"##,
        )
        .with_timeout(AOZORA_VIEW_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aozora-view parity test")
        .into()
}

fn assert_aozora_view_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aozora_view_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aozora-view parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aozora_view_parity(elisp_form: &str, expected: Expect) {
    assert_aozora_view_source_parity("aozora-view.el", elisp_form, expected);
}

pub(crate) fn assert_aozora_view_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aozora_view_oracle("aozora-view.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aozora-view signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aozora_view_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_aozora_view_source_parity("aozora-view-autoloads.el", elisp_form, expected);
}
