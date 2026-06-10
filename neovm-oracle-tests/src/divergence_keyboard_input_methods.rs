//! Divergence tests: keyboard input, key translation, input methods deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_key_translation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'keyboard-translate)
  (fboundp 'local-set-key)
  (fboundp 'global-set-key)
  (fboundp 'define-key))"#,
    );
}

#[test]
fn divergence_input_methods() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'activate-input-method)
  (fboundp 'deactivate-input-method)
  (fboundp 'current-input-method)
  (boundp 'current-input-method)
  (featurep 'leim))"#,
    );
}

#[test]
fn divergence_quail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'quail-select-package)
  (fboundp 'quail-set-keyboard-layout)
  (featurep 'quail))"#,
    );
}

#[test]
fn divergence_input_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'input-decode-map)
  (fboundp 'local-function-key-map)
  (fboundp 'function-key-map)
  (boundp 'input-decode-map))"#,
    );
}

#[test]
fn divergence_key_maps_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((map (make-sparse-keymap)))
  (define-key map "a" 'foo)
  (set-keymap-parent map (make-sparse-keymap))
  (list (keymapp map)
        (cdr map)
        (keymap-parent map))) "#,
    );
}

#[test]
fn divergence_event_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'eventp)
  (fboundp 'event-start)
  (fboundp 'event-end)
  (fboundp 'event-basic-type)
  (fboundp 'event-modifiers)
  (fboundp 'read-event)
  (fboundp 'read-key))"#,
    );
}

#[test]
fn divergence_recent_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'recent-keys)
  (fboundp 'this-command-keys)
  (fboundp 'this-command-keys-vector)
  (fboundp 'clear-this-command-keys))"#,
    );
}

#[test]
fn divergence_key_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'key-description)
  (fboundp 'describe-buffer-bindings)
  (fboundp 'where-is-internal)
  (stringp (key-description [?a ?b]))) "#,
    );
}

#[test]
fn divergence_parse_modifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'event-convert-list)
  (fboundp 'event-apply-modifier)
  (fboundp 'event-apply-hyper-modifier))"#,
    );
}

#[test]
fn divergence_keyboard_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'set-keyboard-coding-system)
  (fboundp 'keyboard-coding-system)
  (boundp 'keyboard-coding-system))"#,
    );
}
