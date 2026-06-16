//! Complex combo batch 178 — `charset` / `coding-system` registry deep
//! dive: all coding-system predicates, charset dimension, code-space,
//! priority list ordering.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx178_coding_system_p_matrix() {
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
    );
}

#[test]
fn div_cx178_coding_system_type_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs (condition-case e (coding-system-type cs) (error :err))))
        '(utf-8 latin-1 iso-8859-9 utf-16 big5 gb2312))
"##,
    );
}

#[test]
fn div_cx178_coding_system_mnemonic_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs (condition-case e (coding-system-mnemonic cs) (error :err))))
        '(utf-8 utf-8-unix latin-1 iso-8859-9 utf-16 big5))
"##,
    );
}

#[test]
fn div_cx178_coding_system_category_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs (condition-case e (coding-system-category cs) (error :err))))
        '(utf-8 utf-8-with-signature latin-1 iso-8859-7
          emacs-mule utf-16 utf-16be utf-16le big5
          no-conversion raw-text undecided binary))
"##,
    );
}

#[test]
fn div_cx178_charset_plist_complete_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (let ((p (condition-case e (charset-plist cs) (error nil))))
            (list cs
                  (plist-get p :dimension)
                  (plist-get p :name)
                  (plist-get p :short-name)
                  (plist-get p :long-name)
                  (plist-get p :docstring)
                  (plist-get p :code-space))))
        '(ascii unicode eight-bit iso-8859-1
          latin-iso8859-1 mule-unicode-0100-24ff))
"##,
    );
}

#[test]
fn div_cx178_charset_dimension_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs (condition-case e (charset-dimension cs) (error :err))))
        '(ascii unicode eight-bit iso-8859-1 latin-iso8859-1))
"##,
    );
}

#[test]
fn div_cx178_coding_system_priority_list_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'coding-category-list)
      (consp coding-category-list)
      (fboundp 'set-coding-priority))
"##,
    );
}

#[test]
fn div_cx178_charset_chars_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (mapcar (lambda (cs)
              (list cs (condition-case e (charset-chars cs) (error :err))))
            '(ascii unicode iso-8859-1))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx178_coding_system_aliases_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs)
          (list cs (condition-case e (coding-system-aliases cs) (error :err))))
        '(utf-8 latin-1 iso-8859-9))
"##,
    );
}

#[test]
fn div_cx178_coding_system_put_get_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (coding-system-plist 'utf-8)
          (plist-get (coding-system-plist 'utf-8) :name)
          (plist-get (coding-system-plist 'utf-8) :mime-charset))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx178_charset_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((cs 'utf-8)
      (charset 'ascii))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (format "Charset/coding mega: %s/%s" charset cs))
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (let ((state (list (coding-system-p cs)
                         (coding-system-type cs)
                         (charset-dimension charset)
                         (charset-plist charset)
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
