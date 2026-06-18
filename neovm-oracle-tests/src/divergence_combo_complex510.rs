/// Batch 510: character operation edge cases — zero-width, combining, SMP, surrogates.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx510_char_width_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (char-width #x0300) (char-width #x200B) (char-width #x200D))
"##,
    );
}

#[test]
fn div_cx510_char_width_wide() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (char-width #x11000) (char-width #x1F600) (char-width #x2A600))
"##,
    );
}

#[test]
fn div_cx510_char_bytes_extreme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "a cafe \U0001F600 end"))
  (list (length s) (string-bytes s) (string-width s)))
"##,
    );
}

#[test]
fn div_cx510_string_width_smp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "abc\U0001F600\U0001F601\U0001F602"))
  (list (length s) (string-width s) (string-bytes s)))
"##,
    );
}

#[test]
fn div_cx510_string_width_zero_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "a\u0301e\u0300o\u0302"))
  (list (length s) (string-width s) (string-bytes s)))
"##,
    );
}

#[test]
fn div_cx510_char_direction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (get-char-code-property ?c 'bidi-class)
      (get-char-code-property ?\u05D0 'bidi-class)
      (get-char-code-property ?\u0600 'bidi-class))
"##,
    );
}

#[test]
fn div_cx510_char_mirrored() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (get-char-code-property ?\( 'mirror)
      (get-char-code-property ?\) 'mirror)
      (get-char-code-property ?< 'mirror))
"##,
    );
}

#[test]
fn div_cx510_char_numeric() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (get-char-code-property ?5 'numeric-value)
      (get-char-code-property ?0 'numeric-value)
      (get-char-code-property #x2150 'numeric-value))
"##,
    );
}

#[test]
fn div_cx510_char_combining_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (get-char-code-property ?a 'canonical-combining-class)
      (get-char-code-property #x0300 'canonical-combining-class))
"##,
    );
}

#[test]
fn div_cx510_char_decomposition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (get-char-code-property #xC0 'decomposition)
      (get-char-code-property #xE9 'decomposition))
"##,
    );
}

#[test]
fn div_cx510_char_uppercase_lowercase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (get-char-code-property ?a 'uppercase)
      (get-char-code-property ?A 'lowercase)
      (get-char-code-property ?a 'lowercase))
"##,
    );
}

#[test]
fn div_cx510_char_titlecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (get-char-code-property ?a 'titlecase)
      (get-char-code-property #x01C5 'titlecase))
"##,
    );
}

#[test]
fn div_cx510_string_to_unibyte_surrogate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (string-to-unibyte "abc")
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx510_string_as_multibyte_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s (string-as-multibyte (string-as-unibyte "cafe"))))
  (list (string-bytes s) (length s) (string= s "cafe")))
"##,
    );
}

#[test]
fn div_cx510_make_string_zero_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (make-string 0 ?x) (string ?h ?e ?l ?l ?o) (length ""))
"##,
    );
}
