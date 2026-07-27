use std::time::Duration;

use crate::{ALECT_THEMES_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod faces;
mod lifecycle;
mod palettes;
mod registry;
mod rendering;

const ALECT_THEMES_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ALECT_THEMES_PRELUDE: &str = r##"
(require 'cl-lib)
(fset 'display-color-cells
      (lambda (&optional _display) 16777216))
"##;

fn alect_themes_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALECT_THEMES_MELPA_PIN, source_file)
        .expect("prepare pinned alect-themes source below ./tmp")
        .with_prelude(ALECT_THEMES_PRELUDE)
        .with_timeout(ALECT_THEMES_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed alect-themes parity test")
        .into()
}

fn assert_alect_themes_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = alect_themes_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("alect-themes parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_alect_themes_parity(elisp_form: &str, expected: Expect) {
    assert_alect_themes_source_parity("alect-themes.el", elisp_form, expected);
}

pub(crate) fn assert_alect_themes_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_alect_themes_source_parity("alect-themes-autoloads.el", elisp_form, expected);
}
