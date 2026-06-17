//! Complex combo batch 355 — `char-code-property` ultimate matrix:
//! general-category, bidi-class, decomposition, numeric-value, digit-value,
//! mirrored, name across Latin/Greek/CJK/emoji/RTL/combining chars.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx355_char_code_property_general_category_full() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (get-char-code-property c 'general-category)))
        '(?a ?A ?0 ?1 ?  ?! ?, ?. ?? ?( ?) ?-
          ?à ?é ?ü ?ñ ?Ä ?ç ?Å ?Æ ?Œ
          ?α ?β ?γ ?Α ?Β ?Γ
          ?世 ?界 ?日 ?本 ?語
          ?א ?ב ?ג ?ا ?ب ?ج
          ?\n ?\t ?_ ?" ?' ?\\ ?# ?$ ?% ?& ?* ?+ ?< ?> ?@ ?/ ?| ?~ ?^))
"##,
    )
}

#[test]
fn div_cx355_char_code_property_numeric_and_digit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c)
          (list c
                (get-char-code-property c 'numeric-value)
                (get-char-code-property c 'decimal-digit-value)
                (get-char-code-property c 'digit-value)))
        '(?0 ?1 ?5 ?9))
"##,
    )
}

#[test]
fn div_cx355_char_code_property_mirrored() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (get-char-code-property c 'mirrored)))
        '(?( ?) ?[ ?] ?{ ?} ?< ?> ?« ?» ?‹ ?› ?a ?A ?0 ?  ?!))
"##,
    )
}

#[test]
fn div_cx355_char_code_property_bidi_class_full() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (get-char-code-property c 'bidi-class)))
        '(?a ?A ?0 ?1 ?  ?! ?- ?( ?)
          ?א ?ב ?ג ?ד ?ה
          ?ا ?ب ?ج ?د ?ه))
"##,
    )
}

#[test]
fn div_cx355_char_code_property_decomposition_full() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c)
          (let ((d (get-char-code-property c 'decomposition)))
            (list c d)))
        '(?à ?é ?ü ?ñ ?Ä ?ö ?Ç ?Å ?Æ ?Œ ?œ))
"##,
    )
}

#[test]
fn div_cx355_char_code_property_name_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (get-char-code-property c 'name))
        '(?a ?A ?0 ?  ?! ?( ?)
          ?à ?é ?α ?β ?Α ?Β
          ?世 ?界 ?日 ?😀 ?🎉 ?🌍))
"##,
    )
}

#[test]
fn div_cx355_char_script_full_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (char-script c)))
        '(?a ?A ?0 ?  ?! ?( ?-
          ?à ?é ?ñ ?Ç ?Æ
          ?α ?β ?Ω
          ?世 ?界 ?日 ?本 ?語 ?中 ?国 ?한 ?글
          ?א ?ב ?ا ?ب
          ?À ?É ?Ñ ?Ø ?Þ ?Ð))
"##,
    )
}

#[test]
fn div_cx355_char_width_emoji_and_variation_selectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (char-width c)))
        '(#xFE0E #xFE0F #x200D #x20E3 ?a ?世 ?😀 ?🎉 ?🌍))
"##,
    )
}

#[test]
fn div_cx355_string_width_with_emoji_and_zwj() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-width "😀")
      (string-width "🎉🌍")
      (string-width "hello 😀 world")
      (string-width "café 世界 😀")
      (string-width "")
      (length "😀")
      (string-bytes "😀"))
"##,
    )
}

#[test]
fn div_cx355_char_props_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((cats (mapcar (lambda (c) (get-char-code-property c 'general-category))
                    '(?a ?A ?0 ?! ?( ?- ?à ?α ?世))))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "Char-code-property mega café 世界 😀 test")
    (put-text-property 1 5 'face 'bold)
    (let ((m (set-marker (make-marker) 10))
          (ov (make-overlay 4 18)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 22)
      (let ((state (list cats
                         (mapcar #'char-script '(?a ?α ?世 ?é))
                         (mapcar #'char-width '(?a ?世 ?😀))
                         (get-char-code-property ?à 'decomposition)
                         (get-char-code-property ?世 'name)
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen()
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    )
}
