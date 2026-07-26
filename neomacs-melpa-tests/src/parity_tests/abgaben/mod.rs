use std::time::Duration;

use crate::{ABGABEN_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod annotations;
mod archives;
mod org_workflow;
mod surface;

const ABGABEN_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn abgaben_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ABGABEN_MELPA_PIN, "abgaben.el")
        .expect("prepare pinned abgaben source below ./tmp")
        .with_prelude(
            r##"(progn
                   (provide 'pdf-annot)
                   (provide 'mu4e))"##,
        )
        .with_timeout(ABGABEN_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed abgaben parity test")
        .into()
}

pub(crate) fn assert_abgaben_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abgaben_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("abgaben parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
