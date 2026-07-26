use std::time::Duration;

use crate::{ACCENT_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod callables;
mod company;
mod corfu;
mod list;
mod menu;
mod surface;

const ACCENT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn accent_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACCENT_MELPA_PIN, source_file)
        .expect("prepare pinned accent source below ./tmp")
        .with_timeout(ACCENT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed accent parity test").into()
}

pub(crate) fn assert_accent_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = accent_oracle("accent.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("accent parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_accent_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = accent_oracle("accent-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("accent autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn accent_exact_pin_dependencies_feature_group_options_version_and_diacritics_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'accent
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
                (featurep
                 'accent)
                accent-version
                (get
                 'accent-version
                 'variable-documentation)
                (get
                 'accent
                 'group-documentation)
                (assq
                 'accent
                 (get
                  'convenience
                  'custom-group))
                (list
                 accent-position
                 (get
                  'accent-position
                  'standard-value)
                 (get
                  'accent-position
                  'custom-type)
                 (get
                  'accent-position
                  'variable-documentation)
                 (assq
                  'accent-position
                  (get
                   'accent
                   'custom-group)))
                (list
                 accent-custom
                 (get
                  'accent-custom
                  'standard-value)
                 (get
                  'accent-custom
                  'custom-type)
                 (get
                  'accent-custom
                  'variable-documentation)
                 (assq
                  'accent-custom
                  (get
                   'accent
                   'custom-group)))
                accent-diacritics))"##;
    let expect = expect![[
        r#"OK (accent "20250210.906" ((emacs (24 3)) (popup (0 5 8))) "Popup for accented characters (diacritics)." ((:maintainers ("Elia Scotto" . "eliascotto94@gmail.com")) (:authors ("Elia Scotto" . "eliascotto94@gmail.com")) (:keywords "i18n") (:revdesc . "d613700dc415") (:commit . "d613700dc4159692f5c30dc5f241c9de41bbb1dc") (:url . "https://github.com/elias94/accent")) t "1.4" "Version of accent.el." "Shows popup with accented letters while pressing C-x C-a on an\naccented character." (accent custom-group) (before ((funcall #'#[nil ('before) #1=(t)])) symbol "If set to 'before (default) it takes the character before the cursor.\nIf set to 'after it takes the caracter after the cursor. Set it to 'after\nif you have the `cursor-type` set to 'block and want to apply an accent to\nthe character under the cursor." (accent-position custom-variable)) (nil ((funcall #'#[nil ('nil) #1#])) (alist :value-type (character (alist :value-type character))) "Used to append custom accented characters to the default one.\nIt uses a list of characters associated to a single letter,\ne.g. '(a (ằ)) ." (accent-custom custom-variable)) ((a (à á â ä æ ã å ā)) (c (ç ć č)) (e (è é ê ë ē ė ę)) (i (î ï í ī į ì)) (l (ł)) (n (ñ ń)) (o (ô ö ò ó œ ø ō õ)) (s (ß ś š)) (u (û ü ù ú ū)) (y (ÿ)) (z (ž ź ż)) (A (À Á Â Ä Æ Ã Å Ā)) (C (Ç Ć Č)) (E (È É Ê Ë Ē Ė Ę)) (I (Î Ï Í Ī Į Ì)) (L (Ł)) (N (Ñ Ń)) (O (Ô Ö Ò Ó Œ Ø Ō Õ)) (S (Ś Š)) (U (Û Ü Ù Ú Ū)) (Y (Ÿ)) (Z (Ž Ź Ż))))"#
    ]];

    assert_accent_parity(elisp_form, expect);
}
