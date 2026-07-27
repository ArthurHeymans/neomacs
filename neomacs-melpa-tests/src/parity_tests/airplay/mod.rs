use std::time::Duration;

use crate::{AIRPLAY_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod device;
mod media;
mod protocol;
mod registry;
mod server;

const AIRPLAY_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn airplay_oracle(source_file: &str) -> CachedMelpaOracle {
    let dependency_boundary = match source_file {
        "airplay.el" => "(provide 'request-deferred)",
        "airplay-video-server.el" => {
            r##"(progn
                   (require 'cl)
                   (defvar httpd-status-codes nil)
                   (defvar httpd-mime-types
                     '(("txt" . "text/plain")))
                   (defun httpd-get-mime (extension)
                     (or (cdr (assoc extension httpd-mime-types))
                         "application/octet-stream"))
                   (provide 'simple-httpd))"##
        }
        _ => "",
    };
    CachedMelpaOracle::new(AIRPLAY_MELPA_PIN, source_file)
        .expect("prepare pinned airplay source below ./tmp")
        .with_prelude(dependency_boundary)
        .with_timeout(AIRPLAY_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed airplay parity test")
        .into()
}

fn assert_airplay_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = airplay_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("airplay parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_airplay_parity(elisp_form: &str, expected: Expect) {
    assert_airplay_source_parity("airplay.el", elisp_form, expected);
}

pub(crate) fn assert_airplay_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_airplay_source_parity("airplay-autoloads.el", elisp_form, expected);
}

pub(crate) fn assert_airplay_server_parity(elisp_form: &str, expected: Expect) {
    assert_airplay_source_parity("airplay-video-server.el", elisp_form, expected);
}
