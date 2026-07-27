use std::time::Duration;

use crate::{ADDRESSBOOK_BOOKMARK_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod completion;
mod display;
mod mail;
mod model;
mod records;
mod registry;

const ADDRESSBOOK_BOOKMARK_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn addressbook_bookmark_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ADDRESSBOOK_BOOKMARK_MELPA_PIN, source_file)
        .expect("prepare pinned addressbook-bookmark source below ./tmp")
        .with_timeout(ADDRESSBOOK_BOOKMARK_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed addressbook-bookmark parity test")
        .into()
}

fn assert_addressbook_bookmark_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = addressbook_bookmark_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("addressbook-bookmark parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_addressbook_bookmark_parity(elisp_form: &str, expected: Expect) {
    assert_addressbook_bookmark_source_parity("addressbook-bookmark.el", elisp_form, expected);
}

pub(crate) fn assert_addressbook_bookmark_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_addressbook_bookmark_source_parity(
        "addressbook-bookmark-autoloads.el",
        elisp_form,
        expected,
    );
}
