use std::time::Duration;

use crate::{ALIGN_CLJLET_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod alignment;
mod defroutes;
mod errors;
mod primitives;
mod registry;

const ALIGN_CLJLET_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn align_cljlet_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALIGN_CLJLET_MELPA_PIN, source_file)
        .expect("prepare pinned align-cljlet source below ./tmp")
        .with_prelude(r##"(require 'clojure-mode)"##)
        .with_timeout(ALIGN_CLJLET_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed align-cljlet parity test")
        .into()
}

fn assert_align_cljlet_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = align_cljlet_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("align-cljlet parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_align_cljlet_parity(elisp_form: &str, expected: Expect) {
    assert_align_cljlet_source_parity("align-cljlet.el", elisp_form, expected);
}

pub(crate) fn assert_align_cljlet_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_align_cljlet_source_parity("align-cljlet-autoloads.el", elisp_form, expected);
}
