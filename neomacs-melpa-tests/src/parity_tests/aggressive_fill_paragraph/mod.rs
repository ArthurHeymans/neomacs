use std::time::Duration;

use crate::{AGGRESSIVE_FILL_PARAGRAPH_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod editing;
mod lifecycle;
mod smoke;
mod suppression;

const AGGRESSIVE_FILL_PARAGRAPH_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aggressive_fill_paragraph_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(
        AGGRESSIVE_FILL_PARAGRAPH_MELPA_PIN,
        "aggressive-fill-paragraph.el",
    )
    .expect("prepare pinned aggressive-fill-paragraph source below ./tmp")
    .with_timeout(AGGRESSIVE_FILL_PARAGRAPH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aggressive-fill-paragraph parity test")
        .into()
}

pub(crate) fn assert_aggressive_fill_paragraph_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aggressive_fill_paragraph_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("aggressive-fill-paragraph parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
