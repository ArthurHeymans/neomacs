/// Batch 511: display table, glyph, character display edge cases.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx511_display_table_standard() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((dt (copy-display-table (standard-display-table))))
  (display-table-p dt))
"##,
    );
}

#[test]
fn div_cx511_display_table_truncation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((dt (make-display-table)))
  (aset dt ?\^I (vector ?\s ?\s ?\s ?\s))
  (set-window-display-table (selected-window) dt)
  (with-temp-buffer
    (insert "a\tb")
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx511_glyph_code_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (make-glyph-code ?A 'bold)
      (make-glyph-code ?B nil)
      (glyph-char (make-glyph-code ?X 'italic))
      (glyph-face (make-glyph-code ?Z 'default)))
"##,
    );
}

#[test]
fn div_cx511_char_to_string_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (char-to-string ?a)
      (char-to-string ?\C-a)
      (char-to-string ?\M-a)
      (char-to-string ?\S-a))
"##,
    );
}

#[test]
fn div_cx511_single_key_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (single-key-description ?a)
      (single-key-description ?\C-x)
      (single-key-description ?\M-x)
      (single-key-description ?\S-x))
"##,
    );
}

#[test]
fn div_cx511_text_char_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (text-char-description ?\C-a)
      (text-char-description ?\n)
      (text-char-description ?\t))
"##,
    );
}

#[test]
fn div_cx511_key_description_simple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (key-description "\C-x\C-f")
      (key-description "\M-x")
      (key-description "\C-c"))
"##,
    );
}

#[test]
fn div_cx511_lookup_key_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((map (make-sparse-keymap)))
  (define-key map "a" 'forward-char)
  (lookup-key map "a"))
"##,
    );
}

#[test]
fn div_cx511_accessible_keymaps_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((map (make-sparse-keymap)))
  (define-key map "a" 'forward-char)
  (define-key map "b" 'backward-char)
  (length (accessible-keymaps map)))
"##,
    );
}

#[test]
fn div_cx511_copy_keymap_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((map (make-sparse-keymap)))
  (define-key map "a" 'fn1)
  (define-key map "b" 'fn2)
  (let ((copy (copy-keymap map)))
    (define-key map "a" 'fn3)
    (lookup-key copy "a")))
"##,
    );
}

#[test]
fn div_cx511_current_minor_mode_maps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((maps (current-minor-mode-maps)))
  (list (listp maps)))
"##,
    );
}

#[test]
fn div_cx511_minor_mode_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (fboundp 'minor-mode-key-binding)
      (fboundp 'global-key-binding)
      (fboundp 'local-key-binding))
"##,
    );
}

#[test]
fn div_cx511_define_prefix_command_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s (make-symbol "cx511-prefix")))
  (define-prefix-command s)
  (list (commandp s) (keymapp (symbol-value s))))
"##,
    );
}

#[test]
fn div_cx511_describe_keys_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'help)
  (list (fboundp 'describe-key)
        (fboundp 'describe-bindings)))
"##,
    );
}

#[test]
fn div_cx511_use_local_map() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (let ((m (make-sparse-keymap)))
    (define-key m "a" 'forward-word)
    (use-local-map m)
    (current-local-map)))
"##,
    );
}
