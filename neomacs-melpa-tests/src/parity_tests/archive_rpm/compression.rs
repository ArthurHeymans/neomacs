use expect_test::expect;

use super::{assert_archive_rpm_parity, assert_archive_rpm_signal_parity};

#[test]
fn gzip_payload_round_trip_preserves_large_text_binary_and_nul_bytes() {
    let elisp_form = r##"(let* ((plain
         (concat
          (make-string 2048 ?A)
          "\nconfiguration=true\n"
          (unibyte-string 0 1 2 127 128 254 255)
          (make-string 1024 ?Z)))
       (compressed
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert plain)
          (let ((status
                 (call-process-region
                  (point-min) (point-max)
                  "gzip" t t nil "-n" "-c")))
            (unless (zerop status)
              (error "fixture gzip failed: %S" status)))
          (buffer-string)))
       (entries
        '(((:tag . 1124) (:data . "cpio"))
          ((:tag . 1125) (:data . "gzip")))))
  (with-temp-buffer
    (archive-rpm--decompress-payload
     compressed entries)
    (list
     (length plain)
     (length compressed)
     (length (buffer-string))
     (equal plain (buffer-string))
     (secure-hash 'sha256 plain)
     (secure-hash 'sha256 (current-buffer))
     enable-multibyte-characters)))"##;
    let expect = expect![[
        r#"OK (3099 69 3099 t "f84bf59e2bedeebab7ab547568ff40d29b066ed53091638ccb6e7db7d6ebc09d" "f84bf59e2bedeebab7ab547568ff40d29b066ed53091638ccb6e7db7d6ebc09d" nil)"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn xz_payload_round_trip_preserves_a_practical_cpio_like_binary_stream() {
    let elisp_form = r##"(let* ((plain
         (string-as-unibyte
          (concat
           "070701"
           (make-string 104 ?0)
           "nested/path.txt\0"
           (make-string 4096 ?p)
           (unibyte-string 0 128 255))))
       (compressed
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert plain)
          (let ((status
                 (call-process-region
                  (point-min) (point-max)
                  "xz" t t nil
                  "-q" "-c" "--threads=1")))
            (unless (zerop status)
              (error "fixture xz failed: %S" status)))
          (buffer-string)))
       (entries
        '(((:tag . 1124) (:data . "cpio"))
          ((:tag . 1125) (:data . "xz")))))
  (with-temp-buffer
    (archive-rpm--decompress-payload
     compressed entries)
    (list
     (equal plain (buffer-string))
     (length plain)
     (length compressed)
     (secure-hash 'sha256 (current-buffer))
     enable-multibyte-characters)))"##;
    let expect = expect![[
        r#"OK (t 4225 120 "7d8f1dd244ded96ba7dad868cd0aa5b3c96c016ef2a07dd019a03df83f56c07a" nil)"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn legacy_lzma_payload_uses_xz_compatibility_and_round_trips_exactly() {
    let elisp_form = r##"(let* ((plain
         (string-as-unibyte
          (concat
           "legacy rpm payload\n"
           (make-string 3072 ?L)
           (unibyte-string 0 42 200 255))))
       (compressed
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert plain)
          (let ((status
                 (call-process-region
                  (point-min) (point-max)
                  "xz" t t nil
                  "-q" "-c" "--format=lzma")))
            (unless (zerop status)
              (error "fixture lzma failed: %S" status)))
          (buffer-string)))
       (entries
        '(((:tag . 1124) (:data . "cpio"))
          ((:tag . 1125) (:data . "lzma")))))
  (with-temp-buffer
    (archive-rpm--decompress-payload
     compressed entries)
    (list
     (equal plain (buffer-string))
     (length plain)
     (length compressed)
     (secure-hash 'sha256 (current-buffer))
     enable-multibyte-characters)))"##;
    let expect = expect![[
        r#"OK (t 3095 66 "a4632c432642be38cf55f7d1627de6c8b43a872d3aa11a952936bfd43ddd9a23" nil)"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn zstd_payload_round_trip_preserves_repetitive_and_high_byte_content() {
    let elisp_form = r##"(let* ((plain
         (string-as-unibyte
          (concat
           (make-string 8192 ?R)
           (unibyte-string 0 1 64 127 128 191 254 255)
           "\nend\n")))
       (compressed
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert plain)
          (let ((status
                 (call-process-region
                  (point-min) (point-max)
                  "zstd" t t nil "-q" "-c")))
            (unless (zerop status)
              (error "fixture zstd failed: %S" status)))
          (buffer-string)))
       (entries
        '(((:tag . 1124) (:data . "cpio"))
          ((:tag . 1125) (:data . "zstd")))))
  (with-temp-buffer
    (archive-rpm--decompress-payload
     compressed entries)
    (list
     (equal plain (buffer-string))
     (length plain)
     (length compressed)
     (secure-hash 'sha256 (current-buffer))
     enable-multibyte-characters)))"##;
    let expect = expect![[
        r#"OK (t 8205 35 "17a35bc433e780d41622aed61241f623b3c34d1d008ed5845e1bb63bbee3b590" nil)"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn payload_format_must_be_cpio_before_compressor_dispatch() {
    let elisp_form = r##"(archive-rpm--decompress-payload
 "irrelevant"
 '(((:tag . 1124) (:data . "tar"))
   ((:tag . 1125) (:data . "gzip"))))"##;
    let expect = expect![[r#"ERR (error "RPM payload is in ‘tar’, not cpio format")"#]];
    assert_archive_rpm_signal_parity(elisp_form, expect);
}

#[test]
fn absent_payload_format_signals_the_same_explicit_format_error() {
    let elisp_form = r##"(archive-rpm--decompress-payload
 ""
 '(((:tag . 1125) (:data . "gzip"))))"##;
    let expect = expect![[r#"ERR (error "RPM payload is in ‘nil’, not cpio format")"#]];
    assert_archive_rpm_signal_parity(elisp_form, expect);
}

#[test]
fn unknown_payload_compressor_is_rejected_with_its_exact_name() {
    let elisp_form = r##"(archive-rpm--decompress-payload
 "payload"
 '(((:tag . 1124) (:data . "cpio"))
   ((:tag . 1125) (:data . "brotli"))))"##;
    let expect = expect![[r#"ERR (error "Unknown RPM payload compressor ‘brotli’")"#]];
    assert_archive_rpm_signal_parity(elisp_form, expect);
}

#[test]
fn corrupted_gzip_payload_signals_decompression_failure_without_partial_success() {
    let elisp_form = r##"(archive-rpm--decompress-payload
 (string-as-unibyte
  (concat "\x1f\x8b\x08\0"
          (make-string 20 ?x)))
 '(((:tag . 1124) (:data . "cpio"))
   ((:tag . 1125) (:data . "gzip"))))"##;
    let expect = expect![[r#"ERR (error "Zlib decompression failed")"#]];
    assert_archive_rpm_signal_parity(elisp_form, expect);
}

#[test]
fn corrupted_xz_payload_surfaces_the_subprocess_diagnostic_as_a_signal() {
    let elisp_form = r##"(archive-rpm--decompress-payload
 (string-as-unibyte
  (concat "\xfd7zXZ\0"
          (make-string 20 ?x)))
 '(((:tag . 1124) (:data . "cpio"))
   ((:tag . 1125) (:data . "xz"))))"##;
    let expect = expect![[
        r#"ERR (error "xz decompression failed: /nix/store/2nm5c858fh52s6mhcffm07s3biaxys44-xz-5.8.3-bin/bin/xz: (stdin): File format not recognized\n")"#
    ]];
    assert_archive_rpm_signal_parity(elisp_form, expect);
}
