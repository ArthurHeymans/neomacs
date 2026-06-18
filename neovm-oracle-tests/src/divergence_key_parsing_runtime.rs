//! Key parsing/description parity: kbd of chords/function/mouse/modifier keys,
//! key-description roundtrip, key-valid-p, key-parse, listify-key-sequence,
//! single-key-description, kbd edge (DEL/ESC/C-?/C-SPC), event-modifiers/
//! basic-type, global key-binding lookup.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn event_modifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (event-modifiers ?\C-a) (event-modifiers ?\M-\C-a)
        (event-basic-type ?\C-a) (event-basic-type ?\M-b))"##,
    );
}

#[test]
fn kbd_edge_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (kbd "DEL") (kbd "ESC") (kbd "C-?") (kbd "C-SPC") (kbd "<backspace>"))"##,
    );
}

#[test]
fn kbd_function_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (kbd "<mouse-1>") (kbd "<down>") (kbd "<C-up>")
        (kbd "M-<f7>") (kbd "S-<tab>"))"##,
    );
}

#[test]
fn kbd_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (kbd "C-x C-c") (kbd "M-x") (kbd "<f5>") (kbd "C-M-a")
        (kbd "RET") (kbd "TAB") (kbd "SPC") (kbd "C-c C-x C-v"))"##,
    );
}

#[test]
fn key_binding_global() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (eq (key-binding (kbd "C-f")) 'forward-char)
        (eq (key-binding (kbd "C-x C-f")) 'find-file)
        (commandp (key-binding (kbd "C-a"))))"##,
    );
}

#[test]
fn key_description_prefix() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (key-description [?\C-x] [?\C-c])
        (single-key-description ?\C-a) (single-key-description ?\M-x))"##,
    );
}

#[test]
fn key_description_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (key-description (kbd "C-x C-c")) (key-description (kbd "M-RET"))
        (key-description (kbd "<f1>")) (key-description [?\C-a ?\M-b]))"##,
    );
}

#[test]
fn key_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (list (key-parse "C-x C-c") (key-parse "M-x") (key-parse "RET")) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn key_valid_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(condition-case e (list (key-valid-p "C-x C-c") (key-valid-p "C-xC-c")
        (key-valid-p "<f5>") (key-valid-p "RET")) (error (cons (quote ERR) (car e))))"##,
    );
}

#[test]
fn listify_key_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (listify-key-sequence (kbd "C-a")) (listify-key-sequence (kbd "abc"))
        (listify-key-sequence [?\M-a]))"##,
    );
}
