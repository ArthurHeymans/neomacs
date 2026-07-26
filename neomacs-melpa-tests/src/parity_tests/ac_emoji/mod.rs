use std::time::Duration;

use crate::{AC_EMOJI_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod data;
mod setup;
mod surface;

const AC_EMOJI_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_emoji_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_EMOJI_MELPA_PIN, "ac-emoji.el")
        .expect("prepare pinned ac-emoji source below ./tmp")
        .with_timeout(AC_EMOJI_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-emoji parity test")
        .into()
}

pub(crate) fn assert_ac_emoji_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_emoji_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-emoji parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
