use std::time::Duration;

use crate::{ALABASTER_THEMES_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod faces;
mod lifecycle;
mod palettes;
mod registry;
mod rendering;
mod workflows;

const ALABASTER_THEMES_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const TRUE_COLOR_PRELUDE: &str = r##"
(require 'cl-lib)
(fset 'display-color-cells
      (lambda (&optional _display) 16777216))
"##;

fn alabaster_themes_oracle(source_file: &str, prelude: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALABASTER_THEMES_MELPA_PIN, source_file)
        .expect("prepare pinned alabaster-themes source below ./tmp")
        .with_prelude(format!("{TRUE_COLOR_PRELUDE}\n{prelude}"))
        .with_timeout(ALABASTER_THEMES_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed alabaster-themes parity test")
        .into()
}

fn assert_alabaster_themes_source_parity(
    source_file: &str,
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = alabaster_themes_oracle(source_file, prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("alabaster-themes parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_alabaster_themes_parity(elisp_form: &str, expected: Expect) {
    assert_alabaster_themes_source_parity("alabaster-themes.el", "", elisp_form, expected);
}

pub(crate) fn assert_alabaster_themes_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_alabaster_themes_source_parity(
        "alabaster-themes-autoloads.el",
        "",
        elisp_form,
        expected,
    );
}





/// Multi-probe batch for `assert_alabaster_themes_autoload_parity` cases (2a).
pub(crate) fn assert_alabaster_themes_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        alabaster_themes_oracle("alabaster-themes-autoloads.el", ""),
        &name,
        "alabaster_themes_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_alabaster_themes_parity` cases (2a).
pub(crate) fn assert_alabaster_themes_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        alabaster_themes_oracle("alabaster-themes.el", ""),
        &name,
        "alabaster_themes_parity",
        cases,
    );
}
