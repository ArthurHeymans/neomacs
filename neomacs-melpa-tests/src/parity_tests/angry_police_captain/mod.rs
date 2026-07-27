use std::time::Duration;

use crate::{ANGRY_POLICE_CAPTAIN_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod callback;
mod errors;
mod registry;

const ANGRY_POLICE_CAPTAIN_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn angry_police_captain_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANGRY_POLICE_CAPTAIN_MELPA_PIN, source_file)
        .expect("prepare pinned angry-police-captain source below ./tmp")
        .with_timeout(ANGRY_POLICE_CAPTAIN_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed angry-police-captain parity test")
        .into()
}

fn assert_angry_police_captain_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = angry_police_captain_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("angry-police-captain parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_angry_police_captain_parity(elisp_form: &str, expected: Expect) {
    assert_angry_police_captain_source_parity("angry-police-captain.el", elisp_form, expected);
}

pub(crate) fn assert_angry_police_captain_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_angry_police_captain_source_parity(
        "angry-police-captain-autoloads.el",
        elisp_form,
        expected,
    );
}
