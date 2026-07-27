use std::time::Duration;

use crate::{ANTI_ZENBURN_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod faces;
mod lifecycle;
mod registry;
mod rendering;
mod variables;

const ANTI_ZENBURN_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ANTI_ZENBURN_THEME_COLOR_PRELUDE: &str = r##"
(fset 'display-color-cells
      (lambda (&optional _display)
        16777216))
(defvar neomacs-melpa-tests--original-face-spec-set-match-display
  (symbol-function 'face-spec-set-match-display))
(fset 'face-spec-set-match-display
      (lambda (display frame)
        (if (equal display
                   '((class color)
                     (min-colors 89)))
            t
          (funcall
           neomacs-melpa-tests--original-face-spec-set-match-display
           display
           frame))))
"##;

fn anti_zenburn_theme_oracle(source_file: &str, prelude: &str) -> CachedMelpaOracle {
    let combined_prelude = format!("{ANTI_ZENBURN_THEME_COLOR_PRELUDE}\n{prelude}");
    CachedMelpaOracle::new(ANTI_ZENBURN_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned anti-zenburn-theme source below ./tmp")
        .with_prelude(combined_prelude)
        .with_timeout(ANTI_ZENBURN_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anti-zenburn-theme parity test")
        .into()
}

fn assert_anti_zenburn_theme_source_parity(
    source_file: &str,
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = anti_zenburn_theme_oracle(source_file, prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anti-zenburn-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anti_zenburn_theme_parity(elisp_form: &str, expected: Expect) {
    assert_anti_zenburn_theme_source_parity("anti-zenburn-theme.el", "", elisp_form, expected);
}

pub(crate) fn assert_anti_zenburn_theme_with_prelude_parity(
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    assert_anti_zenburn_theme_source_parity("anti-zenburn-theme.el", prelude, elisp_form, expected);
}

pub(crate) fn assert_anti_zenburn_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anti_zenburn_theme_source_parity(
        "anti-zenburn-theme-autoloads.el",
        "",
        elisp_form,
        expected,
    );
}
