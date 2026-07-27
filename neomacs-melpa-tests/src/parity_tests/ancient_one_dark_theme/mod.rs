use std::time::Duration;

use crate::{ANCIENT_ONE_DARK_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod faces;
mod lifecycle;
mod registry;
mod rendering;

const ANCIENT_ONE_DARK_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ANCIENT_ONE_DARK_THEME_COLOR_PRELUDE: &str = r##"
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

fn ancient_one_dark_theme_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANCIENT_ONE_DARK_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned ancient-one-dark-theme source below ./tmp")
        .with_timeout(ANCIENT_ONE_DARK_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ancient-one-dark-theme parity test")
        .into()
}

pub(crate) fn assert_ancient_one_dark_theme_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ancient_one_dark_theme_oracle("ancient-one-dark-theme.el")
        .with_prelude(ANCIENT_ONE_DARK_THEME_COLOR_PRELUDE)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("ancient-one-dark-theme parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ancient_one_dark_theme_with_prelude_parity(
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let combined_prelude = format!("{ANCIENT_ONE_DARK_THEME_COLOR_PRELUDE}\n{prelude}");
    let report = ancient_one_dark_theme_oracle("ancient-one-dark-theme.el")
        .with_prelude(combined_prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("ancient-one-dark-theme pre-load parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ancient_one_dark_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ancient_one_dark_theme_oracle("ancient-one-dark-theme-autoloads.el")
        .with_prelude(ANCIENT_ONE_DARK_THEME_COLOR_PRELUDE)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("ancient-one-dark-theme autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
