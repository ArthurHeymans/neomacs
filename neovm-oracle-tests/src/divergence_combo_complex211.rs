//! Complex combo batch 211 — `emoji` / variation selectors / ZWJ /
//! `emoji-glyph-map` / `char-has-emoji-presentation-p` queries.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx211_emoji_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (featurep 'emoji)
          (fboundp 'emoji-insert)
          (fboundp 'emoji-search)
          (fboundp 'emoji-list)
          (boundp 'emoji--repeat-walking))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx211_emoji_presentation_selector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'char-has-emoji-presentation-p)
          (boundp 'emoji-variation-specifications-zwj)
          (boundp 'emoji-variation-specifications-vs))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx211_emoji_in_buffer_with_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "Before 😀 after 🎉 end 🌍")
  (list (buffer-string)
        (length (buffer-string))
        (string-bytes (buffer-string))
        (mapcar #'char-charset (string-to-list (buffer-string)))))
"##,
    );
}

#[test]
fn div_cx211_emoji_width_and_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((emoji-str "😀🎉🌍"))
  (list (string-width emoji-str)
        (length emoji-str)
        (string-bytes emoji-str)
        (mapcar #'char-width (string-to-list emoji-str))))
"##,
    );
}

#[test]
fn div_cx211_variation_selector_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (char-charset c) (char-width c)))
        '(#xFE0E #xFE0F #x200D #x20E3))
"##,
    );
}

#[test]
fn div_cx211_zwj_sequence_handling() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((zwj-seq (concat (string ?👨 ?\x200d ?👩 ?\x200d ?👦))))
  (list (length zwj-seq)
        (string-bytes zwj-seq)
        (string-width zwj-seq)
        (mapcar #'char-charset (string-to-list zwj-seq))))
"##,
    );
}

#[test]
fn div_cx211_emoji_keycap_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((keycap (concat (string ?1 #xFE0F #x20E3))))
  (list (length keycap)
        (string-bytes keycap)
        (string-width keycap)))
"##,
    );
}

#[test]
fn div_cx211_emoji_regexp_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "text 😀 more 🎉 text 🌍 end"))
  (list (string-match "😀" s)
        (match-beginning 0) (match-end 0)
        (string-match "🎉" s)
        (match-beginning 0) (match-end 0)))
"##,
    );
}

#[test]
fn div_cx211_emoji_format_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((emoji-text "Hello 😀 World 🎉"))
  (let ((printed (prin1-to-string emoji-text))
        (read-back (car (read-from-string (prin1-to-string emoji-text)))))
    (list (length emoji-text)
          (string-bytes emoji-text)
          printed
          (equal emoji-text read-back))))
"##,
    );
}

#[test]
fn div_cx211_emoji_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((emoji-str "😀🎉🌍 alpha"))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert emoji-str)
    (put-text-property 1 3 'face 'bold)
    (let ((m (set-marker (make-marker) 4))
          (ov (make-overlay 1 5)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 1 7)
      (let ((state (list (string-width (buffer-string))
                         (length (buffer-string))
                         (string-bytes (buffer-string))
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
