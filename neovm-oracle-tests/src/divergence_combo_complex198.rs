//! Complex combo batch 198 — `coding-system` encode/decode matrix across
//! ALL supported and unsupported codings, with multibyte payload.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx198_encode_decode_roundtrip_utf8_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((text "Hello café 世界 😀 end")
       (enc (encode-coding-string text 'utf-8))
       (dec (decode-coding-string enc 'utf-8-unix)))
  (list (string= text dec)
        (equal text dec)
        (length text)
        (string-bytes enc)
        (length enc)))
"##,
    );
}

#[test]
fn div_cx198_encode_with_signature_bom_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((text "café世界")
       (plain (encode-coding-string text 'utf-8))
       (sig (encode-coding-string text 'utf-8-with-signature)))
  (list (length plain)
        (length sig)
        (aref sig 0) (aref sig 1) (aref sig 2)
        (string= (substring sig 3) plain)))
"##,
    );
}

#[test]
fn div_cx198_decode_invalid_bytes_per_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((raw (unibyte-string #x68 #x65 #x6c #x6c #x6f #xff #xc3 #xa9)))
  (mapcar (lambda (cs)
            (condition-case e
                (let ((dec (decode-coding-string raw cs t)))
                  (list cs (length dec) (string-bytes dec)
                        (mapcar #'char-charset (string-to-list dec))))
              (error (list cs :err (car e)))))
          '(utf-8 latin-1 iso-8859-1 no-conversion)))
"##,
    );
}

#[test]
fn div_cx198_encode_coding_region_in_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string #x68 #x65 #x6c #x6c #x6f #xff #xfe))
  (set-buffer-multibyte t)
  (encode-coding-region (point-min) (point-max) 'utf-8-unix (current-buffer))
  (list (buffer-string) (buffer-size)))
"##,
    );
}

#[test]
fn div_cx198_decode_coding_region_in_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string #xe4 #xb8 #x96 #xe7 #x95 #x8c #x00 #x41))
  (set-buffer-multibyte t)
  (decode-coding-region (point-min) (point-max) 'utf-8-unix (current-buffer) t)
  (list (buffer-string) (buffer-size) (string-bytes (buffer-string))))
"##,
    );
}

#[test]
fn div_cx198_coding_system_plist_query_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (let ((p (coding-system-plist cs)))
            (list cs
                  (plist-get p :name)
                  (plist-get p :mnemonic)
                  (plist-get p :mime-charset)
                  (plist-get p :ascii-compatible-p))))
        '(utf-8 utf-8-with-signature latin-1 iso-8859-9
          utf-16 utf-16le utf-16be big5 gb2312 no-conversion))
"##,
    );
}

#[test]
fn div_cx198_set_buffer_file_coding_system_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-file-coding-system 'utf-8-unix)
  (list (buffer-local-value 'buffer-file-coding-system (current-buffer))))
"##,
    );
}

#[test]
fn div_cx198_string_make_unibyte_then_multibyte_data() {
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
        (string-bytes mb) (string-bytes uni) (string-bytes back)
        (equal mb back)))
"##,
    );
}

#[test]
fn div_cx198_coding_system_aliases_and_parents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (condition-case e
              (list cs (coding-system-aliases cs)
                    (coding-system-parent cs))
            (error (list cs :err (car e)))))
        '(utf-8 utf-8-unix latin-1 iso-8859-1))
"##,
    );
}

#[test]
fn div_cx198_coding_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((text "café 世界 😀 hello")
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
                         (length enc) (string-bytes enc)
                         hash
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
