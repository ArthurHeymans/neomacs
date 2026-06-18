/// Batch 512: further key description divergence characterization.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx512_key_desc_meta() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?\M-x) (single-key-description ?\M-c))
"##,
    );
}

#[test]
fn div_cx512_key_desc_ctrl() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?\C-x) (single-key-description ?\C-c))
"##,
    );
}

#[test]
fn div_cx512_key_desc_shift() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?\S-a) (single-key-description ?\S-z))
"##,
    );
}

#[test]
fn div_cx512_key_desc_meta_ctrl() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?\M-\C-x) (single-key-description ?\M-\C-c))
"##,
    );
}

#[test]
fn div_cx512_key_desc_hyper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?\H-x) (single-key-description ?\H-a))
"##,
    );
}

#[test]
fn div_cx512_key_desc_super() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?\s-x) (single-key-description ?\s-a))
"##,
    );
}

#[test]
fn div_cx512_key_desc_alt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?\A-x) (single-key-description ?\A-a))
"##,
    );
}

#[test]
fn div_cx512_key_desc_punctuation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?\M-!)
      (single-key-description ?\M-?)))
"##,
    );
}

#[test]
fn div_cx512_key_desc_function_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description [f1])
      (single-key-description [f12])
      (single-key-description [return])
      (single-key-description [tab]))
"##,
    );
}

#[test]
fn div_cx512_key_desc_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description [mouse-1])
      (single-key-description [down-mouse-2])
      (single-key-description [double-mouse-1]))
"##,
    );
}

#[test]
fn div_cx512_key_desc_combos() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description [C-M-a])
      (single-key-description [C-M-S-f1])
      (single-key-description [H-s-C-return]))
"##,
    );
}

#[test]
fn div_cx512_char_to_string_modifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (char-to-string ?\M-x)
      (char-to-string ?\C-x)
      (char-to-string ?\s-x))
"##,
    );
}

#[test]
fn div_cx512_text_char_desc_modifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (text-char-description ?\M-x)
      (text-char-description ?\C-x)
      (text-char-description ?\S-a))
"##,
    );
}

#[test]
fn div_cx512_key_binding_event() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (event-basic-type ?\M-x)
      (event-modifiers ?\M-x)
      (event-convert-list '(meta ?x)))
"##,
    );
}

#[test]
fn div_cx512_event_convert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (event-convert-list '(control ?f))
      (event-convert-list '(meta ?x))
      (event-convert-list '(control meta ?f)))
"##,
    );
}
