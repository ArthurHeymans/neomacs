use std::time::Duration;

use crate::{ANKI_CONNECT_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod autoloads;
mod decks;
mod notes;
mod surface;
mod transport;

const ANKI_CONNECT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn anki_connect_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANKI_CONNECT_MELPA_PIN, source_file)
        .expect("prepare pinned anki-connect source below ./tmp")
        .with_timeout(ANKI_CONNECT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anki-connect parity test")
        .into()
}

fn assert_anki_connect_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anki_connect_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anki-connect parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anki_connect_parity(elisp_form: &str, expected: Expect) {
    assert_anki_connect_source_parity("anki-connect.el", elisp_form, expected);
}

pub(crate) fn assert_anki_connect_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anki_connect_source_parity("anki-connect-autoloads.el", elisp_form, expected);
}
