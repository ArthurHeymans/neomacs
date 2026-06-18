//! Unicode character-property parity (get-char-code-property over the unidata
//! tables): general-category, name, numeric-value, bidi-class, decomposition,
//! uppercase/lowercase, mirroring, canonical-combining-class; char-width across
//! scripts, char-script-table, char-category-set/mnemonics.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn bidi_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?A 'bidi-class)
        (get-char-code-property ?א 'bidi-class)
        (get-char-code-property ?5 'bidi-class))"##,
    );
}

#[test]
fn category_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (char-category-set ?A) (category-set-mnemonics (char-category-set ?A)))"##,
    );
}

#[test]
fn char_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?A 'name)
        (get-char-code-property ?λ 'name)
        (get-char-code-property ?€ 'name))"##,
    );
}

#[test]
fn char_script() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (aref char-script-table ?A) (aref char-script-table ?日)
        (aref char-script-table ?α))"##,
    );
}

#[test]
fn char_width_scripts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (char-width ?A) (char-width ?日) (char-width ?ｱ)
        (char-width ?́) (string-width "á") (string-width "日本"))"##,
    );
}

#[test]
fn decomposition() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?é 'decomposition)
        (get-char-code-property ?A 'decomposition)
        (get-char-code-property ?ﬁ 'decomposition))"##,
    );
}

#[test]
fn general_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?A 'general-category)
        (get-char-code-property ?a 'general-category)
        (get-char-code-property ?5 'general-category)
        (get-char-code-property ?\s 'general-category)
        (get-char-code-property ?. 'general-category))"##,
    );
}

#[test]
fn mirroring_canonical() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?\( 'mirroring)
        (get-char-code-property ?A 'canonical-combining-class)
        (get-char-code-property ?́ 'canonical-combining-class))"##,
    );
}

#[test]
fn numeric_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?5 'numeric-value)
        (get-char-code-property ?Ⅻ 'numeric-value)
        (get-char-code-property ?½ 'numeric-value))"##,
    );
}

#[test]
fn uppercase_lowercase_prop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?a 'uppercase)
        (get-char-code-property ?A 'lowercase)
        (get-char-code-property ?5 'uppercase))"##,
    );
}
