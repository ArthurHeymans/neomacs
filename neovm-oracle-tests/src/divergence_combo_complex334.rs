//! Complex combo batch 334 — `coding-system`/`charset` ultimate:
//! encode/decode with utf-8/latin-1/big5/utf-16, BOM check, category
//! matrix, char-charset for eight-bit, charset-plist completeness.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx334_encode_decode_roundtrip_all_major_codings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((text "Hello café 世界"))
  (mapcar (lambda (cs)
            (condition-case e
                (let* ((enc (encode-coding-string text cs))
                      (dec (decode-coding-string enc cs)))
                 (list cs (length enc) (string-bytes enc) (string= text dec)))
              (error (list cs :err (car e)))))
          '(utf-8 utf-8-unix latin-1 iso-8859-1 utf-16 utf-16le utf-16be
            big5 gb2312 no-conversion)))
"##,
    )
}

#[test]
fn div_cx334_encode_utf8_with_signature_bom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((text "café世界")
       (plain (encode-coding-string text 'utf-8))
       (sig (encode-coding-string text 'utf-8-with-signature)))
  (list (string-bytes plain) (string-bytes sig)
        (aref sig 0) (aref sig 1) (aref sig 2)
        (string= (substring sig 3) plain)))
"##,
    )
}

#[test]
fn div_cx334_decode_invalid_utf8_bytes_per_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((raw (unibyte-string #x68 #x65 #x6c #x6c #x6f #xff #xc3 #xa9)))
  (mapcar (lambda (cs)
            (condition-case e
                (let ((dec (decode-coding-string raw cs t)))
                  (list cs (length dec) (mapcar #'char-charset (string-to-list dec))))
              (error (list cs :err (car e)))))
          '(utf-8 latin-1 iso-8859-1 no-conversion)))
"##,
    )
}

#[test]
fn div_cx334_char_charset_classification_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (b)
          (let ((c (decode-char 'eight-bit b)))
            (list b (char-charset c))))
        '(128 144 160 180 200 220 240 255))
"##,
    )
}

#[test]
fn div_cx334_coding_system_category_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs (condition-case e (coding-system-category cs) (error :err))))
        '(utf-8 utf-8-with-signature latin-1 iso-8859-7
          emacs-mule utf-16 utf-16be utf-16le big5
          no-conversion raw-text undecided binary))
"##,
    )
}

#[test]
fn div_cx334_charset_plist_completeness() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (let ((p (charset-plist cs)))
            (list cs (plist-get p :dimension) (plist-get p :short-name)
                  (plist-get p :docstring) (plist-get p :code-space))))
        '(ascii unicode eight-bit iso-8859-1))
"##,
    )
}

#[test]
fn div_cx334_current_bidi_paragraph_direction_all_scripts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (with-temp-buffer (insert "Hello world") (current-bidi-paragraph-direction))
 (with-temp-buffer (insert "مرحبا بالعالم") (current-bidi-paragraph-direction))
 (with-temp-buffer (insert "שלום עולם") (current-bidi-paragraph-direction))
 (with-temp-buffer (insert "") (current-bidi-paragraph-direction))
 (with-temp-buffer (insert "你好世界") (current-bidi-paragraph-direction)))
"##,
    )
}

#[test]
fn div_cx334_set_buffer_multibyte_toggle_data_loss() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx334-tog*")))
  (with-current-buffer buf
    (set-buffer-multibyte t)
    (insert "café 世界 0123456789ABCDEF0123456789")
    (let ((len-mb (buffer-size))
          (bytes-mb (string-bytes (buffer-string))))
      (set-buffer-multibyte nil)
      (let ((len-uni (buffer-size)))
        (set-buffer-multibyte t)
        (let ((len-back (buffer-size)))
        (prog1 (list len-mb bytes-mb len-uni len-back)
          (kill-buffer buf))))))
"##,
    )
}

#[test]
fn div_cx334_string_make_unibyte_multibyte_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((mb "café 世界")
       (uni (string-make-unibyte mb))
       (back (string-make-multibyte uni)))
  (list mb uni back
        (multibyte-string-p mb)
        (multibyte-string-p uni)
        (multibyte-string-p back)
        (length mb) (length uni) (length back)
        (string-bytes mb) (string-bytes uni) (string-bytes back)))
"##,
    )
}

#[test]
fn div_cx334_coding_charset_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((text "café 世界 😀 coding mega")
       (enc (encode-coding-string text 'utf-8))
       (dec (decode-coding-string enc 'utf-8-unix))
       (hash (secure-hash 'sha256 enc)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert dec)
    (put-text-property 1 4 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 3 12)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 14)
      (let ((state (list (string= text dec)
                         (length enc) (string-bytes enc) hash
                         (coding-system-category 'utf-8)
                         (charset-plist 'ascii)
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen()
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1)))))))
"##,
    )
}
