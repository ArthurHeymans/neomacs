use std::time::Duration;

use crate::{AIKEN_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod editing;
mod font_lock;
mod mode;
mod workflows;

const AIKEN_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aiken_mode_oracle() -> CachedMelpaOracle {
    // MELPA built this release from aiken-lang/aiken-mode revision
    // 1af54e4df02eb52cf62034acbe1c6dd54776d843. The package was later removed
    // from the live catalog, but its digest-pinned payload remains available.
    CachedMelpaOracle::new_from_frozen_melpa_archive(
        AIKEN_MODE_MELPA_PIN,
        "aiken-mode.el",
        "https://melpa.org/packages/aiken-mode-20230920.1210.tar",
        "9ba361ec1a4acf2d5c2083c5fe748da3a4015f3219a9a69b10b4babd118a301f",
    )
    .expect("prepare pinned aiken-mode source below ./tmp")
    .with_prelude(
        r##"
(require 'cl-lib)
(require 'compile)
(require 'project)
(require 'thingatpt)
"##,
    )
    .with_timeout(AIKEN_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aiken-mode parity test")
        .into()
}

pub(crate) fn assert_aiken_mode_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aiken_mode_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aiken-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
