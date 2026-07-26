use std::time::Duration;

use crate::{AC_OCTAVE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod completion;
mod lifecycle;
mod surface;

const AC_OCTAVE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_octave_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_OCTAVE_MELPA_PIN, "ac-octave.el")
        .expect("prepare pinned ac-octave source below ./tmp")
        .with_timeout(AC_OCTAVE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-octave parity test")
        .into()
}

pub(crate) fn assert_ac_octave_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_octave_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-octave parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
