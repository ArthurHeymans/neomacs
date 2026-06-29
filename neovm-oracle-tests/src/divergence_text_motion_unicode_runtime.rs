//! Text motion + casing over Unicode parity: forward-word over CJK/mixed
//! punctuation, count-words mixed scripts, forward-char over combining marks,
//! forward-sentence with CJK, special-case upcase/downcase (ß/ﬁ/İ/digraphs),
//! title-case capitalize, Turkish-i, word-at-point CJK, backward-word.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn backward_word_motion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "one two three")
  (goto-char (point-max))
  (list (progn (backward-word) (point)) (progn (backward-word) (point))))"##,
        expect_test::expect![[r#""OK (9 5)""#]],
    );
}

#[test]
fn char_motion_combining() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "a" (string ?e #x0301) "b")
  (goto-char (point-min))
  (list (progn (forward-char 1) (point)) (progn (forward-char 1) (point))
        (point-max)))"##,
        expect_test::expect![[r#""OK (2 3 5)""#]],
    );
}

#[test]
fn count_words_mixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "hello 日本 world café")
  (list (count-words (point-min) (point-max))
        (count-lines (point-min) (point-max))))"##,
        expect_test::expect![[r#""OK (4 1)""#]],
    );
}

#[test]
fn downcase_turkish_i() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (downcase "I") (upcase "i") (downcase ?I) (char-equal ?i ?I))"##,
        expect_test::expect![[r#""OK (\"i\" \"I\" 105 t)""#]],
    );
}

#[test]
fn forward_sentence_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "First sentence. 日本語の文。Third one.")
  (goto-char (point-min))
  (list (progn (forward-sentence) (point)) (progn (forward-sentence) (point))))"##,
        expect_test::expect![[r#""OK (23 33)""#]],
    );
}

#[test]
fn forward_word_cjk() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "hello 日本語 world")
  (goto-char (point-min))
  (list (progn (forward-word) (point)) (progn (forward-word) (point)) (progn (forward-word) (point))))"##,
        expect_test::expect![[r#""OK (6 10 16)""#]],
    );
}

#[test]
fn forward_word_mixed_punct() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "foo-bar_baz.qux")
  (goto-char (point-min))
  (let (pts) (while (and (< (point) (point-max)) (forward-word 1)) (push (point) pts)) (nreverse pts)))"##,
        expect_test::expect![[r#""OK (4 8 12 16)""#]],
    );
}

#[test]
fn special_case_upcase() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (upcase "ß") (upcase "ﬁ") (downcase "İ") (upcase "ﬀ")
        (capitalize "ǆ") (upcase ?ǳ))"##,
        expect_test::expect![[r#""OK (\"SS\" \"FI\" \"i\u{307}\" \"FF\" \"ǅ\" 497)""#]],
    );
}

#[test]
fn title_case_capitalize() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (capitalize "hello WORLD foo") (upcase-initials "the-quick brown")
        (capitalize "ﬂower") (capitalize "123abc def"))"##,
        expect_test::expect![[
            r#""OK (\"Hello World Foo\" \"The-Quick Brown\" \"Flower\" \"123abc Def\")""#
        ]],
    );
}

#[test]
fn word_at_point_cjk() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "test 日本語 end")
  (goto-char 8)
  (list (thing-at-point 'word t) (current-word)))"##,
        expect_test::expect![[r#""OK (\"日本語\" \"日本語\")""#]],
    );
}
