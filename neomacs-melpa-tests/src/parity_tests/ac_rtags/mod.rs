use std::time::Duration;

use crate::{AC_RTAGS_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod actions;
mod callables;
mod candidates;
mod surface;

const AC_RTAGS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_rtags_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_RTAGS_MELPA_PIN, source_file)
        .expect("prepare pinned ac-rtags source below ./tmp")
        .with_timeout(AC_RTAGS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-rtags parity test")
        .into()
}

pub(crate) fn assert_ac_rtags_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_rtags_oracle("ac-rtags.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-rtags parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_rtags_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_rtags_oracle("ac-rtags-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-rtags autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ac_rtags_exact_pin_dependencies_features_group_option_hook_and_source_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-rtags
                      package-alist))))
               (list
                (package-desc-name
                 descriptor)
                (package-version-join
                 (package-desc-version
                  descriptor))
                (package-desc-reqs
                 descriptor)
                (package-desc-summary
                 descriptor)
                (copy-tree
                 (package-desc-extras
                  descriptor))
                (mapcar
                 #'featurep
                 '(ac-rtags
                   rtags
                   auto-complete))
                (get
                 'ac-rtags
                 'group-documentation)
                (get
                 'ac-rtags
                 'custom-prefix)
                (mapcar
                 (lambda (parent)
                   (assq
                    'ac-rtags
                    (get parent
                         'custom-group)))
                 '(ac
                   rtags))
                (get
                 'ac-rtags
                 'custom-links)
                (list
                 ac-rtags-expand-functions
                 (get
                  'ac-rtags-expand-functions
                  'standard-value)
                 (get
                  'ac-rtags-expand-functions
                  'custom-type)
                 (get
                  'ac-rtags-expand-functions
                  'variable-documentation)
                 (assq
                  'ac-rtags-expand-functions
                  (get
                   'ac-rtags
                   'custom-group)))
                (memq
                 'ac-rtags-completions-hook
                 rtags-completions-hook)
                ac-source-rtags))"##;
    let expect = expect![[
        r#"OK (ac-rtags "20191222.920" ((auto-complete (1 4 0)) (rtags (2 10))) "Auto-complete back-end for RTags." ((:maintainers ("Jan Erik Hanssen" . "jhanssen@gmail.com") ("Anders Bakken" . "agbakken@gmail.com")) (:authors ("Jan Erik Hanssen" . "jhanssen@gmail.com") ("Anders Bakken" . "agbakken@gmail.com")) (:revdesc . "595055b5316a") (:commit . "595055b5316a7c92ba1d638f324f98842a0f41a5") (:url . "https://github.com/Andersbakken/rtags")) (t t t) "Auto completion back-end for RTags." "rtags-" ((ac-rtags custom-group) (ac-rtags custom-group)) ((url-link :tag "Website" "https://github.com/Andersbakken/rtags")) (t ((funcall #'#[nil (t) (t)])) boolean "Whether to expand function parameter lists in `auto-complete' mode." (ac-rtags-expand-functions custom-variable)) (ac-rtags-completions-hook) ((init . ac-rtags-init) (prefix . ac-rtags-prefix) (candidates . ac-rtags-candidates) (action . ac-rtags-action) (document . ac-rtags-document) (requires . 0) (symbol . "r")))"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}
