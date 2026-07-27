use std::time::Duration;

use crate::{AIO_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod async_flow;
mod boundaries;
mod promise;
mod select_sem;
mod surface;

const AIO_TEST_TIMEOUT: Duration = Duration::from_secs(60);

fn aio_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AIO_MELPA_PIN, "aio.el")
        .expect("prepare pinned aio source below ./tmp")
        .with_timeout(AIO_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed aio parity test").into()
}

pub(crate) fn assert_aio_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aio_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aio parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
