use std::time::Duration;

use crate::{CachedMelpaOracle, GAME_2048_MELPA_PIN};
use expect_test::Expect;

mod lifecycle;
mod moves;
mod state;

const GAME_2048_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn game_2048_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(GAME_2048_MELPA_PIN, "2048-game.el")
        .expect("prepare pinned 2048-game source below ./tmp")
        .with_timeout(GAME_2048_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed 2048-game parity test")
        .into()
}

pub(crate) fn assert_game_2048_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = game_2048_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("2048-game parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
