/// Batch 472: ebdb, bbdb, lookup, dict, ispell, flyspell, wdired, image-dired.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx472_ebdb_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (require 'ebdb) (error (car e)))
"##,
    );
}

#[test]
fn div_cx472_lookup_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (require 'lookup) (error (car e)))
"##,
    );
}

#[test]
fn div_cx472_dict_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (require 'dictionary) (error (car e)))
"##,
    );
}

#[test]
fn div_cx472_ispell_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ispell)
  (list (boundp 'ispell-dictionary) (fboundp 'ispell-word)))
"##,
    );
}

#[test]
fn div_cx472_flyspell_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'flyspell)
  (list (boundp 'flyspell-mode) (fboundp 'flyspell-buffer)))
"##,
    );
}

#[test]
fn div_cx472_wdired_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'wdired)
  (list (fboundp 'wdired-change-to-wdired-mode)
        (boundp 'wdired-finish-hook)))
"##,
    );
}

#[test]
fn div_cx472_image_dired_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'image-dired)
  (list (fboundp 'image-dired-dired-with-window-configuration)
        (boundp 'image-dired-display-image-buffer)))
"##,
    );
}

#[test]
fn div_cx472_calendar_diary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'calendar) (require 'diary-lib)
  (list (boundp 'diary-file) (fboundp 'diary)))
"##,
    );
}

#[test]
fn div_cx472_appt_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'appt)
  (list (boundp 'appt-time-msg-list) (fboundp 'appt-activate)))
"##,
    );
}

#[test]
fn div_cx472_erc_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'erc)
  (list (fboundp 'erc) (boundp 'erc-modules)))
"##,
    );
}

#[test]
fn div_cx472_rcirc_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'rcirc)
  (list (fboundp 'rcirc-connect) (boundp 'rcirc-server-alist)))
"##,
    );
}

#[test]
fn div_cx472_newsticker_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'newsticker)
  (list (fboundp 'newsticker-start) (boundp 'newsticker-treeview)))
"##,
    );
}

#[test]
fn div_cx472_soap_client_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (require 'soap-client) (error (car e)))
"##,
    );
}

#[test]
fn div_cx472_morse_code() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'morse)
  (list (fboundp 'morse-region) (fboundp 'unmorse-region)))
"##,
    );
}

#[test]
fn div_cx472_sound_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'sound)
  (list (fboundp 'play-sound) (boundp 'play-sound-file)))
"##,
    );
}
