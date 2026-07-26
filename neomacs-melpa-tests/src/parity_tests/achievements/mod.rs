use std::time::Duration;

use crate::{ACHIEVEMENTS_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod catalogs;
mod display;
mod frequency;
mod lifecycle;
mod macros;
mod mode;
mod persistence;
mod scoring;
mod surface;

const ACHIEVEMENTS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn achievements_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACHIEVEMENTS_MELPA_PIN, source_file)
        .expect("prepare pinned achievements source below ./tmp")
        .with_timeout(ACHIEVEMENTS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed achievements parity test")
        .into()
}

fn assert_achievements_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = achievements_oracle("achievements.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("achievements parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_achievements_functions_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = achievements_oracle("achievements-functions.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("achievements-functions parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_achievements_autoload_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = achievements_oracle("achievements-autoloads.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("achievements autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_advanced_achievements_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = achievements_oracle("advanced-achievements.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("advanced-achievements parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn achievements_exact_pin_dependencies_metadata_and_features_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'achievements
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (mapcar
                 #'featurep
                 '(achievements-functions
                   basic-achievements
                   achievements))))"##;
    let expect = expect![[
        r#"OK (achievements "20240703.318" ((keyfreq (0 0 3))) "Achievements for emacs usage." ((:maintainers ("Ivan Andrus" . "darthandrus@gmail.com")) (:authors ("Ivan Andrus" . "darthandrus@gmail.com")) (:keywords "games") (:revdesc . "c229d21ad5d1") (:commit . "c229d21ad5d1e13be08e087ab498800b2b9b7c97") (:url . "https://gitlab.com/gvol/emacs-achievements")) (t t t))"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}
