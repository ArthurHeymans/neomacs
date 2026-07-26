use std::time::Duration;

use crate::{AC_SLIME_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod callables;
mod candidates;
mod setup;
mod surface;
mod transformations;

const AC_SLIME_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_slime_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_SLIME_MELPA_PIN, source_file)
        .expect("prepare pinned ac-slime source below ./tmp")
        .with_timeout(AC_SLIME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-slime parity test")
        .into()
}

pub(crate) fn assert_ac_slime_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_slime_oracle("ac-slime.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-slime parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_slime_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_slime_oracle("ac-slime.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-slime signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_slime_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_slime_oracle("ac-slime-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-slime autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ac_slime_exact_pin_dependencies_features_group_option_faces_and_sources_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-slime
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
                 '(ac-slime
                   cl-lib
                   slime
                   auto-complete))
                (get
                 'ac-slime
                 'group-documentation)
                (get
                 'ac-slime
                 'custom-prefix)
                (assq
                 'ac-slime
                 (get
                  'slime
                  'custom-group))
                (list
                 ac-slime-show-flags
                 (get
                  'ac-slime-show-flags
                  'standard-value)
                 (get
                  'ac-slime-show-flags
                  'custom-type)
                 (get
                  'ac-slime-show-flags
                  'variable-documentation)
                 (assq
                  'ac-slime-show-flags
                  (get
                   'ac-slime
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
                 '(ac-slime-menu-face
                   ac-slime-selection-face))
                ac-source-slime-fuzzy
                ac-source-slime-simple))"##;
    let expect = expect![[
        r#"OK (ac-slime "20171027.2100" ((auto-complete (1 4)) (slime (2 9)) (cl-lib (0 5))) "An auto-complete source using slime completions." ((:maintainers ("Steve Purcell" . "steve@sanityinc.com")) (:authors ("Steve Purcell" . "steve@sanityinc.com")) (:revdesc . "a91f664510d3") (:commit . "a91f664510d3da24b02e87e4aa59d049483a6529") (:url . "https://github.com/purcell/ac-slime")) (t t t t) "Slime auto-complete customizations" "ac-slime-" (ac-slime custom-group) (t (t) nil "When non-nil, show completion result flags during fuzzy completion." (ac-slime-show-flags custom-variable)) ((ac-slime-menu-face ((t (:inherit ac-candidate-face))) "Face for slime candidate menu." (ac-slime-menu-face custom-face)) (ac-slime-selection-face ((t (:inherit ac-selection-face))) "Face for the slime selected candidate." (ac-slime-selection-face custom-face))) ((init . ac-slime-init) (candidates . ac-source-slime-fuzzy-candidates) (candidate-face . ac-slime-menu-face) (selection-face . ac-slime-selection-face) (prefix . slime-symbol-start-pos) (symbol . "l") (match lambda (prefix candidates) candidates) (document . ac-slime-documentation)) ((init . ac-slime-init) (candidates . ac-source-slime-simple-candidates) (candidate-face . ac-slime-menu-face) (selection-face . ac-slime-selection-face) (prefix . slime-symbol-start-pos) (symbol . "l") (document . ac-slime-documentation) (match . ac-source-slime-case-correcting-completions)))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}
