//! Divergence tests: keyboard macros, input decoding, keymaps deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_keymap_parent_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((parent (make-sparse-keymap))
        (child (make-sparse-keymap)))
  (define-key parent "a" 'parent-cmd)
  (define-key child "b" 'child-cmd)
  (set-keymap-parent child parent)
  (list (lookup-key child "a")
        (lookup-key child "b")
        (lookup-key parent "b")
        (keymap-parent child)))"#,
    );
}

#[test]
fn divergence_keymap_prefix_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((map (make-sparse-keymap)))
  (define-key map "C-c" (make-sparse-keymap))
  (define-key map "C-c a" 'cmd-a)
  (define-key map "C-c b" 'cmd-b)
  (list (lookup-key map "C-c")
        (lookup-key map "C-c a")
        (lookup-key map "C-c b")
        (key-binding "C-c a")
        (length map)))"#,
    );
}

#[test]
fn divergence_keymap_where_is() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (stringp (key-description (where-is-internal 'forward-char nil t)))
  (consp (where-is-internal 'forward-char))
  (>= (length (where-is-internal 'forward-char)) 1))"#,
    );
}

#[test]
fn divergence_command_execute_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (commandp 'forward-char)
  (commandp (lambda () (interactive) nil))
  (commandp 'nonexistent-cmd-xyz)
  (commandp 'car))"#,
    );
}

#[test]
fn divergence_this_command_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'this-command-keys)
  (fboundp 'this-command-keys-vector)
  (fboundp 'recent-keys)
  (fboundp 'open-dribble-file))"#,
    );
}

#[test]
fn divergence_input_methods() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'activate-input-method)
  (fboundp 'deactivate-input-method)
  (fboundp 'toggle-input-method)
  (boundp 'current-input-method)
  (stringp current-input-method))"#,
    );
}

#[test]
fn divergence_key_translation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'keyboard-translate)
  (boundp 'keyboard-translate-table)
  (fboundp 'define-keyboard-macro))"#,
    );
}

#[test]
fn divergence_read_key_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'read-key-sequence)
  (fboundp 'read-key-sequence-vector)
  (fboundp 'read-event)
  (fboundp 'read-char))"#,
    );
}

#[test]
fn divergence_accessed_keymaps() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (keymapp (current-global-map))
  (keymapp (current-local-map))
  (listp (current-global-map))
  (listp (current-local-map)))"#,
    );
}

#[test]
fn divergence_minor_mode_map() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (listp minor-mode-map-alist)
  (listp minor-mode-overriding-map-alist)
  (consp (assq 'override-global-mode minor-mode-map-alist)))"#,
    );
}
