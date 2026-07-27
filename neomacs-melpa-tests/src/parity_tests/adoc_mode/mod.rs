use std::time::Duration;

use crate::{ADOC_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod antora;
mod asciidoctor;
mod completion;
mod editing;
mod fill_imenu;
mod font_lock;
mod images;
mod navigation;
mod regex;
mod registry;
mod tempo;

const ADOC_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn adoc_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ADOC_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned adoc-mode source below ./tmp")
        .with_timeout(ADOC_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed adoc-mode parity test")
        .into()
}

fn assert_adoc_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = adoc_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("adoc-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_adoc_mode_parity(elisp_form: &str, expected: Expect) {
    assert_adoc_mode_source_parity("adoc-mode.el", elisp_form, expected);
}

pub(crate) fn assert_adoc_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_adoc_mode_source_parity("adoc-mode-autoloads.el", elisp_form, expected);
}
