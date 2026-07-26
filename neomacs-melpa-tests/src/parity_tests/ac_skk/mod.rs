use std::time::Duration;

use crate::{AC_SKK_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod actions;
mod autoloads;
mod callables;
mod candidates;
mod hiracomp;
mod lifecycle;
mod surface;

const AC_SKK_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_skk_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_SKK_MELPA_PIN, source_file)
        .expect("prepare pinned ac-skk source below ./tmp")
        .with_timeout(AC_SKK_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ac-skk parity test").into()
}

pub(crate) fn assert_ac_skk_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_skk_oracle("ac-skk.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-skk parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_skk_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_skk_oracle("ac-skk.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-skk signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_skk_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_skk_oracle("ac-skk-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-skk autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ac_skk_exact_pin_dependencies_features_group_option_and_sources_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-skk
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
                 '(ac-skk
                   cl-lib
                   tinysegmenter
                   auto-complete
                   skk
                   context-skk
                   skk-comp))
                (get
                 'ac-skk
                 'group-documentation)
                (assq
                 'ac-skk
                 (get
                  'auto-complete
                  'custom-group))
                (list
                 ac-skk-special-sources
                 (get
                  'ac-skk-special-sources
                  'standard-value)
                 (get
                  'ac-skk-special-sources
                  'custom-type)
                 (get
                  'ac-skk-special-sources
                  'variable-documentation)
                 (assq
                  'ac-skk-special-sources
                  (get
                   'ac-skk
                   'custom-group)))
                ac-source-skk
                ac-source-skk-hiracomp))"##;
    let expect = expect![[
        r#"OK (ac-skk "20141230.119" ((auto-complete (1 3 1)) (ddskk (16 0 50)) (tinysegmenter (0)) (cl-lib (0 5))) "Auto-complete-mode source for DDSKK a.k.a Japanese input method." ((:authors ("lugecy" . "https://twitter.com/lugecy")) (:keywords "convenience" "auto-complete") (:revdesc . "d25a26593043") (:commit . "d25a265930430d080329789fb253d786c01dfa24") (:url . "https://github.com/myuhe/ac-skk.el")) (t t t t t t t) "Auto complete source for DDSKK" (ac-skk custom-group) (#1=(ac-source-skk ac-source-skk-hiracomp) ('#1#) (repeat symbol) "When non-nil, show completion result flags during fuzzy completion." (ac-skk-special-sources custom-variable)) ((prefix . ac-skk-prefix) (candidates . ac-skk-candidates) (match lambda (prefix cands) cands) (requires . 1) (symbol . "SKK")) ((prefix . ac-skk-prefix-hiracomp) (candidates . ac-skk-hiracomp-candidates) (match lambda (prefix cands) cands) (requires . 2) (symbol . "SKKH")))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}
