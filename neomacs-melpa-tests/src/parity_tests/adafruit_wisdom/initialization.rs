use expect_test::expect;

use super::{assert_adafruit_wisdom_no_littering_parity, assert_adafruit_wisdom_parity};

#[test]
fn adafruit_wisdom_default_cache_path_uses_isolated_user_emacs_directory() {
    let elisp_form = r##"(list
         adafruit-wisdom-cache-file
         user-emacs-directory
         (equal
          adafruit-wisdom-cache-file
          (locate-user-emacs-file
           "adafruit-wisdom.cache"))
         (file-in-directory-p
          adafruit-wisdom-cache-file
          user-emacs-directory)
         (featurep
          'no-littering))"##;
    let expect = expect![[r#"OK ("~/.emacs.d/adafruit-wisdom.cache" "~/.emacs.d/" t t nil)"#]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_no_littering_feature_selects_the_expanded_var_cache_path_at_load_time() {
    let elisp_form = r##"(list
         adafruit-wisdom-cache-file
         (equal
          adafruit-wisdom-cache-file
          (expand-file-name
           "var/no-littering/adafruit-wisdom.cache"
           user-emacs-directory))
         (file-in-directory-p
          adafruit-wisdom-cache-file
          user-emacs-directory)
         (featurep
          'no-littering))"##;
    let expect =
        expect![[r#"OK ("[ORACLE-HOME]/.emacs.d/var/no-littering/adafruit-wisdom.cache" t t t)"#]];
    assert_adafruit_wisdom_no_littering_parity(elisp_form, expect);
}
