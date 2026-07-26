use std::time::Duration;

use crate::{AC_SLY_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod callables;
mod candidates;
mod setup;
mod surface;
mod transformations;

const AC_SLY_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_sly_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_SLY_MELPA_PIN, source_file)
        .expect("prepare pinned ac-sly source below ./tmp")
        .with_timeout(AC_SLY_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ac-sly parity test").into()
}

pub(crate) fn assert_ac_sly_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_sly_oracle("ac-sly.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-sly parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_sly_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_sly_oracle("ac-sly.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-sly signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_sly_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_sly_oracle("ac-sly-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-sly autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ac_sly_exact_pin_dependencies_features_group_option_faces_and_sources_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-sly
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
                 '(ac-sly
                   cl-lib
                   sly
                   auto-complete))
                (get
                 'ac-sly
                 'group-documentation)
                (get
                 'ac-sly
                 'custom-prefix)
                (assq
                 'ac-sly
                 (get
                  'sly
                  'custom-group))
                (list
                 ac-sly-show-flags
                 (get
                  'ac-sly-show-flags
                  'standard-value)
                 (get
                  'ac-sly-show-flags
                  'custom-type)
                 (get
                  'ac-sly-show-flags
                  'variable-documentation)
                 (assq
                  'ac-sly-show-flags
                  (get
                   'ac-sly
                   'custom-group)))
                (mapcar
                 (lambda (face)
                   (list
                    face
                    (get
                     face
                     'face-defface-spec)
                    (get
                     face
                     'face-documentation)
                    (assq
                     face
                     (get
                      'auto-complete
                      'custom-group))))
                 '(ac-sly-menu-face
                   ac-sly-selection-face))
                ac-source-sly-fuzzy
                ac-source-sly-simple))"##;
    let expect = expect![[
        r#"OK (ac-sly "20170728.1027" ((sly (1 0 0 -3)) (auto-complete (1 4)) (cl-lib (0 5))) "An auto-complete source using sly completions." ((:maintainers ("Damian T. Dobroczy\\'nski" . "qoocku@gmail.com")) (:authors ("Damian T. Dobroczy\\'nski" . "qoocku@gmail.com")) (:revdesc . "bf69c687c4ec") (:commit . "bf69c687c4ecf1994349d20c182e9b567399912e") (:url . "https://github.com/qoocku/ac-sly")) (t t t t) "Sly auto-complete customizations" "ac-sly-" (ac-sly custom-group) (t (t) nil "When non-nil, show completion result flags during fuzzy completion." (ac-sly-show-flags custom-variable)) ((ac-sly-menu-face ((t (:inherit ac-candidate-face))) "Face for slime candidate menu." (ac-sly-menu-face custom-face)) (ac-sly-selection-face ((t (:inherit ac-selection-face))) "Face for the slime selected candidate." (ac-sly-selection-face custom-face))) ((init . ac-sly-init) (candidates . ac-source-sly-fuzzy-candidates) (candidate-face . ac-sly-menu-face) (selection-face . ac-sly-selection-face) (prefix . sly-symbol-start-pos) (symbol . "l") (match lambda (prefix candidates) candidates) (document . ac-sly-documentation)) ((init . ac-sly-init) (candidates . ac-source-sly-simple-candidates) (candidate-face . ac-sly-menu-face) (selection-face . ac-sly-selection-face) (prefix . sly-symbol-start-pos) (symbol . "l") (document . ac-sly-documentation) (match . ac-source-sly-case-correcting-completions)))"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}
