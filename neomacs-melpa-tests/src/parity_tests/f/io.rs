use expect_test::expect;

use super::{assert_f_parity, assert_f_signal_parity};

#[test]
fn f_text_io_round_trips_multiple_codings_and_aliases() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-text-" t))
                    (utf8 (expand-file-name "utf8.txt" root))
                    (latin1 (expand-file-name "latin1.txt" root)))
               (unwind-protect
                   (progn
                     (f-write-text "åß中" 'utf-8 utf8)
                     (f-write "über" 'iso-8859-1 latin1)
                     (list
                      (f-read-text utf8 'utf-8)
                      (f-read utf8)
                      (f-read-text latin1 'iso-8859-1)
                      (multibyte-string-p (f-read-text utf8))
                      (f-size utf8)
                      (f-size latin1)))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK ("åß中" "åß中" #("über" 0 4 (charset iso-8859-1)) t 7 4)"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_byte_io_preserves_unibyte_data_and_partial_ranges() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-bytes-" t))
                    (path (expand-file-name "bytes.bin" root))
                    (bytes (unibyte-string 0 1 127 128 255 65 66 67)))
               (unwind-protect
                   (progn
                     (f-write-bytes bytes path)
                     (let ((whole (f-read-bytes path)))
                       (list
                        (f-unibyte-string-p whole)
                        (multibyte-string-p whole)
                        (string-to-list whole)
                        (string-to-list (f-read-bytes path 2 6))
                        (string-to-list (f-read-bytes path 6)))))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK (t nil (0 1 127 128 255 65 66 67) (127 128 255 65) (66 67))"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_append_text_and_bytes_create_then_extend_files() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-append-" t))
                    (text (expand-file-name "text.txt" root))
                    (bytes (expand-file-name "bytes.bin" root)))
               (unwind-protect
                   (progn
                     (f-append-text "å" 'utf-8 text)
                     (f-append "ß" 'utf-8 text)
                     (f-append-bytes (unibyte-string 0 127) bytes)
                     (f-append-bytes (unibyte-string 128 255) bytes)
                     (list
                      (f-read text)
                      (string-to-list (f-read-bytes bytes))
                      (f-size text)
                      (f-size bytes)))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK ("åß" (0 127 128 255) 4 4)"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_write_bytes_rejects_multibyte_input() {
    let elisp_form = r##"(f-write-bytes "å" "not-created.bin")"##;
    let expect = expect![[r#"ERR (wrong-type-argument f-unibyte-string-p "å")"#]];

    assert_f_signal_parity(elisp_form, expect);
}
