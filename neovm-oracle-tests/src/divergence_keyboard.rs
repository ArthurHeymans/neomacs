//! Keyboard pure-function coverage (thin area: ~3 prior files).
//!
//! Deterministic, non-blocking keyboard ops: kbd parsing, key-description,
//! single-key-description, event-modifiers/event-basic-type/event-convert-list,
//! key-valid-p, kmacro construction/keys/counter/format. Avoids read-*
//! (blocks on EOF) and interactive input.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_kb_kbd_parse_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (equal (kbd "C-c") (kbd "C-c"))
      (stringp (kbd "abc"))
      (vectorp (kbd "C-c"))
      (equal (kbd "RET") (kbd "<return>"))
      (equal (kbd "C-m") [13])
      (length (kbd "C-c C-c")))
"##,
    );
}

#[test]
fn div_kb_key_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (key-description (kbd "C-c C-x"))
      (key-description [?a 13])
      (key-description (kbd "M-x"))
      (key-description (kbd "C-c C-c") "prefix"))
"##,
    );
}

#[test]
fn div_kb_single_key_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (single-key-description ?a)
      (single-key-description 1)
      (single-key-description ?\M-a)
      (single-key-description 13)
      (single-key-description ?\C-\M-a))
"##,
    );
}

#[test]
fn div_kb_event_modifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (event-modifiers ?\C-a)
      (event-modifiers ?\M-a)
      (event-modifiers ?\C-\M-a)
      (event-modifiers ?a)
      (event-modifiers 'mouse-1))
"##,
    );
}

#[test]
fn div_kb_event_basic_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (event-basic-type ?\C-a)
      (event-basic-type ?\M-a)
      (event-basic-type ?\S-a)
      (event-basic-type 'mouse-1))
"##,
    );
}

#[test]
fn div_kb_event_convert_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (event-convert-list (list 'control ?a))
      (event-convert-list (list 'meta control ?a))
      (event-convert-list (list 'shift 'mouse-1)))
"##,
    );
}

#[test]
fn div_kb_key_valid_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (key-valid-p "C-c") (key-valid-p "abc") (key-valid-p "<f5>")
      (key-valid-p "C-x C-c") (key-valid-p "C-xyz") (key-valid-p "M-<"))
"##,
    );
}

#[test]
fn div_kb_kmacro_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn (require 'kmacro)
           (let ((km (kmacro "abc")))
             (list (kmacro-p km) (kmacro-keys km))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_kb_kmacro_counter_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn (require 'kmacro)
           (let ((km (kmacro (kbd "C-a") 5 "d")))
             (list (kmacro-counter km) (kmacro-format km))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_kb_kmacro_definition_and_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn (require 'kmacro)
           (let ((km (kmacro "xyz")))
             (list (car (kmacro-definition km))
                   (length (kmacro-definition km))
                   (kmacro-single-p km))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_kb_event_symbol_and_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (event-modifiers 'down-mouse-1)
      (event-modifiers 'S-mouse-3)
      (event-basic-type 'down-mouse-1)
      (event-basic-type 'S-mouse-3))
"##,
    );
}

#[test]
fn div_kb_describe_bindings_structure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((s (describe-buffer-bindings (current-buffer))))
        (if (stringp s) (length s) s)))
  (error (cons 'errored (car e))))
"##,
    );
}
