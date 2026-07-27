use expect_test::expect;

use super::{assert_archive_rpm_parity, assert_archive_rpm_signal_parity};

#[test]
fn parses_string_and_non_string_index_entries_from_a_realistic_metadata_header() {
    let elisp_form = r##"(let* ((data
         (string-as-unibyte
          (concat
           "neomacs-tools\0"
           "2.7.1\0"
           "payload-count\0")))
       (entries
        '(((:tag . 1000) (:type . 6)
           (:offset . 0) (:count . 1))
          ((:tag . 1001) (:type . 6)
           (:offset . 14) (:count . 1))
          ((:tag . 1009) (:type . 4)
           (:offset . 20) (:count . 1))
          ((:tag . 1124) (:type . 6)
           (:offset . 20) (:count . 1))))
       (header
        (bindat-pack
         archive-rpm--header-bindat-spec
         `((:magic . ,#x8eade8)
           (:version . 1)
           (:reserved . 0)
           (:n-index-entries . ,(length entries))
           (:data-len . ,(length data))))))
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (insert header)
    (dolist (entry entries)
      (insert
       (bindat-pack
        archive-rpm--index-entry-bindat-spec
        entry)))
    (insert data)
    (goto-char (point-min))
    (let ((parsed
           (archive-rpm--parse-header nil)))
      (list
       (point)
       (mapcar
        (lambda (entry)
          (list
           (bindat-get-field entry :tag)
           (bindat-get-field entry :type)
           (bindat-get-field entry :offset)
           (bindat-get-field entry :count)
           (bindat-get-field entry :data)))
        (nreverse parsed))))))"##;
    let expect = expect![[
        r#"OK (115 ((1000 6 0 1 "neomacs-tools") (1001 6 14 1 "2.7.1") (1009 4 20 1 nil) (1124 6 20 1 "payload-count")))"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn parse_header_returns_entries_in_reverse_on_disk_order_as_upstream_specifies() {
    let elisp_form = r##"(let* ((tags '(1000 1001 1002 1124 1125))
       (data "a\0b\0c\0cpio\0gzip\0")
       (offsets '(0 2 4 6 11))
       (header
        (bindat-pack
         archive-rpm--header-bindat-spec
         `((:magic . ,#x8eade8)
           (:version . 1)
           (:reserved . 0)
           (:n-index-entries . 5)
           (:data-len . ,(length data))))))
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (insert header)
    (cl-mapc
     (lambda (tag offset)
       (insert
        (bindat-pack
         archive-rpm--index-entry-bindat-spec
         `((:tag . ,tag) (:type . 6)
           (:offset . ,offset) (:count . 1)))))
     tags offsets)
    (insert data)
    (goto-char (point-min))
    (mapcar
     (lambda (entry)
       (cons
        (bindat-get-field entry :tag)
        (bindat-get-field entry :data)))
     (archive-rpm--parse-header nil))))"##;
    let expect =
        expect![[r#"OK ((1125 . "gzip") (1124 . "cpio") (1002 . "c") (1001 . "b") (1000 . "a"))"#]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn signature_header_alignment_skips_only_the_required_zero_to_seven_bytes() {
    let elisp_form = r##"(mapcar
 (lambda (data-len)
   (let ((header
          (bindat-pack
           archive-rpm--header-bindat-spec
           `((:magic . ,#x8eade8)
             (:version . 1)
             (:reserved . 0)
             (:n-index-entries . 0)
             (:data-len . ,data-len)))))
     (with-temp-buffer
       (set-buffer-multibyte nil)
       (insert header
               (make-string data-len ?d)
               (make-string 8 ?p))
       (goto-char (point-min))
       (archive-rpm--parse-header t)
       (list data-len
             (point)
             (- (point) 17 data-len)
             (char-after)))))
 '(0 1 2 3 4 5 6 7 8 9 15 16))"##;
    let expect = expect![
        "OK ((0 17 0 112) (1 25 7 112) (2 25 6 112) (3 25 5 112) (4 25 4 112) (5 25 3 112) (6 25 2 112) (7 25 1 112) (8 25 0 112) (9 33 7 112) (15 33 1 112) (16 33 0 112))"
    ];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn ordinary_header_parse_does_not_apply_signature_alignment() {
    let elisp_form = r##"(mapcar
 (lambda (data-len)
   (with-temp-buffer
     (set-buffer-multibyte nil)
     (insert
      (bindat-pack
       archive-rpm--header-bindat-spec
       `((:magic . ,#x8eade8)
         (:version . 1)
         (:reserved . 0)
         (:n-index-entries . 0)
         (:data-len . ,data-len)))
      (make-string data-len ?d)
      "PAYLOAD")
     (goto-char (point-min))
     (archive-rpm--parse-header nil)
     (list data-len
           (point)
           (buffer-substring
            (point) (point-max)))))
 '(0 1 3 7 8 9))"##;
    let expect = expect![[
        r#"OK ((0 17 "PAYLOAD") (1 18 "PAYLOAD") (3 20 "PAYLOAD") (7 24 "PAYLOAD") (8 25 "PAYLOAD") (9 26 "PAYLOAD"))"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn finds_null_terminated_string_data_within_the_declared_store_boundary() {
    let elisp_form = r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert "PREFIXalpha\0beta\0OUTSIDE")
  (let* ((entry
          '((:tag . 1000) (:type . 6)
            (:offset . 0) (:count . 1)))
         (found
          (archive-rpm--find-index-entry-data
           entry 7 11)))
    (list
     (bindat-get-field found :tag)
     (bindat-get-field found :data)
     (equal entry found)
     (eq entry found))))"##;
    let expect = expect![[r#"OK (1000 "alpha" nil nil)"#]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn string_entry_without_an_in_bounds_null_terminator_returns_nil() {
    let elisp_form = r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert "PREFIXunterminated\0")
  (archive-rpm--find-index-entry-data
   '((:tag . 1000) (:type . 6)
     (:offset . 0) (:count . 1))
   7 12))"##;
    let expect = expect!["OK nil"];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn non_string_index_types_are_returned_unchanged_without_reading_data() {
    let elisp_form = r##"(mapcar
 (lambda (type)
   (let* ((entry
           `((:tag . 1009)
             (:type . ,type)
             (:offset . 999999)
             (:count . 3)))
          (result
           (archive-rpm--find-index-entry-data
            entry 1 0)))
     (list type
           (eq entry result)
           (equal entry result)
           (bindat-get-field result :data))))
 '(0 1 2 3 4 5 7 8 9))"##;
    let expect = expect![
        "OK ((0 t t nil) (1 t t nil) (2 t t nil) (3 t t nil) (4 t t nil) (5 t t nil) (7 t t nil) (8 t t nil) (9 t t nil))"
    ];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn metadata_lookup_returns_strings_numeric_absence_and_duplicate_first_match() {
    let elisp_form = r##"(let ((entries
       '(((:tag . 1000) (:type . 6)
          (:offset . 0) (:count . 1)
          (:data . "first-name"))
         ((:tag . 1009) (:type . 4)
          (:offset . 20) (:count . 1))
         ((:tag . 1000) (:type . 6)
          (:offset . 30) (:count . 1)
          (:data . "second-name")))))
  (mapcar
   (lambda (tag)
     (list tag
           (archive-rpm--get-header-data
            tag entries)))
   '(1000 1009 9999)))"##;
    let expect = expect![[r#"OK ((1000 "first-name") (1009 nil) (9999 nil))"#]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn interesting_metadata_rendering_is_ordered_aligned_and_omits_non_strings() {
    let elisp_form = r##"(let ((entries
       '(((:tag . 1125) (:type . 6)
          (:data . "zstd"))
         ((:tag . 1000) (:type . 6)
          (:data . "neomacs"))
         ((:tag . 1001) (:type . 6)
          (:data . "2.0"))
         ((:tag . 1002) (:type . 6)
          (:data . "7.fc42"))
         ((:tag . 1004) (:type . 6)
          (:data . "A programmable editor"))
         ((:tag . 1011) (:type . 6)
          (:data . "Example Engineering"))
         ((:tag . 1014) (:type . 6)
          (:data . "GPL-3.0-or-later"))
         ((:tag . 1022) (:type . 6)
          (:data . "x86_64"))
         ((:tag . 1124) (:type . 6)
          (:data . "cpio"))
         ((:tag . 1020) (:type . 4)))))
  (with-temp-buffer
    (archive-rpm--insert-interesting-information
     entries)
    (list
     (buffer-string)
     (count-lines (point-min) (point-max))
     (current-column))))"##;
    let expect = expect![[
        r#"OK ("Name:         neomacs\nVersion:      2.0\nRelease:      7.fc42\nSummary:      A programmable editor\nVendor:       Example Engineering\nLicense:      GPL-3.0-or-later\nArchitecture: x86_64\nFormat:       cpio\nCompression:  zstd\n\n" 10 0)"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn parse_header_rejects_wrong_magic_before_unpacking_any_indexes() {
    let elisp_form = r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert
   (apply #'unibyte-string
          '(142 173 233 1))
   (make-string 12 0))
  (goto-char (point-min))
  (archive-rpm--parse-header nil))"##;
    let expect = expect![[r#"ERR (error "Incorrect header magic")"#]];
    assert_archive_rpm_signal_parity(elisp_form, expect);
}

#[test]
fn parse_header_rejects_a_declared_index_missing_from_truncated_input() {
    let elisp_form = r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert
   (bindat-pack
    archive-rpm--header-bindat-spec
    `((:magic . ,#x8eade8)
      (:version . 1)
      (:reserved . 0)
      (:n-index-entries . 1)
      (:data-len . 0))))
  (goto-char (point-min))
  (archive-rpm--parse-header nil))"##;
    let expect = expect!["ERR (args-out-of-range (:buffer nil) 17 33)"];
    assert_archive_rpm_signal_parity(elisp_form, expect);
}
