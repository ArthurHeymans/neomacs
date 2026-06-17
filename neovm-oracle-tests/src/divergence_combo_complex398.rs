//! Complex combo batch 398 — `coding-system`/`charset` registry ultimate:
//! coding-system-p/type/mnemonic/category/aliases/plist matrix across all
//! major codings, charset-dimension/chars/plist, decode/encode roundtrip.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx398_coding_system_p_matrix_all_codings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs (coding-system-p cs)))
        '(utf-8 utf-8-unix utf-8-with-signature
          latin-1 iso-8859-1 iso-8859-9
          utf-16 utf-16le utf-16be
          big5 gb2312 no-conversion
          undecided binary invalid-cs))
"##,
    )
}

#[test]
fn div_cx398_coding_system_type_mnemonic_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs
                (condition-case e (coding-system-type cs) (error :err))
                (condition-case e (coding-system-mnemonic cs) (error :err))))
        '(utf-8 utf-8-unix latin-1 iso-8859-9 utf-16 big5 gb2312))
"##,
    )
}

#[test]
fn div_cx398_coding_system_category_matrix() {
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
fn div_cx398_coding_system_aliases_and_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs
                (condition-case e (coding-system-aliases cs) (error :err))
                (condition-case e (coding-system-plist cs) (error :err))))
        '(utf-8 utf-8-unix latin-1 iso-8859-9))
"##,
    )
}

#[test]
fn div_cx398_encode_decode_roundtrip_all_major_codings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((text "Hello café 世界"))
  (mapcar (lambda (cs)
            (condition-case e
                (let* ((enc (encode-coding-string text cs))
                      (dec (decode-coding-string enc cs)))
                 (list cs (string-bytes enc) (string= text dec)))
              (error (list cs :err (car e)))))
          '(utf-8 utf-8-unix latin-1 iso-8859-1 utf-16 utf-16le utf-16be
            big5 gb2312 no-conversion)))
"##,
    )
}

#[test]
fn div_cx398_encode_utf8_with_signature_bom() {
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
fn div_cx398_decode_invalid_bytes_per_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((raw (unibyte-string #x68 #x65 #x6c #x6c #x6f #xff #xc3 #xa9)))
  (mapcar (lambda (cs)
            (condition-case e
                (let ((dec (decode-coding-string raw cs t)))
                  (list cs (length dec)
                        (mapcar #'char-charset (string-to-list dec))))
              (error (list cs :err (car e)))))
          '(utf-8 latin-1 iso-8859-1 no-conversion)))
"##,
    )
}

#[test]
fn div_cx398_charset_plist_and_dimension_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (let ((p (charset-plist cs)))
            (list cs (plist-get p :dimension)
                  (plist-get p :short-name)
                  (plist-get p :docstring)
                  (plist-get p :code-space))))
        '(ascii unicode eight-bit iso-8859-1))
"##,
    )
}

#[test]
fn div_cx398_current_bidi_paragraph_direction_all_scripts() {
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
fn div_cx398_coding_charset_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((text "café 世界 😀 coding mega")
       (enc (encode-coding-string text 'utf-8))
       (hash (secure-hash 'sha256 enc)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert enc)
    (put-text-property 1 4 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 3 12)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 14)
      (let ((state (list (string= text (decode-coding-string enc 'utf-8-unix))
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
