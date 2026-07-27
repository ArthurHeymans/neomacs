use std::time::Duration;

use crate::{ALL_THE_ICONS_GNUS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod article;
mod formats;
mod registry;
mod setup;

const ALL_THE_ICONS_GNUS_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const DETERMINISTIC_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'dash)
(defun all-the-icons-gnus-test-properties-at (position)
  (let* ((composition
          (get-text-property position 'composition))
         (component (and composition (car composition)))
         (glyph (and component (cdr component))))
    (list
     :face
     (copy-tree (get-text-property position 'face))
     :composition
     (and
      glyph
      (list
       (car component)
       (substring-no-properties glyph)
       (copy-tree (get-text-property 0 'face glyph))
       (copy-tree (get-text-property 0 'display glyph)))))))
"##;

fn all_the_icons_gnus_oracle(source_file: &str, prelude: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALL_THE_ICONS_GNUS_MELPA_PIN, source_file)
        .expect("prepare pinned all-the-icons-gnus source below ./tmp")
        .with_prelude(format!("{DETERMINISTIC_PRELUDE}\n{prelude}"))
        .with_timeout(ALL_THE_ICONS_GNUS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed all-the-icons-gnus parity test")
        .into()
}

fn assert_all_the_icons_gnus_source_parity(
    source_file: &str,
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = all_the_icons_gnus_oracle(source_file, prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("all-the-icons-gnus parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_all_the_icons_gnus_parity(elisp_form: &str, expected: Expect) {
    assert_all_the_icons_gnus_source_parity("all-the-icons-gnus.el", "", elisp_form, expected);
}

pub(crate) fn assert_all_the_icons_gnus_with_prelude_parity(
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    assert_all_the_icons_gnus_source_parity("all-the-icons-gnus.el", prelude, elisp_form, expected);
}

pub(crate) fn assert_all_the_icons_gnus_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_all_the_icons_gnus_source_parity(
        "all-the-icons-gnus-autoloads.el",
        "",
        elisp_form,
        expected,
    );
}
