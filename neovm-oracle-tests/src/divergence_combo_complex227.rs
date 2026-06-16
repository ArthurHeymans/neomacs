//! Complex combo batch 227 — `char` properties deep: `get-char-code-property`
//! with `general-category`, `bidi-class`, `decomposition`, `decimal-digit-value`,
//! `digit-value`, `numeric-value`, `mirrored`, `old-name`, `iso-10646-comment`.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx227_char_code_property_general_category_full() {
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
    );
}

#[test]
fn div_cx227_char_numeric_value_and_digit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c)
          (list c
                (get-char-code-property c 'numeric-value)
                (get-char-code-property c 'decimal-digit-value)
                (get-char-code-property c 'digit-value)))
        '(?0 ?1 ?5 ?9
          ?Ⅷ ?Ⅳ ?Ⅻ
          ?½ ?¼ ?¾))
"##,
    );
}

#[test]
fn div_cx227_char_mirrored_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (get-char-code-property c 'mirrored)))
        '(?( ?) ?[ ?] ?{ ?} ?< ?> ?« ?» ?‹ ?› ?a ?A ?0 ?  ?!))
"##,
    );
}

#[test]
fn div_cx227_char_bidi_class_full_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (get-char-code-property c 'bidi-class)))
        '(?a ?A ?0 ?1 ?  ?! ?- ?( ?)
          ?א ?ב ?ג ?ד ?ה
          ?ا ?ب ?ج ?د ?ه
          ?\n ?\t))
"##,
    )
}

#[test]
fn div_cx227_char_decomposition_compatibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c)
          (let ((d (get-char-code-property c 'decomposition)))
            (list c d)))
        '(?à ?é ?ü ?ñ ?Ä ?ö ?Ç ?Å ?Æ ?Œ ?œ
          ?ﬁ ?ﬂ ?ℌ ?℃ ?㈎ ?㈏))
"##,
    );
}

#[test]
fn div_cx227_char_old_name_and_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (mapcar (lambda (c)
              (list c
                    (get-char-code-property c 'old-name)
                    (get-char-code-property c 'iso-10646-comment)))
            '(?à ?é ?Ä ?Ç ?Æ ?Œ ?ℌ))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx227_char_name_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (get-char-code-property c 'name))
        '(?a ?A ?0 ?  ?! ?( ?)
          ?à ?é ?α ?β ?Α ?Β
          ?世 ?界 ?日
          ?😀 ?🎉 ?🌍))
"##,
    );
}

#[test]
fn div_cx227_char_script_full_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (char-script c)))
        '(?a ?A ?0 ?  ?! ?( ?-
          ?à ?é ?ñ ?Ç ?Æ
          ?α ?β ?Ω
          ?世 ?界 ?日 ?本 ?語 ?中 ?国 ?한 ?글
          ?א ?ב ?ا ?ب
          ?À ?É ?Ñ ?Ø ?Þ ?Ð ?Þ))
"##,
    );
}

#[test]
fn div_cx227_unicode_property_value_aliases() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'unicode-property-table-internal)
          (char-table-p (category-table))
          (boundp 'char-script-table)
          (fboundp 'char-code-property-description))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx227_char_properties_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((cats (mapcar (lambda (c) (get-char-code-property c 'general-category))
                    '(?a ?A ?0 ?! ?( ?- ?à ?α ?世))))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "Char properties mega café 世界 test")
    (put-text-property 1 5 'face 'bold)
    (let ((m (set-marker (make-marker) 10))
          (ov (make-overlay 4 18)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 22)
      (let ((state (list cats
                         (mapcar #'char-script '(?a ?α ?世 ?é))
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    );
}
